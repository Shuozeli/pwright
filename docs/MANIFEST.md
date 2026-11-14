<!-- agent-updated: 2026-10-21T03:43:07Z -->

The standalone pinned Core v22 reconciliation audit at
`scripts/audit_playwright_core_v22_reconciliation.py` and its eight focused
tests cross-check all 48,477 outcome rows per lane against reviewed invocation
classes. Its immutable-input-bound
[report](../.work/official-conformance/audits/core-v22-reconciliation-v1.json)
finds 2,610 source-only `expected` rows per lane, including 19 reviewed-public
IDs that are still applicable but runtime-unproven. The audit changes no
ledger outcome and grants no runtime or parity credit. This triggers
`docs/upstream-playwright-conformance.md` and `docs/design-tasks-v2.md`.

The new default-off `light-selectors` Rust/Node profile compiles only
`id:light` and `data-testid:light` into non-piercing browser queries. Its
separate `official-conformance-page-check-light-v1` addon leaves the old
page-check profile and v1 wire ABI untouched. The unchanged full 21-case
`selectors-misc.spec.ts` candidate rerun is source-bound and passes 11/21,
versus the frozen 10/21 candidate and unchanged 21/21 Oracle; only the
upstream open-shadow-root case flips to passing. Ten REDs remain. The later
live Rust opt-in source adds the other two upstream attribute aliases with
release selector tests 92/92 enabled and 87/87 default-off, but the frozen
v20 addon does not contain them and no official browser rerun has tested them.
Locator/ElementHandle/evaluation operations and high-cardinality browser
materialization still need implementation or resource proof. This
triggers `docs/design-tasks-v2.md`,
`docs/upstream-playwright-conformance.md`, and `packages/pwright/README.md`.
The independently replayed immutable
[v20 candidate-only receipt](../.work/official-conformance/outcomes/core-selectors-misc21-light-strict-paired-v20/promotion-receipt.json)
(SHA-256 `2889851a3db6106bb726018ade7b8f2f96e72852496d40123ea825a80f3e5bd9`)
changes one candidate row, preserves 48,476 other candidate rows and all
48,477 Oracle rows byte-for-byte, and passes 39 verifier/mutation tests.
At v20, Core totals were candidate 4,660 expected / 88 unexpected / 21
skipped / 43,708 not-run; Oracle 4,439 / 11 / 20 / 44,007.

The complete pinned six-case Core Chromium `frame-frame-element.spec.ts` file
ran unchanged in source-bound dedicated-CDP lanes: candidate 0/6, Oracle 6/6,
both provenance sidecars verified, with no skips/retries/flakes/top-level
errors. A 3/3 source/ID/asset checker pins the manifest. All candidate cases
first fail on the missing JS/TS `Frame.frameElement()` method; exact DOM-owner
semantics are still untested in candidate. The independently verified immutable
[v19 paired receipt](../.work/official-conformance/outcomes/core-frame-frame-element6-strict-paired-v19/promotion-receipt.json)
(SHA-256 `9303f3e34a31f15b92f9f834b734392c88b8589c5628a3a118ba379d3d92ab4b`)
changes exactly six target rows per lane and preserves 48,471 other raw rows
per lane byte-for-byte. At v19, Core totals were candidate 4,659 expected / 89
unexpected / 21 skipped / 43,708 not-run; Oracle 4,439 / 11 / 20 / 44,007.
This triggers `docs/design-tasks-v2.md`,
`docs/upstream-playwright-conformance.md`, and `packages/pwright/README.md`.
The new default-off `frame-element` Rust/Node implementation passes release
backend 5/5, composition 4/4, Node adapter/contract 190/190, native-load,
and TS gates. Its source-consistent addon SHA-256 is
`440ec056a5cfceeacbf851c698ee24c33c3db063ad92262c215d996a568db6a0`.
Four unchanged complete six-case candidate runs on the same artifact yielded
5/6 first (`page.goto` closed before frameset `frameElement()`), then 6/6
three times. All four provenance sidecars verify. The independently replayed
immutable [v21 candidate-only receipt](../.work/official-conformance/outcomes/core-frame-frame-element6-rust-owner-strict-paired-v21/promotion-receipt.json)
(SHA-256 `3f717d16e9cdca75e601ecdcea8185c82df9c42a6c4beebb94cb67ea76f5324d`)
credits the latest six green observations while preserving the initial failure,
its pre-API location, and the unestablished root cause; the frameset case is
explicitly unstable, not parity evidence. The receipt changes only six
candidate rows, preserves 48,471 other candidate rows and all 48,477 Oracle
rows byte-for-byte, and passes 47 verifier/mutation tests. At v21, Core
totals are candidate 4,666 expected / 82 unexpected / 21 skipped / 43,708
not-run; Oracle 4,439 / 11 / 20 / 44,007. This triggers
`docs/design-tasks-v2.md`, `docs/upstream-playwright-conformance.md`, and
`packages/pwright/README.md`.

The complete pinned 13-case `frame-hierarchy.spec.ts` file has a
[source/ID/asset-pinned manifest](../tests/official/playwright-core-frame-hierarchy-public-chromium-v1-manifest.json)
and four passing source-only checker tests. Its source-bound complete-file
[candidate](../.work/official-conformance/results/core-frame-hierarchy13-rust-owner-candidate-20261021-v1.json)
is 8/13 and [Oracle](../.work/official-conformance/results/core-frame-hierarchy13-oracle-20261021-v1.json)
is 13/13; both provenance sidecars independently verify. Four REDs are
confirmed missing opt-in frame-event exposure, while one fails at `page.goto`
before its event assertions; network-frame/topology teardown causality is
unproven. This triggers
`docs/design-tasks-v2.md`, `docs/upstream-playwright-conformance.md`, and
`packages/pwright/README.md`.
The independently replayed immutable
[v22 paired receipt](../.work/official-conformance/outcomes/core-frame-hierarchy13-rust-owner-strict-paired-v22/promotion-receipt.json)
(SHA-256 `535148fa2b02b5af9e5519604cebef296f6e7daa1ff3c43dcba79139f933270a`)
replaces exactly 13 rows per lane, preserves 48,464 other raw rows per lane,
inherits v21 instability, and passes 48 verifier/mutation tests. Current v22
Core totals are candidate 4,674 expected / 87 unexpected / 21 skipped /
43,695 not-run; Oracle 4,452 / 11 / 20 / 43,994. This is not parity.
The new default-off frame-events bridge and independent frame-detach-events
feature pass Node 225/225, Rust host release 598 passed / eight ignored, and
native load 1/1. The corrected combined profile explicitly includes the
separate frame-element leaf; its frozen addon SHA-256 is
`1681505babfcf49f0e62233537f471af911fd73c247cef284ced648543c8c332`.
The first combined snapshot omitted frame-element: full hierarchy 13/13 but
frameElement 0/6 at the capability gate. On the corrected frozen snapshot,
complete hierarchy runs are 10/13 then 12/13, and complete frameElement runs
are 5/6 then 6/6. All six new raw candidate provenance sidecars verify.
Pre-assertion `TargetClosed`/network-frame-topology teardown errors remain
under investigation. These raw attempts do not supersede v22 or establish
stable parity. This triggers `docs/design-tasks-v2.md`,
`docs/upstream-playwright-conformance.md`, and `packages/pwright/README.md`.
Four focused release characterization tests confirm both independent-cursor
orderings and retention of the Request's creation Frame descriptor after
detach, including success and failure terminals. A narrow Node facade fix
retains an already-known canonical `Request.frame()` wrapper after detach;
its regression and focused tests pass 87/87 with no TODO. The pre-topology
network ordering and browser `TargetClosed` causes are still untraced, so no
strict-ledger promotion or stable parity is claimed. These changes
trigger the same three documents.

The new complete [11-case Core Context page-event manifest](../tests/official/playwright-core-browsercontext-page-event-public-chromium-v1-manifest.json)
pins source, IDs, fixture code, and six assets; its source-only checker passes
4/4. Two dedicated-CDP Oracle full-file runs are each 6/11; the same frozen
page-check candidate runs 10/11 then 9/11. All four raw provenance sidecars
independently verify. The opener timeout occurs in both lanes, and Ctrl-click
adds a candidate failure on the second run. The Oracle adapter is not an
all-green reference, candidate stability is not proved, and v22 is unchanged.
The CDP endpoint became unresponsive after the last candidate run; Chrome
was left running rather than restarted. This triggers
`docs/design-tasks-v2.md`, `docs/upstream-playwright-conformance.md`, and
`packages/pwright/README.md`.

The complete pinned Python sync `tests/sync/test_network.py` source has a
[six-function/18-expanded-ID manifest](../packages/pwright-python/tests/official/sync-network-full-v1-manifest.json)
and a 3/3 source/fixture/ledger checker. Six Chromium cases now have verified
source-bound browser results: candidate v2 1/6 and Oracle 3/6. Candidate v1
six setup errors caused by the CDP Page-lane cap remain raw infrastructure
evidence only; three provenance sidecars verify. Oracle is RED for the
loopback-only `server_addr` assertion under required Tailscale binding and
two service-worker cases. Two security-details REDs are candidate-only.
The immutable [Python v10 receipt](../.work/official-conformance/outcomes/python-sync-network6-chromium-strict-paired-v10/promotion-receipt.json)
(SHA-256 `c696c1cab0976ac45564c54814ebf12199268a904f243672203c1ababd9f7f13`)
updates six rows per lane, preserves 6,520 other rows per lane, and passes
44 controls. Cumulative Python totals are candidate 1,651 passed / 124
failed / three errors / 37 skipped / 4,711 not-run; Oracle 1,436 / 13 / 37 /
5,040. Its candidate/harness archive retains external hash-verified Oracle
and upstream dependencies; twelve non-Chromium IDs remain unrun. This triggers
`docs/design-tasks-v2.md`, `docs/upstream-playwright-conformance.md`, and
`docs/python-official-conformance.md`.
A separate pinned two-case Oracle-only secure-fixture diagnostic reruns the
same Service Worker IDs with trusted Tailscale HTTPS and passes 2/2, with
independently verified source-bound provenance. It neither rewrites the
original six-case HTTP results nor attributes all failures to TLS; see the
[manifest](../packages/pwright-python/tests/official/sync-network-service-worker-secure-chromium-oracle-v1-manifest.json)
and [report](../.work/official-conformance/results/python-sync-network-sw2-secure-chromium-oracle-20261021-v1.xml).
A separate source-pinned
[paired secure manifest](../packages/pwright-python/tests/official/sync-network-service-worker-secure-chromium-paired-v1-manifest.json)
reports candidate 1/2 versus Oracle 2/2, with both provenance sidecars
independently verified. The candidate passes response-from-Service-Worker but
does not install BrowserContext `serviceworker` event. This is diagnostic-only,
not a promotion or revision of the HTTP v10 receipt.

The isolated all-capability Python release wheel with the default-off
`navigation-response-finished` source slice has SHA-256
`a71e7a58c3bdc37658343839b3bb0fe21222b444b271553c7c92d2dda27a1f22`.
Two complete source-bound reruns of the same six Chromium network IDs each
remain 1/6; both raw reports and provenance sidecars independently verify.
The HTTPS security-details test advances past `Response.finished()` and then
fails on the absent `Response.security_details()` method. The interim join
requires explicit `request-history`; the independently bounded owner remains
an open design task. This source and diagnostic checkpoint triggers
`docs/design-tasks-v2.md`, `docs/upstream-playwright-conformance.md`,
`docs/python-official-conformance.md`, and `packages/pwright-python/README.md`;
it does not supersede the Python v10 ledger.
The focused [sync/async response-security manifest](../packages/pwright-python/tests/official/response-security-chromium-sync-async-v1-manifest.json)
pins four Chromium IDs and all twelve browser-expanded ledger IDs from two
unchanged upstream source files, fixture code, and certificate assets. Its
source/inventory checker passes 3/3; browser execution and full-file promotion
remain pending. Its inherited per-case capability labels are coarse source
classifications, not an exhaustive candidate closure: all four require normal
`Page.goto()` navigation Response plus `response-security-details`; the HTTPS
pair additionally require `ignore-https-errors` and `Response.finished()` on
that navigation Response (`navigation-response-finished` selecting
`response-finished` in the current candidate). This triggers
`docs/python-official-conformance.md`,
`docs/upstream-playwright-conformance.md`, and `docs/design-tasks-v2.md`.

The independent default-off response-security-details source slice retains
five bounded TLS fields through CDP, host, and Python without changing the
v1 wire or Node API. Its rebuilt installed all-capability wheel SHA-256 is
`0c09817458bc9329c3bbc7c00153c635ae913d3a58567746a2666ed24fa2d2cc`;
300/300 installed browserless Python contracts pass. Adapter 30/30, host
2/2, facade 5/5, the full addon dependency/release-feature matrix, and
the security-only closure/release compile proof pass. The full matrix exposed
and now passes a page-events-only compile repair; its stale server-initialize
closure expectation is corrected to four packages. Official browser execution
and ledger promotion remain pending CDP recovery. This triggers the same three documents
plus `packages/pwright-python/README.md` and `docs/resource-boundaries-v2.md`.

The rebuilt wheel's separate candidate/Oracle Python `core` profile
collect-only reports each identify 6,501 IDs with verified source-bound
provenance. The candidate used the installed wheel and test shim; official
Playwright distribution metadata was installed only to satisfy the pinned
collection-profile check, while the Oracle imported the official package.
Collect-only JUnit files contain zero executed tests, and the 25-example
Python 3.10 profile was not rerun. This does not change the strict v10 ledger.
This triggers `docs/python-official-conformance.md` and
`docs/design-tasks-v2.md`.

The default-off `pwright-network-model` `network-raw-headers` value leaf now
retains CDP extra-info repeated values separately from provisional headers.
Its explicit reader policy distinguishes Python's duplicate-value de-duplication
from Node's duplicate preservation, with input/expanded-entry/retained-string/
FFI-copy bounds. Release model tests pass 21/21 (nine new raw-header tests),
default-off tests pass 12/12, all-feature tests pass 71/71, and release Clippy
passes with warnings denied.
Event-hop pairing and public
host/Python/Node readers are still missing, so this source-only step changes
no official network50 outcomes. This triggers `docs/design-tasks-v2.md` and
`docs/upstream-playwright-conformance.md`.
The pinned network50 audit further identifies an empty request-ExtraInfo
publication batch, absent response-ExtraInfo adapter arm, and a lossy Python
addon header map. These are open integration gates, not passing tests.

The complete pinned Core Chromium `selectors-misc.spec.ts` file ran all 21
unchanged cases in each source-bound dedicated-CDP lane: candidate 10/21,
Oracle 21/21, with independently verified provenance and no skips/flakes or
top-level report errors. Its two-test manifest checker passes. Eleven
candidate-only REDs remain. The independently verified immutable
[v18 paired receipt](../.work/official-conformance/outcomes/core-selectors-misc21-strict-paired-v18/promotion-receipt.json)
(SHA-256 `0bcdeef5540013619a32bc6e5eabf9f456b2b7195dd5f05d2b1bb3e17054c261`)
changes exactly 21 target rows per lane and preserves the other 48,456 raw
rows per lane byte-for-byte. At v18, Core totals were candidate 4,659 expected /
83 unexpected / 21 skipped / 43,714 not-run; Oracle 4,433 / 11 / 20 /
44,013. This triggers
`docs/upstream-playwright-conformance.md`, `docs/design-tasks-v2.md`, and
`packages/pwright/README.md`.

Core dialog trace-only diagnostic: the unchanged source-line-67 official case
still timed out with a synchronous non-Promise runtime. An initial primitive
`Runtime.callFunctionOn` reply had no remote `objectId`; after alert opened,
`Runtime.releaseObjectGroup` was sent and received no reply before timeout.
The narrow no-handle cleanup fix now passes 21 Rust and 15 Node tests; a clean
release-addon source-bound candidate rerun passes the unchanged official file
10/10, while historical v15 candidate 9/10 and Oracle 10/10 remain sealed.
The independently verified immutable
[v16 candidate-only successor](../.work/official-conformance/outcomes/core-page-dialog10-strict-paired-v16/promotion-receipt.json)
changes only that candidate RED row and preserves all Oracle bytes. At v16,
Core totals were candidate 4,635 expected / 59 unexpected / 21 skipped /
43,762 not-run; Oracle 4,385 / 11 / 20 / 44,061. This changes
`docs/upstream-playwright-conformance.md` and `docs/design-tasks-v2.md`, not
the immutable v15 receipt or cumulative counts.

Read-only audits of the provisional evaluation timing patch and pinned
Python network50 reports add outstanding conformance gates. The selected
large-result evaluation route still needs cancellation-safe object-group
release, admission before retained browser data, and an inline-reply bound;
page-world `Object.prototype.constructor` mutation can break later evaluation.
Among 12 candidate-only network50 REDs, five concern raw header fidelity/API
surface and four concern navigation Response lifecycle or security metadata.
These are diagnoses and task priorities, not new passing official outcomes.
This triggers `docs/design-tasks-v2.md`,
`docs/upstream-playwright-conformance.md`, and
`docs/python-official-conformance.md`.

The next Core Chromium complete-file slice has now run and entered the
strict ledger: `selectors-css.spec.ts` has 27 pinned public-behavior IDs. Its
[manifest](../tests/official/playwright-core-selectors-css-public-chromium-v1-manifest.json)
and nine source/ID mutation checks pass; only the existing `deep-shadow.html`
asset is needed. All 27 unchanged cases ran once per dedicated-CDP lane:
[candidate](../.work/official-conformance/results/core-selectors-css27-candidate-20261021-v1.json)
14/27, [Oracle](../.work/official-conformance/results/core-selectors-css27-oracle-20261021-v1.json)
27/27. Both source-bound provenance sidecars independently verify, and the
13 candidate-only REDs remain open: seven composed-shadow CSS, four
scope/relative CSS, one capture+XPath composition, and one `$eval` world
selection. The independently verified immutable
[v17 paired receipt](../.work/official-conformance/outcomes/core-selectors-css27-strict-paired-v17/promotion-receipt.json)
(SHA-256
`8edfeb4781c19bdc2e9f1eddb5ebf7f9f7863396972a1a69f9c59e0cfd70fd18`)
replaces 27 rows per lane, preserving 48,450 off-target raw rows per lane.
At v17, Core totals were candidate 4,649 expected / 72 unexpected / 21
skipped / 43,735 not-run; Oracle 4,412 / 11 / 20 / 44,034. This
triggers `docs/upstream-playwright-conformance.md` and
`docs/design-tasks-v2.md`.

Python ABI131 navigation recheck successor: the second complete 80-case
source-bound run on the same installed wheel records 78 pass / two fail / zero
synthetic errors. Its independently verified immutable
[v9 receipt](../.work/official-conformance/outcomes/python-navigation-recheck-chromium-strict-paired-v9/promotion-receipt.json)
(SHA-256
`42738259dba662f23f324451ad094d0f31f2d9137ad01b1e3df14cd599bc4763`)
updates exactly 80 candidate rows after v8 and leaves 6,446 other candidate
and all 6,526 Oracle raw rows untouched. Candidate totals become 1,650 pass /
119 fail / three errors / 37 skip / 4,717 not-run. The v7 first-run
TargetClosed/teardown errors remain historical evidence, not an established
fix. This triggers `docs/python-official-conformance.md`,
`docs/upstream-playwright-conformance.md`, `docs/design-tasks-v2.md`, and
`packages/pwright-python/README.md`.

Python ABI131 and network full-file checkpoint: the installed default-off
frame-event wheel passes 290/290 browserless Python contracts and the
unchanged focused `window.stop()` case. Its first complete 80-case candidate
run is preserved in the independently verified
[v7 RED receipt](../.work/official-conformance/outcomes/python-navigation-full-chromium-strict-paired-v7/promotion-receipt.json)
(76 pass / two fail / two synthetic errors); a second source-bound complete
run on the same wheel records 78 pass / two fail / zero errors without erasing
the first. The pinned 50-case Python Chromium network manifest ran all its cases
in each HTTP/Tailscale lane: candidate 35 pass / 15 fail, Oracle 47 pass /
three fail. The independently verified
[v8 paired receipt](../.work/official-conformance/outcomes/python-network50-chromium-strict-paired-v8/promotion-receipt.json)
(SHA-256
`b4acfb11c8af4305e55c9f314c8a55193ed6b930bde1cd43b2c0239ae8f27189`)
updates exactly 50 rows per lane while preserving 6,476 other rows per lane;
the candidate ledger totals 1,648 pass / 119 fail / five errors / 37 skip /
4,717 not-run. Trusted-HTTPS Service Worker diagnostics run two unchanged cases
separately (Oracle 2/2, candidate 1/2); initial anchorless Chrome setup
errors remain quarantined. A source-only Page extra-header error-location fix
passes 293/293 browserless contracts but has no new official run. This
triggers `docs/python-official-conformance.md`,
`docs/upstream-playwright-conformance.md`, `docs/design-tasks-v2.md`, and
`packages/pwright-python/README.md`. No complete-corpus or unadapted parity
claim follows.

The next full Core Chromium `page-dialog.spec.ts` slice is pinned by a
[10-case manifest](../tests/official/playwright-core-page-dialog-public-chromium-v1-manifest.json)
and its source/ID checker passes 8/8. All ten unchanged cases then ran in
each source-bound CDP lane: the frozen candidate passed 9/10 and the Oracle
10/10. The sole candidate-only RED is the open-alert Context-close case at
source line 67; the timeout occurs at `page.evaluate` before Context close,
and its subsequent target-close cleanup error is preserved, not masked. The
independently verified immutable
[v15 paired receipt](../.work/official-conformance/outcomes/core-page-dialog10-strict-paired-v15/promotion-receipt.json)
(SHA-256
`b44fc60ac36d0db4b646b319bf2a430f6240511185aebcc74154857dac1d8110`)
changes exactly those ten rows per lane, preserving 48,467 off-target raw
rows per lane. Core candidate cumulative totals become 4,634 expected / 60
unexpected / 21 skipped / 43,762 not-run; Oracle 4,385 / 11 / 20 / 44,061.
The opt-in frame-tree release gate is clean (80 passed, two real-Chrome tests
ignored). This triggers `docs/upstream-playwright-conformance.md`,
`docs/design-tasks-v2.md`, and `packages/pwright/README.md`.

Core history full-file checkpoint: all 16 pinned Chromium
`page-history.spec.ts` assertions ran unchanged in each lane, candidate 11/16
and Oracle 13/16. The immutable [v14 paired receipt](../.work/official-conformance/outcomes/core-page-history16-strict-paired-v14/promotion-receipt.json)
(SHA-256
`07ce72583a7011139e9aeebc94c2be8c3573e29c619a4ed4107557ed44125b9e`)
updates exactly those 16 rows in each 48,477-row ledger, preserving all
48,461 other raw rows per lane. Its three candidate-only, two both-lane, and
one Oracle-only REDs remain explicit; adapted CDP execution is not full or
unadapted public-API parity.

Core `Page.fill` and Python navigation full-file checkpoint: all 45 pinned
Core Chromium `Page.fill` assertions pass in both frozen candidate and Oracle;
the immutable [v13 paired receipt](../.work/official-conformance/outcomes/core-page-fill45-strict-paired-v13/promotion-receipt.json)
(SHA-256
`8a10291c3301a8e6e0877387b9d6d279927eba31a1831ef0e3c420353d1c46de`)
changes one candidate row and preserves every other Core row byte-for-byte.
The latest complete Python Chromium async navigation-file candidate rerun
executes all 80 IDs on frozen ABI130-v2: 77 pass / two fail / one synthetic
report error, versus the unchanged Oracle 77 pass / three fail. Its immutable
[v6 paired receipt](../.work/official-conformance/outcomes/python-navigation-full-chromium-strict-paired-v6/promotion-receipt.json)
(SHA-256
`928da6d927abd7cf0667b63e88a89eb9fab260fe15ff0c50c04db358a295cb33`)
preserves all 6,446 off-target candidate and every Oracle raw row. The
preceding verified
[v5 focused URL-match receipt](../.work/official-conformance/outcomes/python-navigation-url-match-chromium-strict-paired-v5/promotion-receipt.json)
proves three official waiters, while v4 history and v2 Frame receipts preserve
their own frozen focused evidence and v1 preserves the 57-pass/23-fail RED.
The v6 candidate ledger totals 1,617 passed / 104 failed / four errors / 37
skipped / 4,764 not-run. The sole candidate-only unfinished navigation ID
requires subframe events; two cases also fail in the Oracle. A source-only
`Response.frame` fallback added after the wheel froze passes 283/283
source-overlay contract tests but earns no v6 credit. The Pyright baseline
gate now explicitly selects the isolated Python interpreter and still matches
exactly 66 historical errors across 91 files; 17 gate controls pass. These
are fixture-adapted dedicated-CDP Chrome executions, not full-corpus or
unadapted public-API parity. This triggers
`docs/upstream-playwright-conformance.md`,
`docs/python-official-conformance.md`, `docs/design-tasks-v2.md`, and
`packages/pwright-python/README.md`. The Node `Page.goForward()` source and
release native builds additionally trigger `packages/pwright/README.md`:
its source-bound official Core HistoryAPI case passes candidate and Oracle
1/1. A later full history-file run and v14 receipt above replace the prior
focused cross-document smoke assessment without erasing it.

Core `Page.selectOption` full-file checkpoint: the 32 unchanged upstream
Chromium assertions pass 32/32 in a frozen candidate and 32/32 in the Oracle
through an adapted `connectOverCDP` fixture. The default-off
`select-option-navigation-commit` feature composes into the full Page
profile. A no-binding publication-drain guard avoids evaluating a retired
execution context after selection-triggered navigation, with a remaining
cross-Page binding-scope caveat. The immutable v11 receipt keeps the
historical 31/32 candidate RED; the independently verified
[v12 successor](../.work/official-conformance/outcomes/core-page-select-option32-strict-paired-v12/promotion-receipt.json)
(SHA-256
`40c861162bbec094b8729927366000ac3b3026586ce98d16e48dcedb1e6c2dc3`)
changes only that candidate row. This triggers and updates
`docs/upstream-playwright-conformance.md`, `docs/design-tasks-v2.md`,
`docs/knowledge/pwright-locator-select-option.md`, and
`packages/pwright/README.md`. It provides bounded Chromium browser evidence,
not unadapted public-API parity, complete corpus execution, or remote CI.

Opt-in BiDi startup-failure checkpoint: `docs/upstream-playwright-conformance.md`,
`docs/design-tasks-v2.md`, and `packages/pwright/README.md` are triggered by
the new default-off Node/Rust launch-failure profile. Its six channel aliases,
explicit executable, bounded startup logs/time, owned profile and process-group
cleanup are implementation evidence only. Rust leaf release 8/8, Node adapter
46/46, native integration 3/3, addon ABI/load, browserless package 829/829,
TypeScript and focused addon Clippy pass locally. The expanded package-boundary
check now proves the exact new selected closure and preserves older profiles;
full-dependency strict Clippy still hits two pre-existing `pwright-node-host`
`too_many_arguments` warnings. A browserless CI profile step was added but
has not run remotely. The six unchanged official tests now have a separate
strict v3 candidate/oracle capture with 12/12 expected outcomes. That
failure-only result is detailed below; it neither creates a Browser/BiDi
parity claim nor rewrites the historical v9 ledger. A separate v10 successor
has now been published and independently verified, as detailed below.

Python CI interim type gate: one unpinned unit-test annotation is corrected;
the unchanged full 91-file strict-Pyright target set now has 66 explicitly
baselined diagnostics in five SHA-pinned historical plugins. A new versioned
gate and 16 mutation controls pass locally; the pending CI workflow invokes
that exact gate and excludes only two hash-pinned old plugins with Ruff I001
debt from lint. This is not zero-error typing and has not run in remote CI.
Separately, 22 CDP-session Firefox/WebKit marker nodes are not promotable from
their markers alone: session-browser setup precedes the function-level skip.
None of those 22 rows was promoted. A different 11-node callable-fixture
skip successor is recorded below.

Core version-mismatch diagnostic: the new
`run_playwright_core_runserver_url_only.py` v1 wrapper and
`run_playwright_core_runserver_url_owner_v2.py` v2 wrapper keep the existing
Core runner and sealed provenance unchanged. Their manifests, URL-only
preload, owner adapter, distinct local envelopes and 16 focused tests attest
the unchanged upstream `browsertype-connect.spec.ts:1046` assertion on an
upstream `run-server` bound to Tailscale. Oracle passes 1/1 in both pairs;
candidate first stops at a private fixture owner pointer, then reaches public
`connect` in v2 and fails on a generic WebSocket error instead of the HTTP 428
version-mismatch explanation. These are fixture-adapted diagnostics with no
Core ledger promotion. The bounded transport/body/error-mapping repair is
now implemented in the opt-in Playwright Server live path. A fresh isolated
v3 candidate package (release addon SHA-256
`7a2664ef66516bcd1ada56b962a9f4a85abb05add8bcf3905113490fa9f39938`)
passes the unchanged official assertion against oracle 1/1. Both v3 envelopes
and earlier v2 evidence independently reverify. A new immutable paired
successor receipt (SHA-256
`5e6180a6b2e0a90d764276af84b933b14b6117cc4dd799a0f925622ea347e1b8`)
promotes just that `not-run` to `expected` internal diagnostic row per
48,477-row Core lane. The other 48,476 lines are unchanged, independent replay
and 49 mutation controls pass, and public-API/browser parity credit remains
zero. The upstream fixture starts a remote server, not a browser. No claim of
global WebSocket error redaction is made. CI now includes the 49 pure verifier
controls but still has no clean-runner input path for the ignored full outcome
bundle; the new CI step has not run remotely.

Core Firefox/WebKit WS-error diagnostic: four additional unchanged upstream
`browsertype-connect.spec.ts:122` invocations pass oracle/candidate 4/4 in
strict browser-free lanes. A v3 immutable paired successor receipt (SHA-256
`59c7cbb11ad7284ab8aedadb8c850cd4552ffaa0b035b70430c8b4e2e4ef0ef7`)
records only those four `not-run` to `expected` outcomes in each 48,477-row
ledger. The in-memory fixture adaptation and internal source classification
remain explicit; no public-API or browser parity credit is claimed.

Core Bidi Chromium/Chrome WS-error diagnostic: six unchanged upstream
`browsertype-connect.spec.ts:122` invocations pass 6/6 in each isolated
oracle/candidate lane, with one clean attempt per official ID and no browser
launch. The immutable [v4 paired receipt](../.work/official-conformance/outcomes/core-server-connect-ws-error-bidi-chromium-strict-paired-v4/promotion-receipt.json)
has SHA-256
`f7c72c003d4306ade21f077b8b424a60a21d265a72512c5b8ac249c21886d2c6`.
Only six `not-run` rows become `expected` in each 48,477-row ledger; 48,471
other raw lines are byte-identical. Fixture adaptation and
implementation-internal classification deny public-API/browser parity credit.
The clean-CI tooling step SHA-pins all five sealed files with exact historical
project-config Ruff debt: E501 in the capture runner/verifier, I001 in the
capture mutation test, E501+I001 in the promoter, and I001 in the promotion
mutation test (24 diagnostics total). It applies only those per-file ignores,
checks default Ruff formatting on all six files, strictly lints the independent
paired verifier without ignores, and runs 35 clone-free pure mutation cases.
Source-pin and full paired-evidence checks still require ignored local
`.work` inputs; remote CI has not run this step. Clean source requires new
versioned capture/promotion evidence, not edits to hash-pinned v4 files.

Core Bidi-config CDP custom-Close diagnostic: three unchanged upstream
`connect-over-cdp.spec.ts:572` invocations pass 3/3 in each strict browser-free
oracle/candidate lane, with one attempt per official ID. The immutable
[v5 paired receipt](../.work/official-conformance/outcomes/core-cdp-close-bidi-chromium-strict-paired-v5/promotion-receipt.json)
has SHA-256
`13d7df7ccf58fb1838c5f523471a77fadd3b7ad92330d086f13c8d3ac64df068`.
Only three `not-run` rows become `expected` in each 48,477-row ledger; 48,474
other raw lines are unchanged. Independent replay and 36 focused tests,
including mutation controls, pass. The protocol is CDP-over-WebSocket despite
Bidi project names, and the candidate is a frozen v3 package. Tailscale fixture
adaptation and internal source classification deny public-API, browser,
Bidi-protocol, and current-live-package parity credit. The capture
runner/verifier are sealed with historical strict-Ruff E501 debt; all three
new v5 promotion scripts pass strict Ruff.
The clean-checkout CI step SHA-gates E501 only for those two v4 files, strictly
lints v5 and runs 15 synthetic tests. Full replay and 36/36 tests remain
local until a verified clean-CI evidence bundle exists; remote CI has not run.

Core Bidi Chromium Library timeout/redirect diagnostic: five unchanged
upstream `browsertype-connect.spec.ts:221`, `:228`, and `:1056` assertions pass
5/5 in each fresh strict oracle/candidate lane, with one attempt per ID. Two
localhost WebSocket literals and the common fixture host/bind are adapted;
no browser starts. The immutable [v6 paired receipt](../.work/official-conformance/outcomes/core-server-connect-timeout-redirect-bidi-chromium-strict-paired-v6/promotion-receipt.json)
has SHA-256
`43b8f240c16d7c5517fcb16c90f77828c0818552be5e19e368a900356ca266ba`.
Exactly five `not-run` rows become `expected` per 48,477-row lane, while the
other 48,472 raw lines remain byte-identical to v5. Independent raw-evidence
and full-ledger replay passes. Candidate now has 4,563 expected / 43,838
not-run / 21 skipped / 55 unexpected; oracle has 4,312 / 44,137 / 20 / 8.
The frozen candidate snapshot, implementation-internal source classification,
and fixture adaptation mean zero public-API, browser, BiDi-protocol, or
current-package parity credit. Clean CI SHA-pins the seven manifest/source
files, strictly lints six Python scripts under the project Ruff config, and
runs 91 pure focused tests, including mutation controls. The ignored `.work`
evidence and full official runs are not available in clean CI; this step has
not run remotely.

Core Bidi Chromium Library HTTP-401 diagnostic: six unchanged upstream
`browsertype-connect.spec.ts:116` assertions pass 6/6 in each strict
oracle/frozen-candidate lane, one attempt per expanded ID. A localhost
WebSocket URL host and the common fixture bind are adapted; no browser or
remote browser server starts. The immutable
[v7 paired receipt](../.work/official-conformance/outcomes/core-server-connect-http-401-bidi-chromium-strict-paired-v7/promotion-receipt.json)
has SHA-256
`ef2b8a1d16fd3758c784071e50777df0b35935e04b9c1a3eaa8bf3a60a339f9a`.
Only six `not-run` rows become `expected` per 48,477-row lane; the remaining
48,471 raw lines are byte-identical. Candidate totals are 4,569 expected /
43,832 not-run / 21 skipped / 55 unexpected; oracle totals are 4,318 /
44,131 / 20 / 8. This remains an implementation-internal, fixture-adapted
diagnostic with no public-API, browser, BiDi-protocol, or current-package
parity credit. The local clean-CI tooling check hash-pins seven sources,
strictly lints six Python scripts and passes 114 pure tests, including
mutation controls. A separate upstream/snapshot pin test passes locally but
requires ignored inputs and is explicitly excluded in clean CI. Remote CI
has not run and does not have the ignored raw evidence.

Core Bidi Chromium Library custom Close diagnostic: six unchanged upstream
`browsertype-connect.spec.ts:130` assertions pass 6/6 in both strict lanes.
Across both lanes, twelve ordered WebSocket upgrade/message/Close
`4123 / Oh my!` triplets were observed in total (two per project per lane),
but the guard cannot prove a per-spec transport assignment. The immutable
[v8 paired receipt](../.work/official-conformance/outcomes/core-server-connect-close-error-bidi-chromium-strict-paired-v8/promotion-receipt.json)
has SHA-256
`4c0a72a233440263a38bf718fa003fa2e8f8c36ef0243aaac9811c314e14dd68`.
Only six `not-run` rows become `expected` per 48,477-row lane; the other
48,471 raw lines remain byte-identical. Candidate totals are 4,575 expected /
43,826 not-run / 21 skipped / 55 unexpected; oracle totals are 4,324 /
44,125 / 20 / 8. This is fixture-adapted implementation-internal evidence,
not public-API, browser, BiDi-protocol, or current-package parity. Local
strict Ruff and 98 focused tests pass; the clean-CI tooling step runs 97
pure tests and explicitly excludes one local upstream/snapshot pin test.
Remote CI has not run.

Latest Core clean-CI replay is still open: the existing relocatable package
contains an older v1 chain, not the historical v8 ledger. A new deterministic
local v8 ledger-only archive (5,870,164 bytes; SHA-256
`5d457866faaf95b7ed40130282de4ba00c07c1a49160f0c4aecb84614a0d9a95`)
passes isolated full-row replay and focused mutation tests. It includes local
absolute paths, remains unpublished, and is unavailable to clean CI. The
clean-CI tooling step pins its scripts and runs 24 pure tests but cannot replay
the archive. The current v9 successor is not included. A privacy-reviewed
distribution and pinned replay gate are still needed; that would validate
historical integrity, not re-execute official tests. Full
provenance replay also needs raw captures, the frozen addon and a portable
upstream path; none is available in a clean checkout yet.

Core public launch-failure v2 diagnostic: six unchanged official Bidi-config
`browsertype-launch.spec.ts:64` tests run in paired oracle/candidate lanes.
Joint verification accepts six oracle passes with actual exit-1 dummy Node
children and six candidate failures at the original assertion; the candidate
rejects merged `firefoxUserPrefs` before spawn. The earlier v1 Firefox
guard-generated pass is excluded. The historical v8 ledger leaves these six
rows `not-run`; no public parity, browser execution, or BiDi protocol claim
follows. The new clean-CI tooling step runs 43 pure tests but not ignored
captures, and
remote CI has not run.

Core public launch-failure v9 paired successor: the immutable
[receipt](../.work/official-conformance/outcomes/core-dummy-node-bidi-strict-paired-v9/promotion-receipt.json)
has SHA-256
`f35fa767281337803ea16ece4a12bb36de52523fe9e348fc7f4467402c3f215f`.
Exactly six previously `not-run` public `BrowserType.launch` rows at indexes
6,110-6,115 become candidate `unexpected` and oracle `expected`; the other
48,471 raw lines per 48,477-row lane are unchanged. Candidate totals are
4,575 expected / 61 unexpected / 21 skipped / 43,820 not-run; oracle totals
are 4,330 / 8 / 20 / 44,119. Its frozen-addon, fixture-adapted evidence
establishes neither browser execution, public-API parity nor WebDriver BiDi
protocol parity. Clean CI pins three v9 scripts and runs 30 pure focused
tests, including mutation controls, but cannot replay ignored paired captures;
remote CI has not run.

Core public launch-failure v3 capture: the
[strict evidence](../.work/official-conformance/results/core-dummy-node-bidi-strict-v3/)
contains six candidate and six oracle single-attempt passes of the unchanged
`browsertype-launch.spec.ts:64` assertion. The independent verifier checks
all sidecars and pairing, real exit-1 dummy Node descendants, profile cleanup,
and candidate public method admission. The fixture substitutes the executable;
it provides narrow failure-only evidence, not browser execution, a usable
Browser, broad public-API parity, or BiDi protocol parity. The six source
inputs are SHA-pinned in a new clean-CI gate with 58 locally passing pure
mutation tests. The three frozen Python sources pass Ruff lint but are not
project-config Ruff-format clean, so CI preserves their exact evidence-linked
bytes. Ignored captures cannot be replayed there, and remote CI has not run.
The v10 successor below is now published; v9 remains historical candidate-RED.

Core public launch-failure v10 paired successor: the immutable
[receipt](../.work/official-conformance/outcomes/core-dummy-node-bidi-strict-paired-v10/promotion-receipt.json)
has SHA-256
`2ae0478daf066539db4c9904e344dc7c8b102ff65eda8ab9587f2b591f8903e2`.
Independent replay verifies that exactly six target rows per 48,477-row lane
change and the other 48,471 raw lines remain byte-identical to v9. Candidate
totals are 4,581 expected / 55 unexpected / 21 skipped / 43,820 not-run;
oracle totals remain 4,330 / 8 / 20 / 44,119. The old v9 receipt and ledger
bytes are hash-sealed, not live-replayed against the new source; the twelve v3
sidecars are replayed. Fixture adaptation and failure-only scope explicitly
exclude Browser, broad public-API and BiDi-protocol parity. A new clean-CI
step pins three v10 tooling scripts and passes 35 pure mutation tests locally,
but cannot replay ignored captures or ledgers. Remote CI has not run.

Relocatable Core historical snapshot: a new 16-input package keeps the
pinned manifests, inventory, and three paired outcome/receipt generations
byte-identical while allowing a clean-directory checker to replay all 48,477
rows and receipt hashes. The local ignored 172 MiB bundle has sidecar SHA-256
`da9b093b5e914371f93137f72707f6574fd8a64f3dff13ac791349f888f74fc3`.
A deterministic 8,215,934-byte archive has SHA-256
`e5a7ab7af91eedb7b1341e8fb453402aeefe0eabb20ddc2887b8f68bb4362186`;
its hash-pinned, bounded USTAR verifier and 49/49 package/coverage tests pass.
This is not official re-execution. CI still lacks a published, verified input
path, and absolute provenance paths require review before public distribution.

Full Python collection CI gate: fresh pinned Python 3.12 core and hash-locked
Python 3.10 examples environments reproduce the complete 6,526-node,
156-source-file official inventory at SHA-256
`3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41`.
The local CI workflow now collects, checks that digest, and runs four
inventory unit tests; remote CI has not
run this addition. Collection does not execute assertions or verify paired
candidate/oracle outcomes.

Official Python marker-skip promotion: 15 exact pinned upstream nodes each
produced candidate/oracle `skipped` before test-body execution under a
deny-browser-exec sandbox. All 15 final pairs reverify and six negative
controls pass; the latter are wired into local CI. A new immutable paired
successor with receipt SHA-256 `21c65650e7f3b8c35c91e4d08b317227fe9ba94b06c0cc83c4fba0b2390287f1`
changes exactly those 15 `not-run` rows to `skipped` in each 6,526-row lane,
preserving all other 6,511 row bytes and every passed count. Ten promoter
mutation controls pass. This earns zero public-behavior/parity credit;
only the `-final-v2` ignored evidence directory is current.
The new guard passes strict Pyright. The older 67-error Python type finding
has since narrowed to the 66-error interim baseline above; no full-CI green
claim is made.

Official Python callable-fixture marker-skip promotion: 11 more unchanged
upstream nodes produced candidate/oracle setup `skipped` under deny-exec and
deny-outbound sandboxes. A pre-body guard and exact upstream skip reasons
verify all 11 pairs; ten promoter and six verifier mutation controls pass.
The immutable [v2 paired receipt](../.work/official-conformance/outcomes/python-upstream-skip-callable-fixtures-strict-paired-v2/promotion-receipt.json)
has SHA-256 `1b8ce858fa3ab643fb10e8f12142c12b32e3164b8719e2c627d2f6e4a66dbb4a`.
Each 6,526-row successor changes only 11 `not-run` rows to `skipped`, leaving
6,515 other row bytes and all passed/failed/error counts unchanged. Current
candidate totals are 1,542 passed / 102 failed / three errors / 37 skipped /
4,842 not-run; oracle totals are 1,309 passed / four failed / 37 skipped /
5,176 not-run. The preceding 15 skip rows are retained; all 26 cumulative skips
earn zero new public-behavior/parity credit. The separate 22 CDP-session
marker cases remain `not-run` because their browser fixture runs first.

Full Core source-inventory CI gate: the browser-free
`verify_playwright_core_full_source_inventory.py` preflight validates the
pinned upstream commit and full manifest SHA, 558 tracked spec sources, 20
declared suites and 48,477 declared invocations, tracked runner/config inputs,
and the four explicit non-collected fixture specs. The generated
`playwright-test` runner is deferred until dependency installation and its
tracked package source is checked. The local pinned checkout and 15 focused
controls pass. This is a source/declaration gate, not execution or paired
outcome proof; the ignored `.work` ledger bundle still needs a clean-CI input
path.

ABI127 installed-wheel checkpoint: the independent default-off Python
`playwright-server-close-notices` feature selects a Rust close-only session
projection and browser-ops close-state boundary. The additional projection
retains only remote Browser/Context/Page topology and ordered GUID/generation
close notices, not Request/Response notices; the base supervisor still
retains validated protocol objects under 16,384-GUID/4 MiB bounds. Python's native
`next_close_notice()` reader and one facade watcher use generation-keyed
wrapper registries and a bounded wrapper-publication fence to deliver
server-origin Page/Context callbacks while the Browser remains connected;
`disconnected` follows terminal connection closure. The prior remote profile
remains available without this opt-in feature. A fresh ABI127 release wheel
(SHA-256 `8b524b5b424ff2b5dd3a8f871af3a6dbae961fddfdaf895b6000ecc5ff919cb9`)
passes 84 focused installed-wheel tests. The Tailscale-bound synthetic
WebSocket fixture passes ten additional independent reruns, checking the
Page/Context close-before-dispose window, native live inventories, and ordered
Page→Context→Browser callbacks. All-feature Rust release tests pass 41/41
browser-ops, 22/22 page-events and 65/65 session; selected addon strict
Clippy passes. A separate close-notices-only ABI127 wheel (SHA-256
`02a13c04b0b1b9bee4e9e5fa9f0dbdb3a98bda8a61f53c05198f5af11885e4a4`)
omits the frame-title feature and native Page.title method while 51 focused
close/ABI/fake-connect tests pass. The separate default-feature ABI127 wheel
(SHA-256 `731fd13c02b5cc716533f8f0f94027027d51cf94d860cd2656a18d039b2ea9c7`)
passes 661 browserless Python units with 13 explicit opt-in-profile skips and
`uv pip check`. This is not a real Playwright Server/browser or unchanged
official-test proof, and promotes no ledger row. The ABI126 wheel evidence
below remains distinct.

ABI126 remote Page checkpoint: the opt-in Rust `frame-title` operation is
exposed through the Python binding and a freshly installed release wheel
(SHA-256 `ac2f027be220ce59160e6df716c38bb6bd74f48fcfa17f3fed8af22dbfa77f05`).
The focused Python suite passes 70/70, including a real Tailscale-bound
WebSocket exchange against a synthetic protocol server with a main Frame
adopted to its Page; `Page.title()` sends `Frame.title` to that Frame. A
second fixture drops the socket with that command pending and verifies
`TargetClosedError` and ordered Page→Context→Browser notices; it passed
ten additional independent reruns against the rebuilt wheel. The Rust actor
now checks the full GUID lineage, adoption epoch, and Page close tombstone
before send and at the ordered reply. Selected `frame-title` release tests
pass 37/37 browser-ops and 48/48 session; all-feature tests pass 40/40 and
57/57. Strict addon Clippy passes,
and the selected addon adds no normal dependency package. The frozen Python
remote-kill v1/v2 RED/GREEN inputs are separately sealed under
`.work/official-conformance/seals/python-remote-kill-v1-v2-inputs-v1/`;
sealed replay and 10/10 mutation tests pass despite live-source drift. The
seal remains machine-local and is not clean-CI portable. Neither synthetic
fixture nor source-level tests change an official conformance ledger row;
server-origin Page close callbacks remained unverified in this ABI126 wheel
and were later tested through the ABI127 synthetic wheel above.
Real-server/browser validation remains open. The pre-fence ABI126 wheel SHA-256
`e4ef50d9b922530c7705022fd352e2b21741fc84e15c9587f0034d495ef0b713`
remains historical and is not the current tested build.

Core browser-free public-launch conformance checkpoint: a verified
[strict candidate/oracle pair](../.work/official-conformance/outcomes/core-dummy-node-launch-strict-paired-v1/promotion-receipt.json)
passes the pinned Chromium Library `browsertype-launch.spec.ts:64` assertion
1/1 in each lane. It attests a real dummy Node exit 1, CLI/child ancestry,
temporary profile ownership and cleanup, and the Rust-backed addon identity.
An immutable successor promotes exactly that row in each 48,477-row ledger.
The historical [skip-only successor](../.work/official-conformance/outcomes/core-upstream-skips-strict-paired-v1/promotion-receipt.json)
records ten pinned upstream skips per lane and denies parity credit. The
latest [Firefox/WebKit RED successor](../.work/official-conformance/outcomes/core-dummy-node-other-engines-strict-paired-v1/promotion-receipt.json)
then changes exactly two more rows per lane: official oracle expected 1/1 for
each engine, Pwright unexpected 1/1 because neither engine's launch is
installed. Candidate totals are 4,544 expected / 55 unexpected / 21 skipped /
43,857 not run; oracle totals are 4,293 expected / eight unexpected / 20
skipped / 44,156 not run. Its two candidate rows have no native-runtime,
process, or profile attestation, and earn no parity credit. All other row
bytes match the predecessor. These fixture-adapted tests are not an
unadapted whole-suite or real-browser result; historical ledgers remain.

Python ABI125 closed-target browser-free diagnostic: the
[six-case connect-state attestation](../.work/official-conformance/results/python-connect-lifecycle-synthetic-v2-closed-target-oct20/attestation.json)
pins the selected wheel SHA-256
`32c414d3eb2a45417c4a3ba79705be96179dc2ec2dd20896cd0cda7d8169e3e8`.
Unchanged async/sync state assertions pass 6/6 in each oracle and candidate
lane across Chromium, Firefox and WebKit parameters; 13 verifier/mutation
tests pass. Its `launch_server` fixture is replaced by a synthetic protocol
server, so it is explicitly nonpromotable diagnostic evidence. All six
corresponding Python ledger rows remain `not-run` in both lanes.

Core adapter coverage: the pinned read-only
[report](../scripts/report_playwright_core_adapter_coverage.py) and its
[29 focused tests](../scripts/test_report_playwright_core_adapter_coverage.py)
reconcile the 48,477-row inventory and paired ledgers with pinned source and
receipt hashes. Of 43,857 candidate `not-run` rows, 33,188 have no configured
suite-level candidate adapter; 21,810 of these are public API candidates.
Another 7,675 `library` rows select Firefox/WebKit but have only the
Chromium-CDP preload; the remaining 2,994 `not-run` rows have a configured
Chromium route, not verified execution. Missing or incompatible adapters do
not imply non-applicability or a pass. No official ledger row changes as a
result of this report.

Python remote-kill diagnostic: the immutable
[four-case RED attestation](../.work/official-conformance/results/python-connect-remote-kill-synthetic-v1-oct20-final/attestation.json)
records oracle 4/4 and frozen ABI125 candidate 2/4; the missing behaviors
are disconnected and ordered child-close events. Its independent verifier
and 23 mutation tests pass. A separately built, pinned ABI125
[GREEN successor](../.work/official-conformance/results/python-connect-remote-kill-synthetic-v2-terminal-fix-oct20/attestation.json)
passes oracle 4/4 and candidate 4/4 in five independent runs (20/20 candidate
assertions); its independent verifier and 19 focused v2 mutation tests pass.
The prior RED attestation is unchanged. Because `launch_server` is synthetic,
the four official Python ledger rows remain `not-run`. Newer Rust session
source orders Closed publication before pending/current terminal errors;
53/53 page-events release tests and strict Clippy pass. Neither frozen wheel
contains that later source repair or proves a live remote Page operation.

Playwright Server follow-up: the default-off session composition
now replays bounded pre-initialize lifecycle/event ingress in wire order,
rejects a pre-reply Browser close/disposal, and passes 19/19 default and
live-feature release tests. Its optional supervisor drains bootstrap events,
settles up to 128 pending commands, bounds ordered events to 128 entries and
approximately 4 MiB of serialized payload, and reports `Stopped` after an
unexpected actor exit. Its current all-feature release suite passes
40/40 plus strict Clippy. The queue remains the default policy; bindings with
no public remote event listeners select an explicit drop policy that preserves
protocol validation, GUID projection and terminal state without retaining
unsupported ordinary events. The supervisor now
retains a bounded GUID initializer graph, exposes a pre-reply ingress count,
and provides a terminal watcher that does not keep the socket owner alive. The
new independent default-off endpoint
discovery crate resolves HTTP(S) `/json` to WS(S) with an injected fetcher or
explicit `live-reqwest` feature; default and live-feature release suites pass
12/12 and 15/15 respectively, with strict Clippy. Three selected Tailscale-bound
live HTTP discovery fixture tests pass, covering direct lookup, a cross-origin
304 redirect with no query-bearing `Referer`, and the five-hop limit. The live
fetcher follows any
valid-Location 3xx under one cumulative deadline, whereas the pinned
upstream helper has no explicit count limit. Two selected live WebSocket
fixtures also pass for the raw lease and supervisor against a synthetic server.
An explicit `pwright-playwright-server-connect` composition connects direct
WS(S) or HTTP(S) discovery to the validated supervised lease without adding
Node/Python policy or a browser launch. Its default and all-feature release
suites pass 9/9 each, plus one selected synthetic Tailscale-bound HTTP-to-WS
fixture. The default dependency closure has no Reqwest or Tungstenite.
An independent default-off browser-ops crate passes 19/19 release tests for
typed `newContext`/`newPage` GUID handles and empty-option Context close,
with parent-generation revalidation and correlated close replies, without a
full Context/Page API.
The opt-in Node remote-session profile now returns a Browser and composes
empty-option Context/Page creation through Rust browser-ops. Four earlier
Tailscale-bound synthetic native tests and 18 adapter tests pass, including
17 retained-wrapper connect/close cycles, a 160-event unsupported-event burst,
whole-session Page→Context→Browser close ordering, and GUID-disposal/reuse
checks. A new selected Tailscale-bound real-addon fixture passes 2/2 for
explicit Context close and `Browser.newPage()` implicit-Context cleanup on
`Page.close`, Page-before-Context event order, and a still-connected Browser.
Non-terminal externally initiated child-close notifications remain unsupported.
Python ABI125 separately returns a remote lifecycle Browser with empty-option
Context/Page creation and graph-backed inventories that fail closed on unknown
or recycled generations; 49/49 focused fresh installed-wheel Python tests pass
on each CPython 3.9.6 and 3.12.14,
including four Tailscale-bound synthetic variants (one with a 160-event burst),
and current Rust addon source passes strict release Clippy and wheel build.
A prior addon checkpoint passed 41/41 release tests; direct current `cargo test`
cannot link on this Mac without `libpython3.9`. A new selected ABI125 wheel
adds explicit empty-option Context close and `Browser.new_page()` implicit
Context ownership. A fresh focused installed-wheel subset passes 65/65 tests
on both CPython 3.9.6 and 3.12.14 when the Tailscale-bound fixtures are enabled,
including 6/6 compiled-extension WebSocket cases. The wheel and Node addon
pass selected strict Clippy. Arbitrary externally
initiated child close and Context-owned `Page.close` remain unsupported.
Closed remote Pages now report the target-closed error before an uninstalled
action error; the pre-fix RED diagnostic is retained below.
Neither binding has real Playwright Server
validation. A pinned, fixture-adapted Core lifecycle
diagnostic passes the unchanged line-304/321 assertions 2/2 in both oracle
and candidate lanes, with an independent verifier and four negative controls.
The separate fixture-adapted Python lifecycle diagnostic passes four unchanged
async/sync assertion bodies across three browser-engine parameters, 12/12 in
both oracle and candidate lanes with an independent verifier and nine negative
controls.
The pinned Python sync Chromium child-close diagnostic also passes its
unchanged assertion body 1/1 in each oracle and selected-wheel candidate lane,
with independently verified fixture-adapted evidence and no ledger promotion.
The separate pinned Core line-382 child-close diagnostic preserves oracle 1/1
versus candidate 0/1 timed-out RED evidence and a verified 1/1-each GREEN
successor after the Node terminal-emission change; both use a replaced
server-launch fixture and earn no official ledger credit.
These fixture-adapted diagnostics are not unchanged official-server passes;
no official ledger rows were changed. See
[the Server boundary](knowledge/pwright-playwright-server-connect.md).

Core connect-timeout diagnostic, fixed v2: the immutable browser-free v1
selection exposed 18 candidate failures among 36 unchanged timeout assertions
(`ws-slow` 1-second upgrade delay cases), against oracle 36/36. The typed
`ConnectError::Session(SessionError::WebSocket(ConnectTimeout))` mapping now
produces `TimeoutError`. A newly built, isolated addon reran the same 36 rows
with oracle 36/36 and candidate 36/36, independently verified in the
[v2 attestation](../.work/official-conformance/results/core-server-connect-timeouts-strict-v2/core-server-connect-timeouts-strict-v1-fixed-v2-20261012/attestation.json).
Two literal fixture URL hosts were adapted; this implementation-internal
diagnostic earns no official ledger credit and the v1 RED evidence remains.
Two additional browser-free, official-derived pre-fix RED diagnostics are retained
without ledger credit. The [Core HTTP-redirect timeout v1 evidence](../.work/official-conformance/results/core-server-connect-redirect-timeout-strict-v1/core-server-connect-redirect-timeout-strict-v1-second-20261012/attestation.json)
is oracle 9/9 versus candidate 0/9: Pwright rejects a reserved WebSocket
header before the HTTP discovery timeout. The independently verified [Core v2 GREEN successor](../.work/official-conformance/results/core-server-connect-redirect-timeout-strict-v2/core-server-connect-redirect-timeout-strict-v2-fixed-20261012/attestation.json)
passes the same nine unchanged assertions 9/9 in each lane. The fix validates
header syntax and size before HTTP(S) discovery, defers reserved-handshake-name
rejection until after discovery and before WebSocket upgrade, and leaves direct
WS(S) preflight strict. The real-addon fixture passes 3/3, and the strict v2
verifier passes seven mutation controls (nine tests total). The [Python navigation-after-close
v1 evidence](../.work/official-conformance/results/python-connect-navigation-closed-synthetic-v1-red-v2/attestation.json)
is oracle 3/3 versus candidate 0/3: the facade reports its unsupported `goto`
before the target-closed error. Both preserve unchanged assertion bodies and
use versioned fixture adaptations; neither changes canonical ledgers.
The independently verified [Python v2 GREEN successor](../.work/official-conformance/results/python-connect-navigation-closed-synthetic-v2-green-v1/attestation.json)
passes those same three unchanged assertions 3/3 in each lane with a rebuilt
ABI125 wheel and corrected closed-target error precedence; the v1 RED stays
intact, and no ledger row was promoted.
The historical, independently verified [Core remote-Page waiter RED](../.work/official-conformance/results/core-connect-waiters-synthetic-v1-red-v5/attestation.json)
selects three unchanged `run-server` assertions with a browser-free
Tailscale-bound fixture: oracle 3/3, candidate 0/3 on the then-current pinned
context-close-capable addon. All candidate runs create Context/Page, then
fail at the first missing waiter call; none reaches the assertion or proves
request/response/download delivery or terminal ordering. Line 396 never
invokes `waitForResponse`. Nine verifier tests pass. This is an internal
diagnostic with no official ledger promotion.
A default-off
[`pwright-playwright-server-page-events`](../crates/pwright-playwright-server-page-events/src/lib.rs)
leaf now consumes validated, wire-ordered ingress into bounded, Page-attributed
Request/Response/Download and terminal notices (16/16 release tests). The
session's separate `ProjectPageNotices` policy projects notices inline and drops
unrelated events after validation; its selected release suite passes 50/50,
including a 160-event burst, concurrent readers and terminal ordering. Rust
browser-ops acknowledges both Page network subscriptions before a selected
Node `newPage` is published (22/22 selected release tests). A separate
`playwright-server-connect-page-events` Node addon builds and loads, with
typed notices, generation-fenced identities and bounded URL-glob compilation.
The separate selected JS adapter passes 25/25 focused tests, including early
Context/Page terminal publication and terminal-predicate regressions; the
package's default suite includes it and passes 816/816, with strict TypeScript
checks passing. Actor-side command admission now rejects a target GUID recycle
already observed by Rust before send, without consuming a wire command ID;
it cannot prevent an unseen server-side recycle. The independently verified
[v5 GREEN diagnostic](../.work/official-conformance/results/core-connect-page-events-synthetic-v2-green-20261020-v5/attestation.json)
records the same three unchanged Core teardown assertions passing 3/3 in
both oracle and real-addon candidate lanes, plus a separate positive
Request→Response→Download waiter and Page-listener probe in each lane. The
server launcher is replaced by a browser-free synthetic fixture, so this
is neither an unadapted official pass nor real Playwright Server/browser or
full public-API parity proof. Python does not select this capability, and no
official ledger row has been promoted.

Latest Core connect-header diagnostic: the post-arity-refactor v2 selected
`playwright-server-connect-headers` artifact uses a bounded three-string
runtime-identity tuple and profile-local ABI-shape marker `2`; old header
addons are rejected. The four original pinned line-235/250 cases again pass
4/4 in both independent candidate and oracle lanes. The [candidate](../.work/official-conformance/results/core-server-connect-headers-strict-v1/post-refactor-v2/candidate-v1.strict.json)
and [oracle](../.work/official-conformance/results/core-server-connect-headers-strict-v1/post-refactor-v2/oracle-v1.strict.json)
sidecars pin artifact SHA-256
`0b2553d3137a77f162836d8fadeacbf1d5e47038b24ae97fe9d1c9519d068194`.
Only two fixture WebSocket URL hosts were adapted. Four ordered wire receipts
per lane lack per-spec socket IDs, so variant attribution is ordinal. This is
fixture-host-adapted internal diagnostic evidence, not public API parity, an
unadapted official pass, or a Core ledger promotion; v1 remains historical.

Historical browser-free Core connect-header v1 diagnostic: the default-off
`playwright-server-connect-headers` profile composes explicit and default
WebSocket request headers in bounded Rust. Original pinned
`browsertype-connect.spec.ts:235,250` assertions passed 4/4 in both frozen
candidate and independent oracle lanes, expanded across `launchServer` and
`run-server`. Only two fixture URL hosts were adapted to full Tailscale
MagicDNS. The [strict candidate](../.work/official-conformance/results/core-server-connect-headers-strict-v1/final-frozen-v1/candidate-v1.strict.json)
and [oracle](../.work/official-conformance/results/core-server-connect-headers-strict-v1/final-frozen-v1/oracle-v1.strict.json)
sidecars pin the pre-ABI-arity-refactor artifact SHA-256
`30d8478b0d74acfe556b5209633045434503738c4424483ab7ea7bb952f358aa`.
Four ordered wire receipts per lane have no per-spec socket ID; assignment is
ordinal. This is fixture-host-adapted internal diagnostic evidence only, not
current-source parity, public API parity, an unadapted official pass, or a Core
ledger promotion.

Current diagnostic evidence beyond the immutable Core ledger: a pinned,
browser-free `browsertype-connect.spec.ts:116` HTTP 401 slice passes its two
original assertions in both candidate and Playwright oracle lanes (2/2 each).
Both lanes adapt only the fixture's `localhost` URL to this Mac's full
Tailscale MagicDNS host; therefore this is
`official-derived-fixture-host-adapted-internal`, not an unchanged official
run or public API parity. Independent v5 sidecars and negative verifier tests
pass under `.work/official-conformance/results/core-server-connect-http-401-strict-v5/`.
The fixture receipt records two real HTTP 401 sends per lane but has no
per-spec ID on those sends. No Core ledger rows were promoted. A separate
Core Close-error v4 run also passed original assertions after host adaptation,
but its candidate server observed abnormal Close `1006` instead of the
oracle's `4123 / Oh my!`; that frozen v4 evidence remains lifecycle RED.
Its v4 verifier also wrongly required distinct workers and cannot certify
either v4 sidecar; the raw receipts and hashes remain preserved.
After flushing the queued WebSocket Close reply, a separate v5 strict successor
passes both original line-130 assertions in each candidate/oracle lane and
independently verifies two ordered server-side upgrade, message, and Close
`4123 / Oh my!` triplets per lane. Both tests reused worker 0 and port 8907.
The receipt has no spec ID or timestamp for each triplet, so the per-spec
association is an order-based inference, not direct wire attribution.
The v5 candidate native artifact SHA-256 is
`39a8ecfdac8db8d0cf1eb0e100c3c17aaf59fdf9c4dbb4f9739602202f0843f1`.
It remains fixture-host-adapted internal diagnostic evidence, not public API
parity, unadapted official success, or a ledger promotion.

Latest Core diagnostic checkpoint: an opt-in Rust-backed Node DNS/WebSocket
error path now passes two unchanged, browser-free official
`browsertype-connect.spec.ts:122` assertions 2/2 in both frozen Pwright
candidate and Playwright 1.58.0 oracle lanes. The independently verified
[GREEN promotion receipt](../.work/official-conformance/outcomes/core-server-connect-ws-error-strict-green-paired-v2/promotion-receipt.json)
has SHA-256 `e5921619fdfd88dd14ec71ab124ae952f34edc6678e7b914e46692ac0c754da4`.
It changes exactly two rows per 48,477-row lane from the preserved
[RED predecessor](../.work/official-conformance/outcomes/core-server-connect-ws-error-strict-red-paired-v1/promotion-receipt.json)
and leaves the other 48,475 byte-identical. Candidate: **4,543 expected /
53 unexpected / 11 skipped / 43,870 not run**; oracle: **4,290 expected /
eight unexpected / ten skipped / 44,169 not run**. The candidate artifact
SHA-256 is `5f083ad9425b7b2c253d5be311a338ef712928ea8357aab5035ed67d2d10b3a2`.
These tests use an upstream-internal fixture and are diagnostic-only, not
public API, BrowserServer, successful remote-Browser, or browser-execution
evidence. Of the candidate's 4,543 `expected` Core rows, 2,610 are
`upstream-private-source-only` and did not load Pwright; the ledger total is
not a count of Pwright runtime-compatible cases.

ABI123 closed-endpoint checkpoint: Python `BrowserType.connect` now maps a
typed WebSocket connection failure to the upstream-compatible `WebSocket error: `
prefix. The opt-in release wheel SHA-256 is
`3b86f0b56c6dbc2654a3326cd91b62aed0ed2b8d22537e69476226565bfed6d1`;
fresh CPython 3.9.6 and 3.12.14 each report ABI123 with the selected feature,
pass 290/290 contract/connect-facade unit tests and `uv pip check`. Six pinned
official async/sync closed-server cases pass 6/6 in each isolated candidate
and oracle lane, with 12 independently verified single-case sidecars under
`.work/official-conformance/results/python-connect-refused-strict-v1/`.
The original server-launch fixture was replaced by an explicitly versioned
closed-endpoint fixture, so these results prove only the refused-connection
public error behavior; they do not prove BrowserServer startup or a successful
remote connection. See the [strict evidence scope](knowledge/pwright-python-connect-refused-strict.md).
The immutable [paired successor](../.work/official-conformance/outcomes/python-connect-refused-strict-paired-v1/promotion-receipt.json)
(receipt SHA-256 `4979cdc982cf716e5124bf00b972594c27a65fc26104c193ed12957cf5d4c060`)
promotes exactly six rows per lane and independently verifies again. Candidate
is **1,542 passed / 102 failed / three errors / 11 skipped / 4,868 not run**
(ledger SHA-256 `3095771a0928ecc076ecdec8eee5d2946d70de9dbd1ea15e00f61ad7fa79e836`);
oracle is **1,309 passed / four failed / 11 skipped / 5,202 not run**
(ledger SHA-256 `11afbc5f482a3d9cefa36c59598f1771903b74f93749dcd11e8b5b178a7a6208`). The
other 6,520 rows in each lane are byte-identical to the predecessor. The
canonical row schema does not carry a fixture-variant field: interpret these
six `passed` rows with the receipt's explicit fixture-only scope, not as full
server parity. A new browser-free
Playwright Server client core has 24 passing release unit tests and release
Clippy `-D warnings`, but does not yet create a remote Browser or add a default
runtime dependency. Earlier ABI122 verification below remains a separate
historical early-close checkpoint.
An additional default-off `pwright-playwright-server-session` crate now
validates the full `Root.initialize` GUID graph before returning a
connection-owned Browser lease. The earlier default and `live-tokio`
fake-duplex checkpoint passed 14/14 each with strict Clippy; the current
19/19 base and 32/32 optional-supervisor source-only checkpoints are described
above. It has no language binding, Context/Page adapter, or real-server E2E
proof; this is not an additional official test pass.
The optional live WebSocket connector now selects pinned local forks of both
Tungstenite crates, isolating upstream TRACE handshake/payload/Close logging.
Its release transport suite passes 33/33, including real WS TRACE sentinel and
native TLS certificate-validation tests; default-off closure, strict Clippy
and workspace formatting pass. The [security boundary](knowledge/pwright-ws-logging-security-design.md)
records remaining release gaps: forks are unpublished, full connector WSS/Rustls
E2E is unverified, and unrelated CDP stock dependencies can still log their
own traffic. No production authenticated Server support is claimed. The
frozen ABI123 wheel predates this source mitigation.
The same opt-in transport source now classifies rejected WebSocket upgrades
with a status and an 8 KiB-capped HTTP body; an in-memory 401 handshake and
an explicitly selected full-MagicDNS/Tailscale-bound live connector test pass,
as do release tests and strict Clippy. The selected Node diagnostic profile
exposes that failure path in the fixture-adapted Core HTTP 401 v5 capture
above; it does not return a remote Browser or add a ledger row.
Subsequent selected-fork source hardening reads declared-length rejected
HTTP bodies across split TCP reads and caps both raw and UTF-8 output at
8 KiB; no-length bodies remain same-read-only. The frozen HTTP v5 capture
predates this change; focused release/TRACE/TLS and live Tailscale 401 tests
verify the current source, not a new official v5 artifact.

Preceding Python strict Playwright Server close checkpoint: the unchanged official
async `BrowserType.connect` custom-WebSocket-close assertions pass **3/3** in
each isolated oracle and ABI122 candidate lane across Chromium, Firefox and
WebKit. Six independently verified reports under
`.work/official-conformance/results/python-connect-close-post-fix-strict-v2/`
prove the real public call, inbound `Root.initialize` frame and fixture Close
`4123 / Oh my!`. The immutable 6,526-row paired [receipt](../.work/official-conformance/outcomes/python-connect-close-post-fix-strict-paired-v2/promotion-receipt.json)
has SHA-256 `03b22f0bc8e986217e9c6d9c301139049a0d268795ba15197ed5f4716bb52b72`.
Candidate is **1,536 passed / 102 failed / three errors / 11 skipped /
4,874 not run** (ledger SHA-256
`1acbae02e2049b96d2556a3093b8a65cd43b12ac00f53745ee6f58c764990afb`);
oracle remains **1,303 passed / four failed / 11 skipped / 5,208 not run**
(byte-identical ledger SHA-256 `7d044a471c9ee530de390d676d50478d28c3df43a4f106b60528809c7a114ec7`).
Only three candidate rows changed. The opt-in ABI122 wheel SHA-256 is
`2ad72324788ed7ae24eeb6f265e035f64a654e75ebc342ddb2537730f659bc1d`;
fresh CPython 3.9.6 and 3.12.14 installs each pass 67/67 focused browserless
tests and `uv pip check`. This failure path is not a usable remote Browser or
full Playwright Server client.

The preceding Python strict dummy-engine checkpoint: the unchanged official
async/sync Firefox/WebKit launch-failure assertions pass **4/4** in each
isolated oracle/candidate lane. The eight strict reports under
`.work/official-conformance/results/python-dummy-engine-post-fix-strict-v2/`
verify one real dummy Node exit 1 and the correct exception class per run.
The immutable 6,526-row paired [receipt](../.work/official-conformance/outcomes/python-dummy-engine-post-fix-strict-paired-v2/promotion-receipt.json)
has SHA-256 `009556238f14410c229cad349760bef962183fca33cf82569dc6a5c74d70a5bf`.
Candidate is **1,533 passed / 105 failed / three errors / 11 skipped /
4,874 not run** (ledger SHA-256
`5c88ae0e2cad7a0b5c10228e1435d220fb6263311b5d85cbd0f4c323e7eb32a3`);
oracle is **1,303 passed / four failed / 11 skipped / 5,208 not run**
(ledger SHA-256 `7d044a471c9ee530de390d676d50478d28c3df43a4f106b60528809c7a114ec7`).
Only four target rows changed per lane; the other 6,522 are byte-identical
to the preserved red predecessor. The opt-in ABI121 wheel SHA-256 is
`904db7f0af2082451edfbe589afcedd9fdc77295ce627cbc391d9e60b9780367`.
Fresh CPython 3.9.6 and 3.12.14 installs each pass 40/40 focused
browserless tests and `uv pip check`.
This covers bounded explicit-executable early exit only; full Firefox/WebKit
support and complete Python conformance remain open.

Latest browser-free Core CDP error-slice successor: the unchanged official
`connect-over-cdp.spec.ts` assertions at lines 399, 411 and 572 pass **3/3**
in both the independent Playwright 1.58.0 oracle and the fixed Pwright
candidate. The candidate ran from a frozen package snapshot, not the evolving
checkout. The strict v3 [candidate sidecar](../.work/official-conformance/results/core-cdp-error-slice-strict-v3/candidate-v1.strict.json)
and [oracle sidecar](../.work/official-conformance/results/core-cdp-error-slice-strict-v3/oracle-v1.strict.json)
pass independent verification. A separate [snapshot seal](../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v3-seal-v1/attestation.json)
(SHA-256 `87d852a37096b7b287c7c9c3090f4d46704deb8ffd8e2de93eb70602f4143841`)
also replays the pair without depending on the mutable live addon path. The
immutable 48,477-row [paired successor](../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v3/promotion-receipt.json)
has receipt SHA-256 `1f65ad5e2d26ec874fa7c386439c2ef28be58335233cfd5bf4d1f397356d555d`.
Candidate is **4,541 expected / 53 unexpected / 11 skipped / 43,872 not run**
(ledger SHA-256 `400f1e29f84da3f83076bd97d89ebcf76f20ae4cec48e5c518337fb1f7c56736`);
oracle is **4,288 expected / eight unexpected / ten skipped / 44,171 not run**
(ledger SHA-256 `4c4a397cd098fd28e42b4e579217d808a034b8657a21e09228a1a7fd9dec844b`).
Exactly three target rows change per lane and the other 48,474 remain
byte-identical to v2. The source classification remains
`implementation-internal`, with `publicApiParityEvidence=false`; this does
not establish complete Core or public API conformance. The opt-in release
artifact is `packages/pwright/native/official-conformance-page-cdp-errors/pwright.node`
(SHA-256 `0d7dcb931c42d5e912b594ec20fd12e202cc76590c7cab93187fe6829388f5c6`).
The selected/default browserless Node suites pass 791/791 each, but no
browser-backed test ran in this successor.

Historical red v2 Core CDP error-slice checkpoint: three unchanged official
`connect-over-cdp.spec.ts` assertions at lines 399, 411 and 572 genuinely
fail in the candidate and pass in the independent Playwright 1.58.0 oracle.
The strict v2 [candidate sidecar](../.work/official-conformance/results/core-cdp-error-slice-strict-v2/candidate-v1.strict.json)
and [oracle sidecar](../.work/official-conformance/results/core-cdp-error-slice-strict-v2/oracle-v1.strict.json)
were independently verified. The immutable 48,477-row successor contains the
[candidate ledger](../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v2/candidate.jsonl)
and [oracle ledger](../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v2/oracle.jsonl):
candidate **4,538 expected / 56 unexpected / 11 skipped / 43,872 not run**
(SHA-256 `c6304c593856a8a94b24ae173ea385609016139989dd079fc4ef4878f811facd`),
oracle **4,288 expected / eight unexpected / ten skipped / 44,171 not run**
(SHA-256 `6478b726429f40bec7d8c22f943442c57345f246070f2c384386ba33cff376d2`).
Only three previously `not-run` rows change per lane; all other 48,474 rows
match the UA v2 predecessor byte-for-byte. The [promotion receipt](../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v2/promotion-receipt.json)
has SHA-256 `92c6720100cfbecdef6ad36e3bd8fdaf3f327251b4200f83dafeb0ef893e4cb4`.
It preserves `implementation-internal` classifications and records
`publicApiParityEvidence=false`; this is not full Core conformance.
Because the original verifier compares historical candidate provenance with
the evolving live package source, its read-only replay now rejects source
drift. A separate [capture-time snapshot seal](../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v2-seal-v1/attestation.json)
(SHA-256 `65e96b04c6e1cc0edcad15c63dd4ae67ecb4fca9441716a390de52016fef6d7b`)
restores independent verification of the unchanged v2 evidence. The frozen
package source hashes to the original provenance digest
`041a4d60f4901b5f89ffbff492fa13c33e41f6c1ee695c1c81d120e6dba4f03a`;
the [attestor](../scripts/attest_playwright_core_cdp_error_slice_strict_v2_snapshot.py)
and four negative controls pass. The post-fix v3 run above executes a frozen
candidate package snapshot.

Previous browser-free Core CDP default-User-Agent checkpoint: the unchanged
official `connect-over-cdp.spec.ts:299` assertion passes 1/1 in both candidate
and independent Playwright 1.58.0 oracle. Only its fixture URL host is adapted
to the same-port full Tailscale MagicDNS name; the private-helper import keeps
the row `implementation-internal`, and the strict receipt records
`publicApiParityEvidence=false`. The immutable 48,477-row successor is
`.work/official-conformance/outcomes/core-cdp-default-user-agent-strict-paired-v2/`:
candidate **4,538 expected / 53 unexpected / 11 skipped / 43,875 not run**
(SHA-256 `b69852a0f582958f563cecdf055bda37373a3e222378fb4efdb1cea98ac79824`),
oracle **4,285 expected / eight unexpected / ten skipped / 44,174 not run**
(SHA-256 `4198bf312aea62aada3e5f99f0eeb2daca13c980657d01fb81c1141a915b08a8`).
The original UA verifier now rejects the evolved live package; a separate
[UA snapshot seal](../.work/official-conformance/outcomes/core-cdp-default-user-agent-strict-paired-v2-seal-v1/attestation.json)
(SHA-256 `e19b9a8821981cb6f9179de210a43696d34b7c2cf3ccced9ee381a5574370cff`)
reverified its unchanged official sidecars and ledgers against the same
capture-time source snapshot used by the newer error-slice seal.
Only the candidate target changes from `unexpected` to `expected`; the oracle
target was already `expected`, and other rows are preserved. The selected
`official-conformance-page-cdp-default-user-agent` release addon is at
`packages/pwright/native/official-conformance-page-cdp-default-user-agent/pwright.node`
(SHA-256 `92f039f5530075841ca7d59ff523c1f6b7a5ec4884a29ff20042ef3c95826fb2`).
Focused Rust tests pass 7/7, real N-API boundary tests 4/4, selected and
default browserless Node suites 788/788 each, strict TypeScript and release
checks pass, and the normal Cargo package closure adds zero names to the
prior Page-plus-header composition (419 versus 419). The UA strict
verifier/promoter tests pass 21/21 and are in CI with a portable upstream
fixture; the promoter passes Ruff lint. Its sealed source still fails manual
`ruff format --check` on one split SHA literal, so CI's format list omits
that promoter pending versioned recapture. This does not prove full Core
conformance.

ABI121 browser-free official CDP WebSocket checkpoint: the pinned Python
candidate and oracle each pass the unchanged custom-header and custom-close
cases (2/2 per lane) on the full 109-capability ABI121 wheel at
`.work/official-conformance/builds/python-abi121-ws-AE870c/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl`
(SHA-256 `20033ac820480165caf4c420961e22f0fd8c4c29732d2fbcde582c05adabe46e`).
Fresh CPython 3.9.6 and 3.12.14 installs each pass 578/578 browserless
units and `uv pip check`. The independent standalone Node
`cdp-handshake-headers` release artifact at
`packages/pwright/native/cdp-handshake-headers/pwright.node` has SHA-256
`82b11e24df70eec254aaa4e751dfa978422265a9ae517932eb89c781160fabc9`
and passes native-load 1/1, facade 10/10 and real N-API boundary 3/3;
it is not a full Core Page composition or official browser result.
The full 39-feature Page-plus-header release artifact at
`packages/pwright/native/official-conformance-page-cdp-handshake-headers/pwright.node`
has SHA-256 `dfe9607df2c7cfa16d304338e88eb2fa01302b4736e4116c0c67b655a3482769`.
Its selected and default browserless Node suites each pass 783/783; native
load passes 1/1, real N-API header boundary 3/3, strict TypeScript passes,
and its normal Cargo closure adds zero packages over the 338-package Page base.
This does not grant official Core ledger credit.
Separately, two unchanged official Core CDP-header tests ran browser-free with
only the fixture URL host adapted to the same-port Tailscale MagicDNS name.
The explicit WS/HTTP header test passes in both lanes; at this earlier
checkpoint, the default WebSocket `User-Agent` test failed in candidate
(`undefined`) and passed in oracle.
Strict sealed evidence was atomically promoted to
`.work/official-conformance/outcomes/core-cdp-handshake-headers-strict-paired-v1/`:
candidate **4,532 expected / 57 unexpected / 11 skipped / 43,877 not run**
(SHA-256 `131e637c7061f7587b0436bd7bc803f1c0419b1a76d714c5f84b6bbcaa3cd823`),
oracle **4,280 expected / 11 unexpected / ten skipped / 44,176 not run**
(SHA-256 `adf29c0f3b4da2f3ae011d42fa3c97491ee8c65efa4e28f00abe4595dcd22889`).
Only two rows changed per lane; the existing `implementation-internal`
classification and 2,610 source-only count were preserved.
Two further unchanged official Core CDP URL-query tests then passed 2/2 in
both lanes without a browser, verifying query retention and `/json/version/`
path joining. Their separate sealed evidence was promoted onto the header
bundle at `.work/official-conformance/outcomes/core-cdp-url-query-strict-paired-v1/`.
At that URL-query checkpoint, the candidate was **4,534 expected / 57
unexpected / 11 skipped / 43,875
not run** (SHA-256 `8d4d6a5804657f9757b535c78caf293f41c059ee71375e46fc4413a46245df09`);
the oracle was **4,282 expected / 11 unexpected / ten skipped / 44,174
not run** (SHA-256 `58503837e2cc5fedd1e4055bc4703ec65d7d7cdc26475ad02ba3b2b871d75fb5`).
Only the two URL rows changed per lane; the other 48,475 rows, including
the default-User-Agent failure, are byte-identical to the header predecessor.
The first Core header and Python connect evidence helpers are not yet in the
CI Ruff file list. A manual check with the Python package configuration finds
style issues in those hash-pinned scripts; do not reformat them in place and
invalidate the sealed evidence. Version, recapture, and add the quality gate.
The later Core URL-query helpers passed pinned Ruff check/format before capture.
A still later browser-free official `BrowserType.executablePath()` pair passed
3/3 in both candidate and oracle after a versioned test-only adapter supplied
the official fixture's private `_playwright` backpointer. The public candidate
path remains computed by the selected Rust browser-registry profile. The
immutable successor is
`.work/official-conformance/outcomes/core-browser-type-owner-strict-paired-v2/`:
candidate **4,537 expected / 54 unexpected / 11 skipped / 43,875 not run**
(SHA-256 `40625a1de093799e3ad19b0059721c95fd84f3fa04654f373d273ff80abd2e71`),
oracle **4,285 expected / eight unexpected / ten skipped / 44,174 not run**
(SHA-256 `a03b9b2f9cf1ec06e58db2ce08bad2c770b8eb8620f73ee1aec82d0415f10fd5`).
Only the three prior unexpected rows changed in each lane; the Core default
WebSocket User-Agent failure then remained. The Core runner/verifier/promoter pass
Ruff, and their eleven evidence/integrity tests pass. The original source-ledger
classification bytes remain unchanged, but the corrected invocation audit
labels these three cases `implementation-internal`/`private-client-access`,
not public API parity. The separate immutable seal at
`.work/official-conformance/outcomes/core-browser-type-owner-strict-paired-v2-seal-v1/attestation.json`
pins the strict verifier and promoter source hashes and records
`publicApiParityEvidence=false` without rewriting the outcome ledger.
That separate browser-owner seal-v1 attestor has manual Ruff `EXE001`/`B023`
debt and an immutable self-hash; fix it only in a versioned reseal.
Four ABI121 strict sidecars at
`.work/official-conformance/outcomes/ws-strict-{candidate,oracle}-abi121-{header,close}-v1.provenance.json`
independently verify the pinned-source runs and the sole same-port
loopback-to-full-MagicDNS fixture adaptation. The immutable paired successor
at `.work/official-conformance/outcomes/python-cdp-ws-abi121-strict-paired-v1/`
passed read-only `--verify-existing`; only the two target Chromium rows moved
from `not-run` to `passed` in each 6,526-row ledger. At that predecessor, candidate was
**1,529 passed / 102 failed / three errors / 11 skipped / 4,881 not run**
(SHA-256 `799b0e0da339a22ec23161785e4d81d99dc79da04c0c2f0e1ea6ff5276f53c11`),
and oracle was **1,296 passed / four failed / 11 skipped / 5,215 not run**
(SHA-256 `0dfaee1ccdbbdbc3483fdc00cee3533fc8dbf49a7ec58218f399164c4a51c675`).
ABI119 candidate 0/2 and ABI120 candidate header timeout / close pass remain
historical, unpromoted evidence. No browser-backed Python or official Core
rerun follows from this Python shard; the complete official suites remain
open, while the newer Core UA successor above resolves only its one
implementation-internal row. See
`upstream-playwright-conformance.md`.
Six historical red ABI121 official Python sidecars cover the unchanged async
`BrowserType.connect` custom-close test on Chromium, Firefox and WebKit:
candidate fails 3/3 with the real missing method, oracle passes 3/3.
The sidecars independently verify, and a strict promoter published an
immutable 6,526-row paired successor at
`.work/official-conformance/outcomes/python-connect-close-abi121-strict-paired-v1/`.
Candidate was **1,529 passed / 105 failed / three errors / 11 skipped /
4,878 not run** (SHA-256
`cc7329662a1ef479c250a3615b55e057cbba8e8454aa798998ded5c5102aa26f`);
oracle was **1,299 passed / four failed / 11 skipped / 5,212 not run**
(SHA-256 `7ec3fbbf7efb8c6564b19bedf09caeab4e2c16e7b2c2c26be678169db1ecb105`).
Only those three target rows changed; all other 6,523 rows match the prior
bundle byte-for-byte. The preceding CDP WebSocket counts above are historical.
Playwright Server `connect` is not CDP `connect_over_cdp`.

The historical red ABI121 Python dummy-engine predecessor is
`.work/official-conformance/outcomes/python-dummy-engine-abi121-strict-paired-v1/`.
Eight independently verified `*-v2.provenance.json` sidecars under
`.work/official-conformance/results/python-dummy-engine-abi121-strict-v1/`
bind the four unchanged async/sync Firefox/WebKit dummy-executable launch
cases in both lanes. Oracle passes **4/4** with a real pinned Node asset exit;
candidate passes **0/4**, failing with `UnsupportedError` before spawn. No
failure was recast as a pass or skip, and no Chrome was used. Only those four
previously `not-run` rows changed per lane. The 6,526-row candidate ledger
is now **1,529 passed / 109 failed / three errors / 11 skipped / 4,874 not
run** (SHA-256 `dd0609e85a704f97c55762587126c0790d80a444bc116cc9f7e3f55c8d0efb9d`);
the oracle is **1,303 passed / four failed / 11 skipped / 5,208 not run**
(SHA-256 `f15b5292a64e27607bc632074fcac26457f879decf569ff8eb616435588e9b95`).
The paired bundle's atomic receipt and read-only verification pass; the
preceding connect-close ledger remains intact.
Manual Ruff on the five new dummy-engine runner/verifier/promoter and unit-test
files collectively report `EXE001`/`I001`/`F401`. Hash-pinned capture sources must be
versioned and recaptured before fixing those files; this is not a green lint
gate for the shard.

Earlier strict official Python checkpoint: the pinned ABI119 wheel and independent
Playwright oracle each passed the unchanged async/sync Chromium dummy-executable
launch-failure tests (four 1/1 official runs). A source-pinned guard records
the public launch call and the real Node asset's exit code 1 under an
exec/network-denying sandbox. Four independently verified sealed reports were
promoted together, after a second pre-publication evidence check, to an
immutable paired 6,526-row bundle at
`.work/official-conformance/outcomes/python-dummy-node-abi119-strict-paired-v1/`.
Only the two target Chromium rows changed in each ledger; candidate was then
**1,527 passed / 102 failed / three errors / 11 skipped / 4,883 not run**
(SHA-256 `9c057de3aff4b3c2141d150999e12adcbd162ba1f7876cdc6d2a949fddda5f80`),
and oracle was then **1,294 passed / four failed / 11 skipped / 5,217 not run**
(SHA-256 `a95e15e4cfb4708a6e8a98832fea7e4e2759ca8ad034f13a3bfc8933f21913f1`).
This proves the installed ABI119 wheel for those two tests, not the current
Rust source or the rest of the official suite. The Core strict ledgers are
unchanged.

Earlier ABI119 packaging checkpoint: the full 106-capability wheel at
`.work/official-conformance/builds/python-abi119-cdp-full-iWlHU8/`
(SHA-256 `86d1ff028301b4bd42f9f2a3a39b8cef172217e8a9600e1fcf0517c7b38b1254`)
imports its own ABI119 extension from fresh CPython 3.9.6 and 3.12.14
environments. Each passes 571/571 installed-wheel browserless units and
`uv pip check`; whole-package Ruff check/format and strict Pyright pass.
The earlier Node `official-conformance-page-cdp-session` composition built a
release native artifact (SHA-256
`958155e45ca6162e05f2d6f13d4a13f6d76953af7658d8b1677bc6ce8c68ec9a`),
passes native-load smoke, 763/763 browserless tests, strict TypeScript, and
the zero-added-package dependency check at that checkpoint. The current
source additionally implements an opt-in Browser-root CDP session, including
filtered Page discovery and cleanup after a post-create event-flush failure,
but that earlier artifact does not include the feature. Selected release
host source and test compilation pass. The current normal Node browserless
list passes 773/773 with strict TypeScript. The full Node CDP dependency
script passes on the current source with zero added normal packages and both
feature-off/feature-on release checks; a combined artifact must be rebuilt.
Focused CDP host release tests pass 32/32. The separate topology release test
target remains unrun after compilation raised swap from about 777 MiB to
1.4 GiB and was stopped; its test-only ownership error is corrected in
source but has not been recompiled. Rebuilt native-load remains open. None of these
checks was an official browser pass; the CDP endpoint was unresponsive and
strict ledgers were unchanged at that checkpoint.
Four currently unexpected Core `$eval`/`$$eval` `*`-capture IDs now have a
typed Rust source implementation and focused tests. A source audit found and
fixed premature capture materialization: later filters and descendants now
apply to the uncaptured suffix before selecting the captured ancestor. The
selected release host `cargo check` and `cargo check --tests` pass with
Browser CDP, handles, and selector registration; the selector module passes
89/89 release tests. A rebuilt native artifact and official browser reruns
remain open. Ledger
counts do not change. The earlier Browser-root topology release test target
exceeded safe memory headroom and was interrupted before an outcome. Node's
normal browserless suite now passes 773/773 on the current facade source.
At that ABI119 checkpoint, fresh isolated candidate and oracle Python 3.12.14
environments collected all **6,526/6,526** pinned upstream cases on that wheel; four paired
CDPSession/tap manifests select the same 21 cases and source digests in both
lanes. These are **collect-only** reports, not executed test passes. The
wheel's packaged README metadata predates the post-build 571-test verification;
its packaged Python facade files matched source at that checkpoint. This
historical metadata caveat does not negate the ABI121 transport-wheel proof
above. The Python addon source passes selected `cdp-session` and ABI121
WebSocket transport release checks and base-versus-selected normal
dependency-tree comparisons, but the full feature-by-feature dependency
script remains incomplete under memory pressure.
The pre-dummy-launch Python base ledgers were candidate **1,525 passed / 102 failed /
three errors / 11 skipped / 4,885 not run** and oracle **1,292 passed / four
failed / 11 skipped / 5,219 not run**; the ABI119 dummy-node successor is
reported in the earlier checkpoint above.
The Core counts at the earlier source-only checkpoint were candidate
**4,531 expected / 56 unexpected / 11 skipped / 43,879 not run** and oracle
**4,278 expected / 11 unexpected / ten skipped / 44,178 not run**. The current
paired successor is reported above. Each Core ledger includes 2,610 explicitly
upstream-private source-only rows, not candidate public-runtime passes.

Earlier Python ABI116 touchscreen checkpoint: async/sync Page.tap,
Locator.tap, retained ElementHandle.tap, and Page.touchscreen.tap now forward
to the Rust host behind separate `touchscreen-input` and `element-handle-tap`
features. The repaired full 105-feature wheel at
`.work/official-conformance/builds/python-abi116-touch-final-Zs7Dqb/`
has SHA-256
`2473eae420bc850e5981643636405c84e1fab64b414960cabf474e1725146095`.
Fresh CPython 3.9.6 and 3.12.14 installations each pass 535/535 browserless
units and `uv pip check`; the isolated minimal-feature release Rust addon
suite passes 44/44. Dependency-tree comparison finds exactly three new
touchscreen packages, with no further Pwright package for retained tap.
The full addon dependency-boundary script, strict full-profile release Clippy,
and rustfmt checks pass.
Whole-project strict Pyright reports zero errors; Ruff check and format pass.
The default-off native `cdp-session` owner and exact auxiliary topology hook
pass nine focused release host tests. ABI118 adds a PyO3 delivery lease that
cleans up an unclaimed session through the Python wrapper-registration boundary;
four focused release addon tests pass. The frozen 106-feature wheel at
`.work/official-conformance/builds/python-abi118-cdp-final2-Aod8pr/`
(SHA-256 `4c747a33fd5bcca04bd7eee183cf07263a52f455f5a6f3a5e84c2532e321f3e3`)
passes 547/547 installed-wheel units and `uv pip check` on fresh Python 3.9.6
and 3.12.14. The full dependency script proves the same 108-package base
closure for `cdp-session`; full-profile strict release Clippy passes. A second
build from identical source has byte-identical payloads after normalizing
only CycloneDX serial number/timestamp. The official browser rerun remains
pending, so no conformance ledger row changes.
The attach, event-claim, generation-watch, and cleanup source changes pass
25/25 focused release host tests and 27/27 focused ABI119 Python addon tests;
selected-feature strict Clippy passes. It retains late parent/child attach
responses, rolls unacknowledged CDP events back under the original per-session
limits, parks one watcher for idle events, and compensates an unclaimed native
child even after portable Page retirement. The ABI119 installed-wheel proof is
recorded above; unchanged official browser cases remain unverified. The Node
addon bridge and lazy facade watch also have the native-build proof above,
but no browser result. A new full 12-case Chromium-library
CDPSession Core manifest is source-pinned; browser-free official runner
collection selects exactly 12 tests in one `chromium-library` file. No
browser-backed result exists for this manifest.
The Python async CDPSession facade now schedules awaitable event callbacks,
reports asynchronous failures through the event loop, and bounds/cancels
owned callback tasks. Its focused fake-native suite passes 36/36 and is
included in the installed-wheel suite above; neither is official-browser
evidence. Synchronous CDP callback
reentrancy remains unsupported.
Official Python `test_tap.py` remains at the prior 0/10 candidate versus
10/10 oracle evidence; the dedicated Chrome CDP endpoint has not been
restarted without approval, and no strict-ledger row changed. A separate
source audit of the 26 official Page locator-handler cases identifies native
action/expectation checkpoints and callback acknowledgment as the required
next capability; no handler runtime or browser result is claimed.

Earlier browserless checkpoint: the cumulative 47-feature route addon is
release-built, frozen (native SHA-256
`cbc16676bafca0a1c5a79c9c00bb3413657fa3193348670803c5aaf6dd5b8845`),
and verified by Rust host 629 passed / eight ignored, network-events 25/25,
standalone addon 9/9, strict Clippy, Node 689/689, TypeScript, dependency
boundaries, and native-load checks. Node `Response.headerValue` now has a
bounded native-history-backed implementation. The first official 55-case
rerun was refused *before test execution* because another task had opened
two nonblank pages in the shared dedicated Chrome; no browser result or
strict-ledger row changed. The installed-wheel Python `Page.select_option`
24-async/23-sync reruns are also waiting for an unoccupied Chrome. The newer
immutable 47-feature freeze is
`.work/official-conformance/builds/core-route-large-post-data-candidate-frozen-20261011-v2.2YMq5E/candidate/`
(native SHA-256 `0daec0da334a94e161a73809a243fae336933bb44092ec5d8458f23b0843d5ca`).
It adds same-request POST-body preservation and raises only the opt-in
large-route CDP event/WebSocket caps to 8 MiB; the baseline remains unchanged.
Combined Rust host release tests passed 629 with eight existing ignores,
strict Clippy and rustfmt passed, Node 690/690 passed, and the frozen ABI 1 /
47-feature load passed. At that earlier checkpoint, this artifact had no
official browser result; its later 55-case run is recorded below.
Separately,
25 image-tools upstream-private source-only cases passed **25/25 in each
candidate-labeled and oracle-labeled run** under a distinct v2
`sourceOnlyAdapter`; an independent verifier checks their pinned sources,
42 PNGs, exact IDs, guard receipts and provenance. Exactly those 25 rows were
promoted from `not-run` to `expected` in each strict Core ledger, with
`evidenceScope=upstream-private-source-only` and
`candidateRuntimeRequired=false`. Both run sidecars explicitly record
`candidateRuntimeAttested=false` and `publicApiParityEvidence=false`: the
promotion is **not** Pwright runtime, browser or public API parity evidence.
An additional pinned Core462 MCP-config source-only shard passed **8/8** in
each labeled lane using two private config tests over four projects, with no
browser or Pwright candidate runtime. Exactly those eight rows were promoted
in each strict Core ledger after an exact result-path/digest review fix and
30 focused verifier/summarizer tests. At that Core462 checkpoint the ledgers
contained 2,607 explicitly source-only rows; three later rows bring the
current total to 2,610. See the full conformance record below.

The generic Core and Python result-ledger entrypoints now require current
source-bound provenance and digest-bind the report bytes they parse; this
verification gate does not promote any additional test row.

Current conformance checkpoint: the unchanged official 36-case Python async
Chromium `Page.evaluate` shard passes **36/36** on the pinned full-feature
candidate wheel and **36/36** in the oracle. On the later frozen ABI 108
`Page.select_option` wheel, candidate async passes **23/24** and sync **22/23**
against official oracle **24/24** and **23/23**; both candidate-only failures
involve an input-triggered navigation destroying the CDP execution context.
An opt-in navigation-commit source repair passes 14/14 focused release tests
and its addon feature-chain check, but no new wheel or official browser result
exists: two isolated builds were interrupted under unrelated memory pressure,
and the dedicated CDP endpoint is unresponsive. The strict ledger is unchanged.
At that earlier checkpoint, the source-bound candidate strict ledger was
**1,455 passed / seven failed /
one error / 11 skipped / 5,052 not run** out of 6,526 (SHA-256
`ba230f183d88455d0ad603d08e8375713e45a8b408b1a9acffd7dea2f5fe6665`);
the oracle was **1,181 passed / four failed / 11 skipped / 5,330 not run**
(SHA-256 `eae68b1fd2b2df27b54e6135633241fe82090064cee803128527bf220c32a951`).
Paired Python passes are **1,176**. An opt-in one-link child-Frame
retained-click source repair passes four focused
release fake-CDP tests and backend/host release checks; exact HostSession
integration and an official browser rerun remain unverified, so the Python
strict ledger is unchanged.

The frozen combined Node addon reran the
complete Core Chromium `Page.fill` and `Page.route` files: **44/45** and
**53 passed / one failed / one skipped** on the later 47-feature freeze,
respectively. The first fresh 55-case
route pair has the same source-bound tooling in both lanes; its oracle is
**53 passed / one failed / one skipped**; seven candidate failures advanced
to passes in the first freeze and three more in the 47-feature successor.
A historical paired 60-case `selectors-react` run gave oracle **57 expected /
three skipped** versus candidate **57 unexpected / three skipped**. The later
opt-in React-only candidate now passes **57/57 runnable tests**, with three
upstream skips; its source-bound result was promoted after a full audit. A
fresh same-runner oracle rerun was interrupted when the CDP endpoint stopped
responding and supplied no promotable full-file result. At that historical
checkpoint the Core strict ledgers were candidate **4,508 expected / 14 unexpected / 11 skipped / 43,944
not run** (SHA-256
`0cdb227361a396a0088abe0dcf261738a4067406a7d6a06a75cfd8267f4397a0`)
and oracle **4,213 expected / 11 unexpected / ten skipped / 44,243 not run**
(SHA-256 `35fa34455be9a86f4e65dd90688c402595724a78f66e47accec035d07454689d`).
Those ledgers then explicitly tagged 2,607 upstream-private source-only rows:
18 earlier, 2,556 from Core460, 25 from image-tools Core461, and eight from
MCP config Core462. They do not
exercise the candidate public API and carry `candidateRuntimeRequired=false`.
The earlier route promotion replaced exactly 55 rows per lane, with complete
invocation audits. The subsequent image-tools and MCP config promotions
changed only their 25 and eight source-only rows per lane, respectively.
The later frozen 47-feature candidate then reran the same official 55-case
`Page.route` file: **53 expected / one unexpected / one skipped**, matching
oracle outcome categories on all 55 IDs, while improving three candidate
rows (CORS/header values and >1-MiB POST). The remaining Referer case fails
for a different reason in candidate versus oracle, so category agreement is
not complete behavioral parity. Its 48,477-row audited successor is candidate
**4,451 expected / 71 unexpected / 11 skipped / 43,944 not run** (SHA-256
`d82424206c85dbad97e604c541368cd15212e01474634a5fce2c67f425873e42`).
At the 2026-10-11 checkpoint, an unchanged 32-case Core Chromium
`Page.selectOption` manifest was pinned
(SHA-256 `acc5e1b0ce2d6b3eff4c14a8b7ad650c76091ae457e293039311394231aead99`);
the oracle had collected 32 IDs, but both browser executions were pending. A
2026-10-11 preflight found no foreign nonblank tab in dedicated Chrome. One
task-owned blank tab must remain open or Chrome 155 rejects the oracle CDP
attach before test bodies begin; the successful route pair used that anchor.
That Chrome process stopped answering CDP and was not restarted by this task.
A different Chrome PID later appeared and `chrome-cdp status` became ready.
Before a fresh React oracle run, two other-task XLQY pages opened; the pinned
runner refused them at preflight without executing tests. This task closed
only its own blank anchor. This was a historical no-result state; the v11/v12
paired browser results described at the top supersede it.
The separate official Python sync `Page.select_option` manifest pins 23
Chromium IDs (SHA-256
`6cb985f0b8c4d74df0368f1b6cbe0e21b660f2b25882edf56d3191ecc1261026`);
both lanes later executed them: candidate **22/23** and oracle **23/23**.
The opt-in navigation-commit fill addon is frozen (SHA-256
`9f019c54294bdc8ccc7f33a44796abb5df2bc10f4d7c5458abcca974d7f5c579`)
and passed browserless release/contract/type/boundary gates; its official
45-case browser rerun remains pending and earned no strict-ledger credit.
The new opt-in `select-option-timeout-progress` Rust layers passed focused
release checks (4/4, 8/8, 12/12). The opt-in Python `Page.select_option`
full-feature wheel now builds and loads on Python 3.9 and 3.12, with 436/436
isolated package units per interpreter and 9/9 focused Rust parser/guard
tests. The opt-in React selector addon now builds in release mode and loads
with its expected native marker; ten browserless helper tests, including
parent-first and shadow-first ordering plus trailing attribute whitespace,
and the Node 670/670 package, type, and dependency-boundary suites pass. A
separate opt-in Core
`Page.selectOption` addon also builds and loads with its native marker. The
opt-in `route-fulfill-cors` host repair passes 38/38 release Page.route tests
with the feature enabled and 29/29 with it disabled; its cumulative release
addon also builds, loads, and is frozen independently. A further opt-in
`route-regexp` Rust matcher passes six focused release tests, and its combined
RegExp/CORS/unroute addon builds and loads with exact native markers. These
are browserless checks only. The further opt-in route-history addon now loads
with exact marker and raw-header port; its focused Rust tests, 676 Node tests,
types and boundaries pass. A newer source-only redirect-identity repair passes
63/63 focused Node adapter tests, and a standalone `context-cookies` addon
passes release/native-load and Node 682/682 checks. Their 45-feature
cumulative route addon passed release build and native-load after two
memory-safe aborted attempts; its frozen native SHA-256 is
`f5345728c4e2f193af41cacd2e6897669a57a2ae09aed154796de87813fcf81e`.
The fresh official 55-case browser pair and ledger promotion are detailed in
the full upstream conformance record. The separate opt-in 2 MiB POST repair now handles
Chromium's base64 `postDataEntries` in bounded source code. A source-only
follow-up changed its idle route watcher from an unconditional 12 MiB
reservation to a small admission plus post-delivery byte charge; four new
AAA tests now pass within the standalone addon release suite (9/9), and
strict Clippy passes. A bounded opt-in patch also raises the combined
route-history network-body cap for at most four large requests per Page; its
release tests and cumulative addon build now pass, while the official
line-1039 browser test remains pending. The archived route sidecars had an
older runner hash and were not used for ledger promotion. No new official Python 24-case,
Core 32-case, or React 60-case browser result has been promoted.
Verified report, package and manifest receipts are in
[Python official conformance](python-official-conformance.md) and
[Full upstream ledger](upstream-playwright-conformance.md). Neither complete
official corpus has yet run to completion.

Previous strict checkpoint: the complete 36-case Python async Chromium
`page.evaluate` shard ran as a source-bound pair: candidate **26 passed / ten
failed**, official oracle **36 passed**. The 6,526-row Python candidate ledger
is **1,400 passed / 15 failed / one error / 11 skipped / 5,099 not run**
(SHA-256 `89b9493d3c79982ad621071da51c3cf8d4b6b363ff5dd2d0132888852da718d2`);
oracle is **1,134 passed / four failed / 11 skipped / 5,377 not run**
(SHA-256 `90d6295861ceb2461ebe0ec82796b4520cc7cce9d212faf8a08af6bc620ebc78`).
The ten new candidate-only gaps are eight Python evaluation value codecs and
two error-message semantics. The preceding ABI108/97-feature retained-click
timeout wheel passed two unchanged official Chromium timeout cases; older
oracle full70 already passed both IDs, while fresh retries had failed at CDP
setup on a Chrome-extension service worker. The first complete 55-case
Core `Page.route` candidate run was **30 passed / 24 failed / one skipped**.
A versioned origin-correct rerun including the four-message JS fix is
**36 passed / 18 failed / one skipped** and advances its 48,477-row strict
ledger to **1,801 expected / 30 unexpected / eight skipped / 46,638 not run**
(SHA-256 `b68d9534680e4ceba8ceeaad2916ca41617b561f90cf1f776aa095e7ed881b22`).
The next unchanged 55-ID run selects an opt-in header/URL/redirect native
profile: **40 passed / 14 failed / one skipped**, with source-bound result
SHA-256 `f0c20fdff003f24657c7a9ab906d5f1bdfc364e3f53a2b1d30e726a92f2c0653`.
The newest candidate strict ledger has **1,805 expected / 26 unexpected /
eight skipped / 46,638 not run** (SHA-256
`51a46b47dcca314abf83bb6265f231e24385a95c21bdd80a57ffbfe0a2844174`).
A one-case Core oracle diagnostic first hit the same setup blocker. After a
transient CDP unavailability and `chrome-cdp start`, the full independent
origin-correct 55-case oracle run completed **53 passed / one failed / one
skipped**. The immutable 48,477-row oracle successor now records **1,522
expected / 11 unexpected / seven skipped / 46,937 not run** (SHA-256
`5631c74c9d9d60030319696fbed9f4600430db392cc0a3ae1a29be9ef244fb3f`).
In the latest run, exactly 40 route IDs pass in both lanes, 13 fail only in the candidate, one
fails in both, and one is skipped in both. The extension was not disabled;
this paired slice is not full-corpus conformance. Exact receipts are in the
Python and upstream-conformance evidence documents.
An added Node `Request.allHeaders()` passes 607/607 package tests. Its full55
diagnostic also passes 40, but line 988 newly passes while line 695 encounters
a 65-over-64 network-request capacity limit; it is retained separately and
not merged into the strict route ledger. A new source-pinned 45-case public
Chromium `page-fill.spec.ts` manifest has identical candidate/oracle collected
IDs. After the unrelated tab left the dedicated Chrome profile, the complete
official file ran: candidate **40 passed / five failed**, oracle **45 passed**.
The latest Core strict successors are candidate **1,845 expected / 31
unexpected / eight skipped / 46,593 not run** (SHA-256
`3b1a91ec1ef1c85d838ffc37841cb5a67e891b382f450c7e0dcbc3ec814e3415`)
and oracle **1,567 expected / 11 unexpected / seven skipped / 46,892 not run**
(SHA-256 `2d32b91be9b8110586a24183d9cc8c3169312641482cc7925cdcda89f0ccf705`).
All five Page.fill discrepancies are paired candidate-only failures; the
source-bound report hashes and exact source lines are in
`docs/upstream-playwright-conformance.md`.
Subsequent browserless repairs cover the Node `Page.fill` error/argument
boundary (627/627 package tests) and an opt-in Rust-backed `Page.unroute`
profile (620/620 package tests); neither has official-browser credit yet.
The 60-case source-pinned Core `selectors-react.spec.ts` manifest has been
collected, but not executed. An unrelated nonblank tab currently prevents the
dedicated Chrome conformance runner from starting; it has not been closed.
After freezing the first Core route report, the four duplicate-Route error
messages were aligned with the upstream `!` and four no-second-native-call
regression tests were added. Browserless Node package tests pass 596/596,
types and dependency boundaries pass. The second unchanged 55-ID source-bound
candidate run gives all four official browser credit, paired with oracle
passes in the later complete origin-correct run.

The first full-feature **opt-in** Python retained-click wheel (SHA-256
`79f4e2eaec461dd4dd8f492d0ad9fc6c472825fbb989b6938806cb122e6f67c4`)
native-loads on Python 3.9 and 3.12: each has 422/422 package units with
one documented default-profile-only ABI assertion explicitly deselected;
the parser release tests pass 3/3. It has **no official browser credit**.
After freezing that wheel, the opt-in backend/host source now distinguishes
ordinary DOM-detached ElementHandle click errors from Page/Frame closure and
preserves the official message. Feature-on backend release tests pass 132/132
and focused host release tests 4/4. A separate post-fix full-profile wheel
(SHA-256 `3a820d641bf4ffd41b1fb9386f6203a639207a38b3ee9ac34a83f062c0e86258`)
now native-loads on Python 3.9 and 3.12, with 97 selected features and
422 package units plus four facade tests passing on each (one documented
default-profile-only assertion deselected). Its source-frozen normalized
payload reproduces. Its two unchanged official top-level Chromium
detached-handle cases now pass 2/2 in both candidate and oracle lanes with
verified source-bound receipts. Exactly those two candidate rows are promoted
in a new immutable 6,526-row strict successor (SHA-256
`5015b54bd58c16f0f345af5ae6ef034f6999194031471b1c2e7fbe60941677d6`):
1,322 passed / five failed / 11 skipped / 5,188 not run. The oracle strict
ledger already counted both passes and remains unchanged. See
`docs/retained-handle-click-design.md` and
`docs/python-official-conformance.md`.
The post-repair wheel has since completed the unchanged 70-case Chromium click
file: **65 passed / four failed / one fail-closed report error** with verified
source-bound provenance. An immutable candidate strict successor applies that
report and then reapplies the explicitly qualified, previously paired favicon
fixture variant: **1,372 passed / seven failed / one report error / 11 skipped /
5,135 not run** (SHA-256
`9d6851f599101d90ab531ed9442e1381253e53bd3c65f55ea8fb7712464b44b4`).
Paired candidate/oracle pass evidence rises to **1,093** IDs; the original
favicon 404 remains a shared failure, and the variant is not silently counted
as an original-fixture pass. Candidate-only gaps are child-Frame retained
click, two timeout-detail assertions and a fail-closed duplicate-JUnit
report. A later raw-preserving one-case diagnostic shows its test body hit the
same child-Frame boundary before teardown reported retained native resources;
the complete report remains classified as error. See
`docs/python-official-conformance.md` for exact reports.
An opt-in retained-click source repair now keeps bounded per-invocation
actionability progress and sanitized blocker previews through successful
timeout cancellation. Feature-on backend release tests pass 136/136, default
123/123, host release 643 passed / 14 ignored, strict Clippy and Python-addon
check pass. This statement describes the historical source-only checkpoint;
the new full wheel and official 2/2 run at the top of this file supersede it.

The complete pinned 64-case Core Chromium `Page.goto` file now has fresh
source-bound candidate and oracle reports against the opt-in burst addon:
61/64 and 63/64 respectively have upstream-expected outcomes. Only :471,
:543 and :740 gain new candidate strict credit beyond the already-promoted
:629. The immutable 48,477-row core458 successor has 1,764 expected /
13 unexpected / seven skipped / 46,693 not-run (SHA-256
`81b8dcb18287c2582cac834cef7414835acc8e86a8a6e34fac08512ca1c191f5`).
The remaining candidate-only gaps are `toHaveURL` on JavaScript redirect
(:492) and exact `framedetached` event delivery (:859); the protocol-less
MagicDNS failure (:49) is shared with oracle. No full-corpus credit is
claimed. See `docs/upstream-playwright-conformance.md`.
After this full-file report, the opt-in Node Context/Frame `toHaveURL` bridge
passes 12/12 focused tests, 587/587 package tests, types and dependency
boundaries. It covers the official matcher Page identity, optional fields
and structured received URL, routes URL observation/waiting through Rust, and leaves the
default profile unchanged; :492 has **no new official browser or ledger
credit** until an isolated paired rerun.
A fresh complete 64-case paired rerun now gives :492 an official candidate
pass: candidate **61 expected / three unexpected**, oracle **63 expected /
one unexpected**, with both new sidecars source-bound. The candidate-only
unexpected rows are :217 (network Request Frame absent from topology, not yet
diagnosed as deterministic) and :859 (`framedetached` not delivered by the
frozen addon); both lanes fail the protocol-less MagicDNS :49. Because :217
was previously passing, the older Core strict ledger is not superseded yet.
See `docs/upstream-playwright-conformance.md` for report hashes.
A candidate-only isolated :217 diagnostic then passed, as did :217 in a
second complete source-bound 64-case candidate run. That second run is **62
expected / two unexpected**, versus the paired oracle **63 / one**. The new
48,477-row candidate strict successor changes exactly :492 to expected:
**1,765 expected / 12 unexpected / seven skipped / 46,693 not run** (SHA-256
`5519d9e8bd3456edb6eca231961097c76565b6facc3e9f023342930333e114bb`).
The first run's :217 topology/network error remains a separate unresolved
observation; the frozen addon still cannot deliver :859.
An opt-in exact `framedetached` source path now preserves canonical retired
Frame identity across child-first and sequential topology publications.
Focused release Rust and Node tests pass; reconnect-effect coverage now
passes feature-on/default topology release tests and Node lifecycle tests.
Precommit child admission remains unresolved. No new selected addon or
official :859 credit is claimed. See `docs/node-ffi-compatibility-design.md`.

The exact retained `ElementHandle.click` path is now opt-in and source-wired
through the top-level backend, Rust host, PyO3 and Python async/sync facades.
Four isolated ABI108 façade tests, two focused release host tests and three
release PyO3 option-parser tests pass; this does not test live native dispatch,
iframe geometry or Chrome. The frozen official wheel remains
unchanged; the optional handle path has no official or strict-ledger pass. See
`docs/retained-handle-click-design.md` and
`docs/python-official-conformance.md`.
Review additionally fixed retained `trial=True` dispatching pointer input.
The opt-in host now retains an observed creation-world snapshot and validates
it across retries and before pointer presses; a retained-only input gate now
revalidates under the Page dispatch lock. An owned mouse-press token gives
cancellation a narrow release claim. The current full feature-on backend
release suite passes 132/132; focused retained tests pass 9/9,
default backend tests 123/123, host tests 2/2, owned-press tests 7/7 and
queued-lock world tests 2/2. These are local gates only: modifier rollback,
raw-Mouse overlap, per-step movement, host-level retirement/cancellation
proofs, live Chrome and official cases remain open. See
the retained-click design.

Node `Page.goto` now delegates to the selected Rust main-Frame navigation
executor when available, with the v1 Page route retained otherwise. Focused
Node adapter tests pass 30/30, package tests 575/575, and types and dependency
boundaries pass. A newer source-bound 64-case Core Page.goto report is
summarized above; the earlier opt-in one-row result is below. See
`docs/node-ffi-compatibility-design.md` and
`docs/upstream-playwright-conformance.md`.

An opt-in source repair now accounts for 20 concurrent Frame navigation
waiters separately from one Page.goto command host-operation slot. It retains
the 20-waiter rejection boundary and leaves default capacity unchanged.
The first fresh artifact exposed a lower eight-item topology/lifecycle queue
and failed the unchanged Core Page.goto 20-waiter case **0/1 candidate versus
1/1 oracle**. Those source-bound reports remain frozen. A burst-only bounded
publication fence now lets the 20 waiters and goto share that same eight-item
queue; the default fail-fast path is unchanged. Release burst tests pass
10/10 and default Frame navigation tests 5/5. The rebuilt addon native-loads,
and the same pinned official case passes **1/1 candidate and 1/1 oracle**
with verified source-bound receipts. The immutable 48,477-row Core candidate
successor changes only that row to expected: **1,761 expected / 16 unexpected /
seven skipped / 46,693 not run**, SHA-256
`07832667f3bf0f85c60e9685394992cae42ecceaa9fac47e9a8038cf8529bed3`.
The full Core Page.goto file and measured runtime memory remain open.

The shared 404-console fixture failure now has a versioned, case-scoped
favicon plugin, exact one-case manifest and companion source-bound receipt
outside the historical runtime hash. Eight offline tests and Ruff pass.
After unrelated Chrome tabs were externally cleared, the unchanged official
assertion passed 1/1 in both candidate and oracle lanes under this explicit
fixture variant. The strict successors update exactly this one row, while
historical original-fixture reports remain unchanged. See
`docs/python-official-conformance.md`.

Earlier Python strict successors (6,526 rows): candidate **1,372 passed /
seven failed / one report error / 11 skipped / 5,135 not run** (SHA-256
`9d6851f599101d90ab531ed9442e1381253e53bd3c65f55ea8fb7712464b44b4`);
oracle **1,098 passed / four failed / 11 skipped / 5,413 not run** (SHA-256
`de156ebcf865eb589ad458eaaaf43c9fa0fae87aed9fe3a7543f7329d1e1363b`).
This includes **1,093 paired passes**, one explicitly fixture-variant-qualified.
It is not full-corpus execution.

Prior ABI108 Python official checkpoint: the frozen reload/click v2 wheel
(SHA-256 `862c7074e3bbd6a9782b11af0e1ff66f9ffd84c13e0a40b1490b2858d48fa027`)
passes the unchanged Context init-script case 1/1 and five previously timed-out
Chromium click cases 5/5 in both candidate and oracle lanes. Four schema-5
sidecars verify source-bound. Its immutable six-case-only 6,526-row strict
successors contain candidate **1,319 passed / five failed / 11 skipped /
5,191 not run** and oracle **1,097 / five / 11 / 5,413**; exact hashes and
audits are in `docs/python-official-conformance.md`. A separate full click-file
diagnostic records candidate 60 pass / nine fail / one fail-closed report error
and oracle 69 pass / one shared 404-console failure. Neither 70-case result is
promoted into those strict successors. The remaining handle cases require an
exact retained-object pointer action; the source-only path carries
no official pass credit. At that prior source checkpoint, the opt-in
`retained-element-click` locator backend leaf preserved the original
object ID and passed 131/131 feature-on release tests plus 123/123 default
tests. The current guarded feature-on result is 132/132, as noted above.
It deliberately rejects child/OOPIF Frames; subsequent opt-in
host/addon/Python source wiring is described above and in
`docs/retained-handle-click-design.md`.

Prior official Python ABI108 Chromium checkpoint: four paired source-bound
reports cover the eight-case click shard and one Context init-script case.
Candidate click now passes 7/8, with the `ElementHandle.click` row still
failed; oracle passes 8/8. Candidate init script still fails 0/1 with
`['context']` versus `['context', 'page']`; oracle passes 1/1. A separate
non-official probe confirms both init scripts execute, but the Page binding
callback arrives after `reload()` returns. This adds no official pass credit.
The installed ABI108 wheel SHA-256 is
`bcfa4da75e539534701d41c5a9bcca4347e993bf98aab211cd8ad9c32f6d796e`.
This exact full-feature wheel imports on Python 3.9 and 3.12; each interpreter
passes 418 local units with one default-profile-only assertion excluded.
The immutable 6,526-row strict successors now have candidate **1,313 passed /
six failed / 11 skipped / 5,196 not run** (SHA-256
`afbdfb0460759c8639d9e93c37e6ed6c42c11df95dd9bc85fbe76945d2520653`)
and oracle **1,097 passed / five failed / 11 skipped / 5,413 not run**
(SHA-256
`e0c902c39d7b039740feb28f6d694d6b1cb06be90394193a6ae29d996df4c8ca`).
Exactly nine selected rows are refreshed in each lane; three candidate click
failures become passes and oracle counts do not change. Full invocation
audits preserve 6,514 applicable / 12 non-applicable and classification
SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
The complete 70-case click file now has candidate eight passed / one failed /
61 not run and oracle 69 passed / one failed. Exact report and ledger hashes
are in `docs/python-official-conformance.md`; full-corpus execution remains
incomplete.
The installed ABI108 candidate also ran all 70 corrected Chromium click cases
diagnostically: 53 passed / 14 failed / three fail-closed report errors.
Five cases reached the 30-second deadline (two previously unsupported offset
cases, one previously unsupported force case, and two prior fail-closed
reports), and four unrelated
Chrome pages were present at the end. This source-bound full-file report adds
no strict-ledger credit; the paired eight-case results above remain the
candidate's promoted click evidence.
Scoped click-actionability source repairs now address the five timeout
symptoms, but are not in that installed wheel and have no official pass credit.
The exact-retained-handle click source path currently supports only
top-level Frames. Python `ElementHandle.click` still requires an installed
feature-on wheel and correct iframe input coordinates. Its capability boundary and
verification plan are recorded in `docs/retained-handle-click-design.md`;
no official pass credit is claimed there.

At the prior ABI107 v6 checkpoint, paired source-bound
strict-Context/duplicate-registration, seven-case focus, init-script and
eight-case click selections recorded candidate 2/2, 7/7, 0/1, 4/8 versus
independent oracle 2/2, 7/7, 1/1, 8/8. The oracle later completed all 70
corrected click-file Chromium IDs with 69 passes and one 404-console failure;
paired exposed-binding reports passed 1/1 in each lane.
The opt-in 97-feature ABI107 wheel (SHA-256
`bf0edae04560a7945ea08164a86552d924eac75a12d03a91531241765d6ebda2`)
passes 389/389 fresh installed full-profile units; its `Frame.focus` and
strict-Context official rows pass. The ABI107 init-script callback failed
despite `runImmediately`.
Fresh v6 candidate and independent-oracle browserless collections each find
6,501 core and 25 pinned Python 3.10 examples IDs, with all four source-bound
sidecars verified; no execution credit follows.
An independent opt-in Rust/FFI `context-strict-selectors` slice stores a
bounded Context default and wires selected Python Page/Frame APIs; release
Rust tests pass 4/4. It is absent from the default and v5 wheel profiles but
present in v6. Other selector entry points remain incomplete; one official
strict-Context case does not prove general selector parity.
The complete Python async click file is pinned in the corrected v2
manifest (SHA-256 `e3f67855d66abb186fed974dc9b7e614e498acfc58741a557b5146aea373b02c`)
to the clean upstream v1.58.0 source and all 70 Chromium public-API cases;
the v1 manifest is retained only for its historical receipts because it
mislabels 13 primary actions. Both installed-v5 candidate and
independent-oracle browserless collections find exactly 70 and their
provenance verifies source-bound. A separately pinned eight-case Chromium
execution manifest (SHA-256
`bc6c6c05c77e087d1792d9571f0afa2ab953a6069a0e3df205b83526969f4a8d`)
passed candidate 4/8 and oracle 8/8 on ABI107; the new ABI108 reports pass
candidate 7/8 and oracle 8/8. Within the full 70-case click file, candidate
now has eight passed / one failed / 61 not run; oracle has 69 passed / one
failed. Exact hashes are in
`docs/python-official-conformance.md`.
The frozen v6 candidate subsequently ran the complete 70-case Chromium click
file diagnostically: 48 passed / 19 failed / three fail-closed report errors,
with verified source-bound provenance. Four unrelated Chrome pages appeared
near the end, and there is still no uncontested candidate repeat, so
this report does not advance the candidate strict ledger. The Python
`Page.dblclick`, `Locator.click(steps)` and `Page.click(position)` changes were
absent from that v6 wheel; the ABI108 paired reports now verify their three
exact click rows as passing. The bounded early binding-event stage and
exact-session cancellation cleanup pass 34 focused Rust release tests, while
the ABI108 official Context init-script callback still fails.
The next Core Page-click public-only manifest pins 81 exact Chromium cases
from the clean upstream v1.58.0 checkout, with six private-hook rows explicitly
outside the bounded selection. Fixed source/ID/filter checks pass 61 focused
runner/provenance tests and official browserless listing finds 81; there is
no candidate collection, browser result or Core ledger promotion. See
`docs/upstream-playwright-conformance.md` for the manifest identity.
The separate Core Page-route manifest pins all 55 public Chromium cases; both
upstream browserless listings agree on the exact ID digest. There are no
private-hook exclusions, saved collection sidecar, or browser outcomes. Its
service-worker case may require verified trusted HTTPS rather than the
ordinary HTTP MagicDNS fixture; keep it in the selection and record its
eventual paired result explicitly.

Implementation and official-test checkpoint (2026-10-10): the opt-in
`frame-navigation-burst-20` Node native addon builds and passes its real
browserless load probe; Rust release and Node package checks pass. Python's
binding watcher now handles nested asynchronous calls and shields Page/Context
registration transactions from caller cancellation. The ABI107 v4 wheel used
for the strict browser results below
passes all **379/379 installed package units on both Python 3.9 and 3.12**,
including the previously stale native-profile identity assertion. Its
wheel archive SHA-256 is
`7dc721ea74d0b1fc277634738fe7b46f1cf738f69aa44451f8f81da8c161d8bd`.
Its normalized payload matches a second release build.
Independent Browser-shared 64-entry registration and dispatch bounds,
callback-child ownership and self/cross-Browser close regressions pass 22/22.
The v3 wheel exposed a generic duplicate Page-binding session initialization
in official new-Page tests. A narrow exact target/session topology replay
guard has two passing release fake-event regressions and a strict Clippy gate;
the v4 installed wheel passes the unchanged official Python Chromium
`test_goto_should_work` 1/1 with verified source-bound provenance. The v3
candidate failed a pinned forced-colors case that the official oracle passed;
after an initial browser-occupancy refusal, v4 candidate and independent
oracle each pass that unchanged case 1/1 with source-bound provenance. Exactly
one applicable candidate row advances from failed to passed. A later pinned
seven-case Chromium clear-cookie selection passes 7/7 in both installed v4
candidate and independent oracle; its source-bound oracle promotion changes
exactly seven previously unrun rows. At this historical v4 checkpoint, strict
Python candidate was 1,295 passed / nine failed / 11 skipped / 5,211 not run;
oracle was 1,021 passed / four failed / 11 skipped / 5,490 not run.
Candidate-pass/oracle-not-run was 280.
A separate 39-case v4 BrowserContext run was interrupted after 13 isolated
reports when an exposed-function case timed out and dedicated Chrome CDP
became unreachable. It has no aggregate/provenance or strict-ledger credit;
the shared Chrome was not restarted; a separate dedicated Chrome process
later recovered externally.
The complete release `pwright-composition-chromium` library suite now passes
411 executed / zero failed / 16 explicitly ignored after correcting three
stale test-only fixtures or expectations. Its strict release Clippy, whole-
workspace formatting, and diff-whitespace checks pass; none of these local
gates adds official test execution credit.
Combined binding/forced-colors Rust regressions pass 27/27, selector tests
42/42 and the same-profile full host suite 661 passed / 14 explicitly ignored;
the earlier media-only tests pass 9/9. Exact dependency anchors, four addon
release checks and combined/standalone strict Clippy gates pass locally.
Partial-install current-document rollback, native-Future cancellation and
FFI/native-scheduler settlement resource cycles remain open. Two new Rust
FFI admission tests pass but reproduce count/byte starvation, not liveness.
The pinned full Python source collects **6,501 core + 25 examples cases** in
the v4 candidate and independent oracle environments with verified source-bound
provenance; these are browserless collections, **not official executions or
strict-ledger promotions**. No frozen full-corpus parity is claimed.
See `docs/upstream-playwright-conformance.md` and
`docs/python-official-conformance.md` for exact scope.

Newer ABI107 v5 browserless wheel checkpoint: the isolated repaired wheel
under `.work/official-conformance/builds/python-init-scripts-v5.PeiLyY/wheels/`
has SHA-256 `0229c917a9745f9161eb71f7171706788fa000d499660829effcb7730585c510`;
an independently rebuilt wheel has SHA-256
`67f039c9e049a80d102b8915268bd35600b5dbc148b002b1a57b9b41a3057068`.
Their normalized payloads match, including native SHA-256
`5c580c78a239c352d59ad6abcd4a2335fef9bd9b4a`; ABI and 96-feature
profile match v4. Isolated Python 3.9 and 3.12 package suites each pass
380/380. All 81 release dependency/feature checks, addon strict Clippy,
pinned Ruff gates, Python 3.9 runtime cast audit and expanded CI Pyright pass.
Four formerly omitted official-tool test scripts are now in that CI Pyright
path, with 101 focused pytest and 208 CI unittest cases passing. Three bounded
official Chromium selections later ran against this v5 wheel; the strict v4
results above are historical, not the latest ledgers.

Full browserless v5 candidate and independent official-oracle collection now
confirms the same pinned 6,501 core plus 25 examples IDs in each lane, with
four source-bound provenance sidecars and byte-identical 6,526-ID inventories.
This is collection-only evidence and adds no strict-ledger execution credit.

Historical paired clear-cookie evidence: the exact seven-case Chromium manifest
passes 7/7 in both v4 candidate and independent oracle with source-bound
report/provenance pairs. The immutable 6,526-row oracle successor has 1,021
passed / four failed / 11 skipped / 5,490 not run (SHA-256
`accd58457eb6ff6ba7459e7fa1d11bf9f2f3b35956ac836865ed74d2eb94db4d`);
the candidate's seven already-passing rows receive refreshed evidence without
changing its 1,295 / nine / 11 / 5,211 outcomes (successor SHA-256
`7d574909bc125ed0869fe0ec15fd1cc3d15aadc2c314fe2cc47bc9f33814161b`).
Both complete invocation audits pass; no partial 39-case evidence was merged.
The opt-in Node `official-conformance-page-goto-burst-20` composite profile
builds and native-loads browserlessly, with addon SHA-256
`3dfb2ac8698de70f5b2416688b2d15de879251d9d7bf5c5549474ecb9fbd5c83`.
It combines the existing 38-feature official Page union with only the burst
feature; default and prior artifact profiles remain unchanged. Node package
tests pass 565/565 and focused contracts 122/122. This build earns no
official JS/TS browser execution or strict-ledger credit.
An exact one-row Page.goto burst-20 manifest (SHA-256
`023527fbced185707d95a5513b34ae468cca6c65912f40f18773812ead3cab57`)
collects the unchanged upstream case and passes 50 runner/provenance tests.
Its paired browser execution refused two unrelated nonblank XLQY pages in
dedicated Chrome before creating reports; only the task-owned blank tab was
closed. Core strict ledgers therefore remain unchanged.
At its preparation checkpoint, the Python Chromium `emulation_focus` file had
a pinned seven-case manifest (SHA-256
`c6e1449fbbfc55bfc6418d4cb51ee1b6e7bfeb04e123a9d173f72ea6cda708c3`):
all seven applicable rows were `not-run` in both then-current strict ledgers.
Candidate and independent oracle source-bound browserless collections each
found 7/7; their later executed outcomes and promotion are summarized above.
The interrupted Context init-script failure exposed a concrete CDP parameter
gap: new Page initialization must install inherited scripts with
`runImmediately: true`, unlike dynamic registration. The narrow Rust source
change has two fake-CDP payload regressions; focused release tests pass 6/6
under both `init-scripts` and `context-bindings,init-scripts`, and strict
release Clippy passes in both profiles. The frozen v4 wheel predates this
change. Its later v5 official callback rerun still fails in the candidate,
with the failed row retained; the separate earlier exposed-function timeout
is not yet root-caused.
A one-case Python Chromium init-script callback manifest (SHA-256
`fdaecf074e6f043415fe74a93c4ae6f541b76102840aa280fc65c67600932f15`)
collects source-bound 1/1 in both old-v4 candidate and oracle lanes, with
zero execution or promotion. A separate Core Page set-content public manifest
(SHA-256 `47309cfb41f978f065e74a4af687b64b50e1b343699bf9df1c6a4b1703df43f1`)
pins exactly 15 of the unchanged 17 Chromium cases; two `toImpl`-dependent
timeout cases remain explicitly inventoried outside this bounded selection.
Official browserless collection and exact ID-set checks pass 15/15; 55
runner/provenance tests and 18 subtests pass. No Core browser result exists.

Earlier independent Python oracle Page-route checkpoint (2026-10-10): the
existing pinned 18-case async Chromium manifest (SHA-256
`da5aacd59a19ee1852ffe7d7c0918a32cd8115895ce770ee409523297f8cdece`)
ran 18/18 unchanged official cases successfully. The complete source-bound
JUnit/provenance SHA-256 pair is
`fb5b3e0df88f636525808096e7b2755dec0653e2098d99a8fc8db3cdfc11d0b7` /
`eb5e90242f5ff294acd97408044f3c176f4e772b54dd5cbac08a400fa37cfec5`.
The 6,526-row applicability audit marks 17 as public behavior and one
private-helper test as `implementation-internal`/`non-applicable`; that one
already had a pass in the base and receives no new credit. A source-bound
filtered-17 receipt (SHA-256
`fb0f456fbf3cf2915ddce9b59d16632e1d2e3a5d651fa4cbbfda43025afe7303`)
and five hermetic tests verify this exact exclusion and immutable merge.
The strict oracle successor SHA-256
`1dead4360f1813878780093ea80c836f937ba636a51076f818bd5a36110f36d1`
records 1,014 passed / four failed / 11 skipped / 5,497 not run; exactly 17
previously unrun public-API rows changed. Its full invocation audit passes
with unchanged classification SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
At that checkpoint, the candidate ledger remained 1,294 passed / ten failed /
11 skipped / 5,211 not run; candidate-pass/oracle-not-run was 287 rows. Exact artifact
paths and interpretation are in `docs/python-official-conformance.md`.

Earlier Python official checkpoint (2026-10-10): the pinned complete 39-case
BrowserContext candidate rerun with portable current wheel SHA-256
`ca97ca7b2387f69155170ad5044c264e7039eb8df934153b1083505883c977ae`
recorded 32 passed / seven failed, with source-bound JUnit/provenance SHA-256
`98fcd60681bdf26b38a075cc3526e0fc3e89fecfb02b2e89773e4ac0bac7e7d4` /
`d9c82dd0527bb25504d6020c8f93dd67b4cc73de2776fc22617f828de39f3638`.
Only the 35 independently oracle-passing cases were refreshed in the strict
candidate ledger: 29 passed / six failed, seven earlier failures repaired;
four oracle-failing popup rows remain `not-run`. The 6,526-row candidate
successor has 1,294 passed / ten failed / 11 skipped / 5,211 not run, SHA-256
`402c08878c6888f76cab8a7a6f9e4c5ee73d971b232fd1f3288737aa943ac21c`;
receipt SHA-256 is
`c19d8054cb5a386d2a0efc25192f9ef997165f8ee614e05ff48d6d586ca4bcf1`.
The independent unchanged 29-case keyboard oracle then recorded 28 passed /
one upstream skip, with source-bound JUnit/provenance SHA-256
`e5862c438f7faa375b300c0ab2b5018132e2ef001b68d2803025e4e316e572b5` /
`8f4af08d2e3c9cc691ccea2fde90b6d7e5418f106bd7a6f6d69028a58d3050a6`.
Its oracle-only strict successor has 997 passed / four failed / 11 skipped /
5,514 not run, SHA-256
`ab24acc36857ae684080ff0d7eb3300bb3f5c6befba96d93a888427fb1fa86c5`;
receipt SHA-256 is
`fd4bad76676aa205394b1fc95d3aa669eaed3cbc0a3b5e9467dd3513486ff4f0`.
Both complete invocation audits pass. At that historical checkpoint, the
candidate-pass/oracle-not-run gap was 304 rows; the full Python and Core JS/TS
corpora remain unfinished. See
`docs/python-official-conformance.md` for exact artifacts and failure IDs.

Core page-goto full-file candidate rerun (2026-10-10): the unchanged pinned
64-case `chromium-page` file was rerun with the cumulative release addon
(SHA-256 `51b14eec29f4b858e611d7b8ce5b6ded95043f11bbcb0a660f65b860f6ca43dd`).
The source-bound JSON/provenance pair verifies (SHA-256
`236ba4368451a44bf53906e6c756deae1517b9103ea2b66cbbd6de3cf7201621` /
`f661975bb83e56008e173690e5aac0cd6439c30a6a646df6618e3abbc400cca6`):
57 expected / seven unexpected versus the earlier 45/19. Twelve candidate
failures became expected; the shared protocol-less MagicDNS URL failure and
six candidate-only navigation/lifecycle/capacity failures remain explicit.
The immutable 48,477-row candidate successor
`.work/official-conformance/outcomes/playwright-core-candidate-core456-page-goto-chromium-v2.jsonl`
(SHA-256 `1e124b3170c786cfcb4377cd4792ac4346d2de914a71f350ec83770c5cfa1c8f`)
records 1,760 expected / seven skipped / 17 unexpected / 46,693 not run.
Exactly the 64 target rows change from the v1 ledger; all other rows match.
The 48,477-row invocation audit passes. The oracle remains 63/1 for this
file; full Core corpus parity is still open.

Earlier Python local wheel follow-up (2026-10-10): the portable release wheel at
`.work/official-conformance/builds/python-context-gap-click-frame-v1.pO5u1c/repaired.X2v9sv/`
has SHA-256 `ca97ca7b2387f69155170ad5044c264e7039eb8df934153b1083505883c977ae`.
Its isolated installation passes 351/351 unit tests, including additive
modifier-click and Frame script-tag facades. A second build has identical
payload files except generated SBOM UUID/timestamp and dependent `RECORD`;
archive hashes differ. An earlier paired-manifest candidate browser attempt
was refused before execution when two other-task Chrome pages reopened, so
that attempt claimed no new Python strict outcome. The later completed rerun
and strict refresh are recorded at the top of this manifest.

Verified Python BrowserContext subset promotion (2026-10-10): the unchanged
39-case paired async Chromium reports yielded 35 official-oracle passes and
four explicit popup failures. `scripts/promote_playwright_python_verified_subset.py`
rechecks both schema-5 parents, 39 isolated reports, the exact source/manifest
and fixed base-ledger hashes, then derives 35-case reports plus a receipt
(SHA-256 `2858b98b3b8ca5be99496367c1746f54b782a946fe9acc01a26e101e83c4f77b`).
Its 17 hermetic tests pass. The 6,526-row immutable strict successors pass
receipt, selected-row, unrelated-row, summary and invocation audits: oracle
969 passed / ten skipped / four failed / 5,543 not run (SHA-256
`63b2962367ce219c0d174514f21fcd5c3a7b18940b6d829b3e55fbd9ba11cd68`);
candidate 1,287 passed / 11 skipped / 17 failed / 5,211 not run (SHA-256
`a44a39ac38f15afb07bd249f7a9f748cc1c47fd5b1c1d068dce574b5b74c21f6`).
The 13 paired candidate discrepancies are failures, not pass credit. Four
popup rows remain `not-run` in both strict successors. The candidate's local
`Frame.add_script_tag` thin facade has six browserless tests passing but is
absent from this installed ABI110-wheel official run. Full-corpus verification
remains open; details and exact paths are in `docs/python-official-conformance.md`.

Python `BrowserContext.request` oracle catch-up (2026-10-10):
`docs/python-official-conformance.md` records a pinned unchanged-source
Chromium async/sync tranche of 51 + 47 expanded cases. The independent
official oracle passed 98/98 in isolated runs, with both schema-5 sidecars
source-bound against current and archived capture tooling. Exactly these 98
IDs changed from `not-run` to `passed` in its 6,526-row strict ledger: oracle
934 passed / ten skipped / four failed / 5,578 not run (SHA-256
`fc9798778600c0a2ee8bea947a0c9e48dc37f3deb42fdada5516d50f9754bb2b`).
Candidate remains 1,279 passed / 11 skipped / four failed / 5,232 not run;
its earlier 98 pass records were not recounted. A first current-wheel rerun
correctly refused two other tasks' nonblank shared-CDP pages before execution;
they were not closed by this task. After they closed, the isolated current
candidate wheel passed the same 51 async cases; a later empty-CDP window let
the same wheel pass all 47 sync cases. Exactly these 98 existing pass rows
received current-wheel result references without changing outcomes. The
strict candidate successor SHA-256 is
`51cb64812fa743773f78288c6d2b96b5bf1d1c66385b665f1002511280094e83`.
The goal still
requires all 6,526 Python and the full Core JS/TS corpus, a frozen candidate
build, and resolution/accounting of remaining failures.
An opt-in Rust-owned Context offline toggle now has thin Node/Python bindings
and focused local tests, but the two pinned official Python toggle cases
still need paired browser execution. Its bounded Rust owner now tracks dirty
Contexts before CDP sends, compensates ambiguous failures and re-broadcasts
same-value requests after failed rollback or caller cancellation; 18 focused
release tests pass. Cancellation can leave a temporary split until a later
request, and service-worker sessions remain outside the selected topology.

Prepared next full-file conformance selections (2026-10-10):
`packages/pwright-python/tests/official/browsercontext-full-gap-chromium-manifest.json`
pins all 39 still-oracle-`not-run` Chromium Context rows without selecting
for expected success; the exact upstream source check and 39/39 oracle
collection pass. Two null-viewport conflict cases now have Python async/sync
preflight parity and five passing local unit checks; the Node boundary has
three matching adapter checks and one entrypoint check, without official pass
credit. A diagnostic independent oracle execution of the 39-case Context
selection completed 29 passes / ten unhidden failures with verified
source-bound provenance, but a non-task CDP page was present immediately
afterward, so exclusivity throughout is unproved and no strict-ledger rows
were promoted. Six of the ten failures have harness/fixture causes (four
headed Chrome auth classifications and two adapter error-name prefixes);
four popup cases remain unresolved.

The oracle adapter's Browser API-name correction now passes 83 browserless
checks and has a new runtime-tree pin; the earlier 39-case report remains
bound to its archived old runtime and receives no retroactive credit.
The corrected adapter and explicit `chrome` channel then ran the exact same
39-case independent oracle shard: 35 pass / four popup failures, with
source-bound schema-5 JUnit/provenance and task-only CDP inventory at the
start/end (report SHA-256
`0f17a39b482acd1839914b1ce3133ea35251364ac14c41f7eecb78d7ad931197`).
The paired current-wheel candidate then completed all 39 cases: 25 pass /
14 fail, with a verified schema-5 sidecar (report SHA-256
`2959f793ffab73c4e511ca20c0088424dfa1d882dcb167475afac264d6812bac`).
The exact comparison is 22 both pass / 13 oracle-pass-candidate-fail / one
shared failure / three oracle-fail-candidate-pass. Four persistent popup
oracle failures remain diagnostic. The later verified 35-case selection at
the top of this manifest promotes only oracle-passing rows, without relabeling
it as the original 39-row execution.

`tests/official/playwright-core-page-goto-manifest.json`
pins the full official navigation file; oracle collection listed 64/64
`chromium-page` rows and browser execution recorded 63 passes plus one
unhidden MagicDNS-fixture failure with verified source-bound provenance.
An initial candidate attempt was refused before execution when other tasks
reopened nonblank CDP pages. A later isolated full-file candidate run recorded
45 expected / 19 unexpected, with 18 candidate-only discrepancies and one
shared fixture failure (report SHA-256
`fa7d07dd8e2274b7e5f01196a26af766f517fda9bd239e5e4aa79871a6ba6a73`).
Core full-file capture and replay enforce its source
hash and exact 64-ID selection, and an opt-in Tailscale-bound second origin
repairs the collapsed-origin fixture; it does not prove a distinct Chromium
process. Python Context gap collection adds zero strict-ledger pass credit;
the Core oracle result has been rebased into the 48,477-row `core456` successor
with exactly 63 expected and one unexpected outcome change (SHA-256
`b3bfe2c43c130198d326710bc42aab34df19ad685659200352fca24b05ae542e`).
The candidate's corresponding strict successor changes exactly those 64 rows
to 45 expected / 19 unexpected, with 48,413 unrelated rows unchanged
(SHA-256 `720ac506039bf4cc863baff4bb15b9bbb92b7bd00f0b67eb23063d398a56c5bd`).
Read-only triage identified top-level referer policy, `data:` request-event
filtering and finite navigation-waiter capacity among the 18 candidate-only
failures. The referer path is now locally repaired with 133/133 release crate
tests; it is included in the latest artifact but has no official rerun/credit.
The `data:` network projection now has a bounded local suppression fix with
16/16 default, 24/24 selected-feature and 12/12 model release tests, but
no official credit. The 20-waiter capacity gap remained in that historical
artifact; the newer opt-in source-only 20-waiter-plus-command repair is
summarized above.
`docs/architecture-v2.md` records the added bounded CDP-only suppression
owner and its late-event eviction limit.
At that checkpoint its exact path needed 20 Page slots and 20 Frame host
operations, not 21 waiters;
simply raising per-Page slots across 32 owners would grow the fixed-arena
declared lower bound from 66.78 MiB to 333.91 MiB. The newer source selects
the narrow opt-in capacity instead; measured browser proof remains required.
The cumulative Node native release artifact SHA-256 is
`51b14eec29f4b858e611d7b8ce5b6ded95043f11bbcb0a660f65b860f6ca43dd`;
its ABI smoke passes 1/1. The browserless Node contract suite passes 563/563
and strict TypeScript passes. Undefined navigation-timeout reset and navigation
error call-log presentation have focused Rust/Node checks. The later complete
64-case official browser rerun and strict ledger are recorded at the top of
this manifest; the earlier blocked attempt did not create evidence or touch
the other task's pages.
See the language-specific conformance documents.

Optional WebSocket transport leaf checkpoint (2026-10-10):
`docs/architecture-v2.md` and `docs/design-tasks-v2.md` are the triggered
architecture/task records for the new protocol-neutral
`pwright-transport-ws` leaf. Its default package has no runtime WebSocket I/O;
`live-tokio` adds a direct client connector. Bounded typed headers/canonical
endpoints, message caps, and parser-accepted Close-frame reason are independent
of CDP and the Playwright remote protocol. Neither host nor Node/Python selects it yet. The
focused CI step is added but has not run on GitHub Actions. Local scoped
release tests pass 16/16 without the live feature and 19/19 with it; targeted
rustfmt and strict release Clippy pass both selections. Dependency trees prove
the default leaf excludes `tokio-tungstenite`, the live selection includes it,
and default Node host/Python addon closures exclude this leaf. Five upstream
Python fake-WebSocket rows remain `not-run`, with the literal loopback endpoint
versus Tailscale fixture-bind blocker unresolved. The leaf also now keeps
remote Close replies and canceled/failed local close attempts retryable;
Tungstenite 0.26's full-handshake TRACE credential logging is a production
selection blocker. No conformance credit.

Python BrowserContext lifecycle evidence checkpoint (2026-10-10):
`docs/python-official-conformance.md` records the three-case unchanged
Chromium async Context/Page close-and-enumeration manifest at
`packages/pwright-python/tests/official/browsercontext-lifecycle-chromium-manifest.json`
(SHA-256 `ad583439cbfebb64f51efa915696dec53ed4fa0b17e70127b869bc4de35140a7`).
The pinned v1.58.0 source and fixture hashes pass the source checker. The
browser-backed candidate and independent oracle each pass 3/3 with verified
provenance sidecars, and those three IDs are strictly promoted in both
6,526-row Python ledgers. Candidate: 1,271 passed / 11 skipped / four failed /
5,240 not run (SHA-256
`5dc187f795c70934492890f1b8081158631ca409b0730e109ba02f2b0eabc228`);
oracle: 828 passed / ten skipped / four failed / 5,684 not run (SHA-256
`e33c80e0ae686cc8cce0e3fccac4a2c326aea9a956fe8f39636347ad196fa892`).
The exact JUnit/provenance records are in the Python conformance document;
this selection does not establish full-suite compatibility.

Python BrowserContext lifecycle follow-up checkpoint (2026-10-10):
`docs/python-official-conformance.md` records the subsequent unchanged
four-case Chromium Context lane at
`packages/pwright-python/tests/official/browsercontext-lifecycle-followup-chromium-manifest.json`
(manifest SHA-256
`8483d7c56eff097a77c02262670551c0a1ae93911b02e40941030e686c3a5141`).
Candidate and independent oracle each pass 4/4, strictly promoting only those
four IDs. At this intermediate checkpoint, the 6,526-row ledgers were candidate 1,275 passed / 11
skipped / four failed / 5,236 not run (SHA-256
`a51ed018358f528437c7684b9769f7affd7d567c84cbf60859800ca7bc7288ce`)
and oracle 832 passed / ten skipped / four failed / 5,680 not run (SHA-256
`bd339cdfd6b1db64377fe13c0e10fd525d0caa4da07ff00521824e54610e5d95`).
This does not change the five WebSocket rows' `not-run` status.

Python BrowserContext multipage checkpoint (2026-10-10):
`docs/python-official-conformance.md` records a third unchanged four-case
Chromium Context lane at
`packages/pwright-python/tests/official/browsercontext-lifecycle-multipage-chromium-manifest.json`
(manifest SHA-256
`281319eac629f5c879edd11184c66296a628407816a949b007c7ad54ebe28a67`).
Candidate and independent oracle each pass 4/4, strictly promoting only those
four IDs. The latest 6,526-row ledgers are candidate 1,279 passed / 11
skipped / four failed / 5,232 not run (SHA-256
`f1b3290a9321004934c6219fb237a8307700b75e7a6a023752afa03fa6d5d1b4`)
and oracle 836 passed / ten skipped / four failed / 5,676 not run (SHA-256
`a5d269049c709e20ba1c56cf487ec08b0e9ac76b23120182d7f1a7cbe5b6ed9b`).
The three manifests together added 11 paired Context public-API passes;
all other rows, including the four oracle-matching fixture failures, retain
their previous outcomes.

Python all-engine executable-path evidence checkpoint (2026-10-10):
`docs/python-official-conformance.md` and `docs/design-tasks-v2.md` record
the unchanged pinned Firefox/WebKit async/sync metadata cases after exact
official revisions 1509/2248 were installed. Candidate and oracle each pass
4/4 per engine without launching a browser. Four new schema-5 provenance
sidecars reverify against the archived exact capture tooling/harness, not the
later formatted live tooling. Only the four previously `not-run` path keys join
the Chromium-only strict
successor; all six `executable_path` keys now pass in both 6,526-row ledgers.
The candidate all-engine ledger is 1,268 passed / 11 skipped / four failed /
5,243 not run (SHA-256
`e6ab49adfd8a2f64b9d2736e43328917faab17437d2c373076f0ceccb5d14e22`),
and the oracle is 825 passed / ten skipped / four failed / 5,687 not run
(SHA-256 `cabcfc78429d12e962fefd8df8ddc3e4c9e77ff842e551b02dc4c4e814be92a1`).
The full four JUnit/provenance SHA-256 pairs, immutable ledger paths and
unchanged audit-classification SHA-256 are in the Python conformance doc.
Positive Firefox/WebKit launch, `PLAYWRIGHT_BROWSERS_PATH=0` root parity and
full-corpus compatibility remain unproved.

Python Chromium executable-path evidence checkpoint (2026-10-10):
`docs/python-official-conformance.md` and `docs/design-tasks-v2.md` record
the unchanged pinned async/sync Chromium metadata tests: isolated candidate
and oracle each pass 4/4 with capture-time-verified schema-5 sidecars binding
installed revision 1208. Only the two previously `not-run` executable-path keys are
selected for strict promotion. New 6,526-row ledgers are candidate 1,264
passed / 11 skipped / four failed / 5,247 not run (SHA-256
`97693577b5a02e2c2358372fcef6db9b575af58e3fb67492ee01f83a50b184a2`)
and oracle 821 passed / ten skipped / four failed / 5,691 not run (SHA-256
`71bf7132c80a286de32064b461f27228e14653fc6fa105c3b2291f9f357e0108`).
The full four-row JUnit/provenance hashes, immutable ledger paths, and
unchanged audit classification SHA are in the Python conformance document.
An isolated installed-wheel registry probe now rejects upstream `.egg-info`
shadowing; 59 focused tool tests pass. No browser was launched. At this
Chromium-only checkpoint, the four Firefox/WebKit path rows were still
`not-run`; the all-engine successor above promotes them. `=0` root parity
and the full corpus remain open.
Exact ABI110 capture tooling and harness are archived and both v2 sidecars
verify against them; the candidate package remains verified in its retained
isolated venv, not through the source-only `--package-root` override.

Playwright v8 Context lifecycle leaf checkpoint (2026-10-10):
`docs/architecture-v2.md` and `docs/design-tasks-v2.md` record the independent
`pwright-trace-playwright-v8-context` Rust state/lease coordinator. Injected
output, clock, IDs and ledger permit bounded chunk/group lifecycle and
fail-closed cleanup without selecting a browser, task, filesystem or ZIP
writer. Release tests pass 19/19, with strict Clippy and dependency proof.
The focused CI gate also checks default-host tree exclusion but has not run
on GitHub Actions. Context close/disconnect/cancellation must call explicit
abort; production output also needs RAII temp-file cleanup. No host/Node/Python
integration, `.network`/ZIP export, official trace outcome or strict-ledger
promotion is claimed. The existing broader tracing tasks remain open.

Selected Playwright trace-directory launch checkpoint (2026-10-10):
`docs/architecture-v2.md`, `docs/design-tasks-v2.md`,
`docs/node-ffi-compatibility-design.md`, `docs/upstream-playwright-conformance.md`
and `packages/pwright/README.md` record the opt-in Node/Rust directory
preparation boundary. Four browserless native tests and 533 Node units pass;
the exact Core dummy-process oracle and selected candidate each pass 1/1
under the pinned process/network sandbox, with diagnostic provenance only.
No Context recording or strict-ledger outcome is claimed. The Core
classification auditor now accounts for 27 private `_tracing` invocations
without changing their `not-run` outcomes. GitHub Actions has not run the new
selected-profile step; it clears global `-D warnings` for nine existing
host/addon dead-code warnings, which remain technical debt.

Opt-in tracing and startup-log dependency checkpoint (2026-10-10):
`docs/architecture-v2.md`, `docs/design-tasks-v2.md`, and
`docs/upstream-playwright-conformance.md` record the independent bounded
Playwright-v8 JSONL/directory Rust leaf and the new provider/composition
`startup-logs` Cargo feature. The default Node host dependency tree excludes
the trace leaf. Release tests and strict Clippy pass in both startup-log
selections and for the trace leaf; CI steps are added but have not run on
GitHub Actions. No official trace/launch outcome or strict ledger is advanced
by these implementation-only checks.

Core dummy-process diagnostic checkpoint (2026-10-10):
`docs/upstream-playwright-conformance.md` and `docs/design-tasks-v2.md`
record the pinned, sandboxed official Core line-64 launch-failure comparison.
The oracle passes 1/1 with both a public-launch and dummy exit-1 receipt;
candidate fails before spawning the dummy because inherited `tracesDir` is
unsupported. The candidate has no valid process provenance and neither strict
ledger changes. The dedicated manifest/runner/process guard and 17 focused
Python tests are browserless; Node `handleSIGINT: false` compatibility passes
529/529 package tests and TypeScript checks. The bounded native startup-log
capability is now explicitly selected by owned Node launch and passes
browserless provider/composition/host release tests. The rebuilt owned-launch
addon also passes a browserless Node-to-N-API-to-Rust synthetic-process smoke
test (1/1); `tracesDir` still blocks the official candidate. CI now gates that
opt-in build and smoke test, but GitHub Actions has not run the new step. A
separate opt-in tracing capability remains open.

Python fixture-host blocker evidence (2026-10-10):
`docs/python-official-conformance.md` now points to the exact four-row
environment-blocker registry and read-only checker. Both full strict ledgers
still record the four tests as failed public-API cases; matching oracle failures
explain the MagicDNS/`localhost` fixture mismatch but earn no pass credit.
The checker pins historical sources, reports and sidecars without running a
browser or changing applicability classification.

Python dummy-Node launch diagnostic checkpoint (2026-10-10):
`scripts/run_playwright_python_dummy_node_diagnostic.py`, its four focused
tool tests, and exact manifest SHA-256
`7f191e7c75bf0decd1a19f065f54a96bff5d5dd57bd43db6513fe242bae9c50b`
guard one unchanged official Chromium launch-failure test in async and sync
oracle/candidate modes. All four 1/1 runs passed under a process-exec
allowlist and outbound-network denial, with public launch failure and dummy
Node exit-1 receipts; no browser/CDP was started. The first oracle async
attempt failed at fixture import before launch and remains non-credit.
`docs/python-official-conformance.md` records all JUnit/provenance hashes.
These are diagnostic-only, do not prove successful browser launch or
Firefox/WebKit launch semantics, and do not change either strict ledger.

Persistent-Context preflight boundary checkpoint (2026-10-10):
`docs/architecture-v2.md`, `docs/node-ffi-compatibility-design.md`,
`docs/design-tasks-v2.md`, and `packages/pwright/README.md` are the triggered
architecture/task/package records for the new dependency-free Rust decision
crate and opt-in Node N-API profile. Default behavior retains the JS leaf;
valid persistent launch is still unsupported. Release leaf tests 28/28,
default and selected Node units 512/512 each, selected native bridge 4/4,
pinned guarded public comparison of 16 rejection and three accepted-preflight
inputs, strict TypeScript, formatting, release addon build, native-load and
exact dependency-closure checks pass. The selected
artifact SHA-256 is
`e1288f11e26c9d54aadb6f35f985a5c444fb4154539bada11a358cef18677ca3`.
The Node 20 CI job now builds and tests this opt-in profile independently.
The browserless public differential first exposed and then verified a fix for
boxed-empty certificate `origin` and `passphrase` conversion in both Node
profiles. The dedicated native-test script now selects its profile, so a
default-profile run cannot pass by skipping all four checks.
No official ledger outcome changed. `docs/upstream-playwright-conformance.md`
remains the corpus accounting record; the Rust extraction adds no official
test evidence. The opt-in Rust decision now matches pinned classic Firefox
`-juggler*`, while `moz-*` BiDi and truthy non-array `ignoreDefaultArgs` retain
their bypass behavior. The unchanged default JS profile still has that parity
gap; positive owned-launch gates remain open. The focused selected suite
passes 23 with one intentional skip, the default suite 15 with nine opt-in
skips. CI does not set the local pinned-Core oracle path, so its two oracle
comparison tests remain skipped there.

Core455 source-only accounting checkpoint (2026-10-10):
`docs/upstream-playwright-conformance.md` and `docs/design-tasks-v2.md`
record 18 guarded, no-I/O upstream-private invocations in each lane. The
48,477-row candidate and oracle successors change exactly those 18 rows from
`not-run` to `expected`; their SHA-256 values are
`5cc090bd05c928255cc9da69d9e535879c59af467c4ecb89445a827257cef14e`
and `eb54dc153626706a97e54fad339f1c610e0d9f23232a7c614f74da59ef1bda6e`.
Their source-only evidence explicitly does not attest the Pwright runtime or
public API parity. The separate applicability audit classifies 31 additional
false-public rows as internal; no other official outcome changes.
CI now verifies the pinned upstream AST guard and 67 browserless Core
accounting-tool tests, installing TypeScript so source-only guard tests cannot
silently skip. It also bootstraps a clean pinned upstream checkout using pnpm
only and replays/verifies all eight Core455 schema-3 sidecars. This new gate
passed locally under Node 20 / pnpm 10 (18/18 in each lane), but GitHub Actions
has not run it yet. The source-only evidence still grants zero public-API
parity credit.

Owned-launch checkpoint (2026-10-10): the optional Chromium launch
composition arms its successful provider lease before CDP binding; two
fake-process tests cover abandoned takeover and cancelled cleanup. A fresh
Rust host entrypoint shares the owned-launch handoff, while public Node and
Python Chromium `BrowserType.launch` now exist behind opt-in features with
explicit executable paths and temporary profiles. The Python source/native
package is ABI107; its 330/330 package units, Ruff, strict Pyright and release
build checks pass. Node has 511/511 package units in each of default,
owned-launch and browser-registry profiles; its type, boundary and
release checks pass. The optional owned-launch addon and isolated Python wheel
were rebuilt from the newest host guard below; default Node remains unchanged,
while editable Python was replaced after exact-source archiving below. The native default Context remains physically present but unpublished;
strict unaddressability, process-tree supervision and successful positive
launch/close conformance remain open. Firefox/WebKit positive launch and
valid `launchServer` remain unsupported.

Python ABI107 WebKit persistent-Context page-argument preflight now matches
the unchanged official `owned` case without enabling valid WebKit launch.
The pre-fix oracle/candidate pair was green/red; after narrow facade ordering
repair it is green/green, with Firefox an official skip in both lanes. Four
schema-5 sidecars and an exact post-fix archive verify. New strict 6,526-row
ledgers are candidate 1,259 passed / 11 skipped / four failed / 5,252 not run
(SHA-256 `3bae07314d8d9b717a503e1b553d813faa5f11d01deec922cdf9d8d33ddf0c81`)
and oracle 816 passed / ten skipped / four failed / 5,696 not run (SHA-256
`c622c0a3c4aa8ab9a0afdfbfa1aa89531b683ae50fc4e63f7fbeac38a54852b6`).
The historical manifest has no explicit browserless policy; the no-spawn
claim is source/fixture/process-observation evidence, not a harness guard.
The new exact-only browserless manifest SHA-256
`3e59ca6e06feea4101e87840c5bfe1f73e72f2d326fa9a8d738198245e66b869`
adds that guard without changing the historical files: pinned test AST and
fixture/package hashes, owned one-engine async selection, no CDP/channel or
injection environment, plus pre/post source checks. WebKit again passes in
both lanes and Firefox remains a shared upstream skip. Four new schema-5
sidecars verify live and against
`.work/official-conformance/python-abi107-webkit-browserless-exact-archive.apuY9p/`.
Fresh 6,526-row strict ledgers refresh only those two evidence references,
not outcomes: candidate SHA-256
`24a36b73150a04012f1ca45911601758ef9ae61bad186b2b28af374995242888`;
oracle SHA-256 `ae78115993a256ca90af4c6da4389b924422ce37d6149eba965c034e27cb39c3`.
Both applicability audits retain SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
Oracle starts a Node driver and may create temporary directories; no-browser
does not mean no subprocess or side effect. Focused guard/provenance tests
pass 81/81; two older profile tests fail under local pytest 8.4.2 because
their fixture pins pytest 8.4.1.

Python's next browserless official tranche pins the unchanged async invalid
numeric `proxy.server` case across all three engines. A strict owned-only
manifest locks the test AST, lazy `browser_factory` fixture chain and both
pre-spawn implementations; the official Node driver and Tailscale-bound
fixture servers run, but no browser/CDP process is reached. Oracle passed
3/3, candidate failed 3/3 before the narrow protocol-shape correction, then
passed 3/3 without enabling valid proxy launch. Red and green schema-5
sidecars reverify against their separate exact source archives. The latest
6,526-row Python strict ledgers change only these three public-API keys:
candidate 1,262 passed / 11 skipped / four failed / 5,249 not run (SHA-256
`9abb589487cadbb01d379876e6fdb4bd08d093bcaf10c246e4ab0845946841e8`),
oracle 819 passed / ten skipped / four failed / 5,693 not run (SHA-256
`e396771db5ad1b646ac88214af3f5b05291e543a3459200c95bf2d92769d9523`).
The complete applicability classification SHA remains
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.

The Python official runner's `owned` mode disables both historical CDP
launch adapters, omits the endpoint, preflights the real public launch, and
records `launchMode` in schema-4 provenance, JUnit and isolated receipts.
Candidate and oracle each collected 12 selected preflight rows, but
collection-only JUnit has zero executed tests. In separate Chromium executions,
the unchanged official async/sync invalid-page-argument and nonexistent-
executable cases passed 4/4 in both lanes. Both execution sidecars verify;
exactly four `not-run` keys in each 6,526-row strict Python ledger were
promoted to `passed`. At that checkpoint candidate had 1,250 passed / eight skipped / four
failed / 5,264 not run (SHA-256
`e47bb3fb7b82985ce3b6d956b6ac57b4e2363bfefc3deca2fe0de7bb57fc6917`);
oracle had 806 passed / seven skipped / four failed / 5,709 not run (SHA-256
`3c6a1fe198573c937757574e9c4337155cd00a4cc4d2180bcee65ac714803ff2`).
These four error-path cases do not launch Chrome and do not prove positive
owned-browser behavior. All six prior ABI106 schema-2 metadata sidecars
reverify with exact archived tooling and pre-schema-4 shim/runtime sources
at `.work/official-conformance/owned-launch-pre-schema4-tooling/`. Full Core
48,477-row and Python 6,526-row conformance remains open.

The eight remaining pinned Firefox/WebKit preflight rows then ran separately
by engine, browserless, with candidate and oracle each six passes and two
upstream skips. Four new source-bound schema-4 sidecars reverify. The strict
6,526-row successor ledgers changed exactly eight more `not-run` keys:
candidate 1,256 passed / 10 skipped / four failed / 5,256 not run (SHA-256
`67e9d92c0ffb5a020bf9fa207cdac90871fc32789f2854cc3cef8f22cc8d4969`),
oracle 812 passed / nine skipped / four failed / 5,701 not run (SHA-256
`1c7fba90cd6c29e030bf5bd5e63248bf7e8504c5b212bc61fdd79d1b56667686`).
The successor filenames say `abi108` as a checkpoint label, while the native
candidate ABI remains 107. A combined two-engine pytest report with a
fixture-order async-after-sync error is diagnostic only, not ledger evidence.

Python browserless internal glob tranche: pinned manifest SHA-256
`70c6c6cad3d032e4e03136d4d5e3c2c7029e0e80aa8144a8dff21858f6620af3`
selects `tests/async/test_page_route.py::test_should_work_with_glob`. Each
candidate/oracle engine row passed 3/3 with six verified schema-4 sidecars,
without starting a browser. Exactly two candidate and three oracle `not-run`
keys advanced in the 6,526-row ABI109 ledgers: candidate 1,258 passed /
10 skipped / four failed / 5,254 not run, SHA-256
`865700ea62525460b5f64e267c43cea91a4e527e33a4f14dff3514bc05edfa95`;
oracle 815 passed / nine skipped / four failed / 5,698 not run, SHA-256
`1ee5d32fe559121c2d07aa051bec77a00fe6a78fa6fc4e31e0455d2d6dc70f18`.
The formal applicability audit classifies these rows as
`implementation-internal`/`non-applicable`, since the upstream test imports
private helpers. They measure full-corpus execution, not public API parity.

Positive candidate-only owned-launch diagnostic: a separate exact-source
Python async/sync close-twice manifest (SHA-256
`b6f8c788e55b88c0aec8e8ae02ca901e51973f388993f6af6650031dff2ff854`)
ran with Chromium channel `chrome`; Pwright launched the installed browser in
temporary profiles and both unchanged official tests passed. The candidate
JUnit/provenance SHA-256 pair is
`80a1d7293dfc9a65a7c4dafbd51cca0e75593639566d3e4ef85c856501c1a089` /
`e1021c7fdece10d9162451684bfca72dc4c9ff08350948d1fe9d7ff17e9a0019`,
and the sidecar verifies. No matching temporary-profile process remained
afterward. The official oracle's genuine browser launch needs a narrow
exception to the standing Pwright-only browser rule; it was not run. These
two keys remain `not-run` in the strict ledger, and process-tree cleanup is
not yet proven by the current provenance.

Core owned-launch staging: a separate exact-source one-spec manifest (SHA-256
`2b1cfe4ba33f5b4ba15b7d2b84fcc5c11e26169a64a91fa016b2d7ccaea44286`)
selects pinned upstream `chromium-library`
`tests/library/browsertype-launch.spec.ts:130` (`should be callable twice`).
The new runner calls the real public `chromium.launch` in candidate/oracle,
without the historical `launch -> connectOverCDP` adapter, and labels any
result diagnostic until process/profile identity is attested. Browserless
official collection resolves 1/1; new and existing runner checks pass 10/10
and 31/31. The opt-in release Node addon exists at
`packages/pwright/native/owned-launch/pwright.node` (SHA-256
`9231bb5f4e0407c7984dcdbc77b8fb08d979f76c5e6df31b5b15e68ee22d4b43`).
No Core owned browser/spec was run. A true run would create a new Chrome process outside the mandated
`chrome-cdp` workflow, so it awaits a narrow user-approved policy exception.

Core browserless launch preflight: pinned upstream
`tests/library/browsertype-launch.spec.ts:75` passed 1/1 candidate and 1/1
oracle with verified provenance; its manifest SHA-256 is
`16be68d3b2368a5fff8a9f201cf697c886ec450c03cf1982f0c34ac7b382a98b`.
The 48,477-row core443 ledgers change exactly that row from `not-run` to
`expected`: candidate 1,604 expected / six skipped / seven unexpected /
46,860 not run, SHA-256
`5a74cb8955c1d717f399a2791ee56e55c6a7783306a77173355e2ef5c9a80bef`;
oracle 1,307 expected / five skipped / six unexpected / 47,159 not run,
SHA-256 `9a341bfca76a845f33aaeb1c61e2b75af131bd1ea104b9817205971ed5957d66`.
No browser was started, so this does not prove positive Rust launch.

Core browserless arguments/proxy tranche: exact-source manifest SHA-256
`4f7f527c2f63ce4a1bc9bd8cc67f65d566cd5d1f54cd7a54651ef3d23fc63ab5`
selects pinned `browsertype-launch.spec.ts:40`, `:56` and `proxy.spec.ts:23`
under Chromium Library. Candidate and oracle each passed 3/3, with
source-bound provenance verified. The strict core444 48,477-row ledgers
change exactly those three `not-run` keys to `expected`: candidate 1,607
expected / six skipped / seven unexpected / 46,857 not run, SHA-256
`791eee8a7fb2c35d2151862adbf1f569eab24be9f35cda1e787d83fd2ac1a223`;
oracle 1,310 expected / five skipped / six unexpected / 47,156 not run,
SHA-256 `5a1ec13430fc57ccb1301e7b6eefd6647c1d354e96d6a19b80540ae3aec5deb2`.
All three reject before browser spawn. At this checkpoint Node units passed
468/468; types and dependency boundary checks pass.

Core Firefox/WebKit invalid-executable preflight: the same pinned upstream
`browsertype-launch.spec.ts:75` passes 1/1 for each non-Chromium Library
engine in both candidate and oracle, with four verified source-bound
sidecars, no browser spawn, and manifest SHA-256
`455f923e5866f39055b802f996135ab9271dd0133543ec7c21f74fd66cc4d44b`.
The core445 48,477-row ledgers change exactly those two engine keys from
`not-run` to `expected`: candidate 1,609 expected / six skipped / seven
unexpected / 46,855 not run, SHA-256
`8f281beff4c1851717a237d903733082ffbd515455ccdb84ee336722f94ed821`;
oracle 1,312 expected / five skipped / six unexpected / 47,154 not run,
SHA-256 `3aa0fa8c99dfd9c62eacf67db0610b28adf869fcb66965ffe8c235072cbc850a`.
The full invocation audit passes. Node units now pass 471/471 and types and
boundary checks pass; this does not prove positive non-Chromium launch.

Core Chromium `launchServer` remote-pipe preflight: pinned
`tests/library/chromium/launcher.spec.ts:19` passes 1/1 in candidate and
oracle with verified provenance and no browser spawn. Manifest SHA-256 is
`aa9845826071c8794121536e0a58083b87ecb655ec0ff2a3bb49afab43be4d8c`.
The core446 strict 48,477-row ledgers change only this key from `not-run` to
`expected`: candidate 1,610 expected / six skipped / seven unexpected /
46,854 not run, SHA-256
`2e06cebde74d8643ba48ab31c8834c3282ccc0bf2c8fbb5548a62aff9fcca2c1`;
oracle 1,313 expected / five skipped / six unexpected / 47,153 not run,
SHA-256 `71fc385156b5612447037edeb69cef8a5db6d138ad202155fe44c848db17d7a9`.
Full invocation audits pass. Node package units pass 472/472 and type checks
pass. This does not implement positive Playwright BrowserServer semantics.

Core Bidi Library preflight batch: exact-source manifest SHA-256
`4ae7f22502c2f3e65fc6f91b30c5d84280a24c62cb07a913efd34cae1efafbcb`
selects four pinned pre-spawn BrowserType error rows in each of six Bidi
projects. Candidate and oracle each passed 24/24 without a browser/CDP
endpoint, with verified source-bound provenance; all 24 classify as public
API candidates. The core447 strict 48,477-row ledgers change only these keys
from `not-run` to `expected`: candidate 1,634 expected / six skipped / seven
unexpected / 46,830 not run, SHA-256
`e9623f7b7e56e4e5330e971496d1ce632389398c7a703eefe1fff57e6ab360b5`;
oracle 1,337 expected / five skipped / six unexpected / 47,129 not run,
SHA-256 `4568ec15716616d37ce912c00a3c34a04ec5aa2f45f38be9eb54d8884c4184b0`.
This does not prove positive Bidi browser launch.

Core Bidi preflight follow-up: exact-source manifest SHA-256
`e645e86f85df3f61dda3481e153bc8662982bb06590d5c604e8d0d54ca5db2d9`
selects 18 additional invalid-channel, proxy and Chromium launch-argument
rows across Bidi projects. Candidate and oracle each passed 18/18 without
browser/CDP. All ten Core schema-2 source-bound provenance sidecars verify;
the 18 keys classify as public-API candidates. The core448 strict 48,477-row
ledgers change only these keys from `not-run` to `expected`: candidate 1,652
expected / six skipped / seven unexpected / 46,812 not run, SHA-256
`099c97f1180d1c6e27d5d90b3d38ec8c162cce90030bf2558cc27a6061c371af`;
oracle 1,355 expected / five skipped / six unexpected / 47,111 not run,
SHA-256 `d2ad0a6c5f0aa882c4924d21f14307df0cf7fe05a551240c2b4663b86159af9b`.
This is pre-spawn parity only, not positive Bidi launch evidence.

Core Library BrowserType metadata gap: manifest SHA-256
`ae93f4e445c4b1cce86e3c89e24dc714115b8718da3b91f8320867c503ee4bf3`
selects five browserless pinned invocations. Candidate/oracle each have two
public `name()` passes and three `executablePath()` unexpected outcomes;
all six schema-2 sidecars reverify. The core449 strict ledgers change only
these five keys: candidate 1,654 expected / six skipped / ten unexpected /
46,807 not run, SHA-256
`890b27693fd6910a5379b7c79bf245ad08de9a222706922079eecb126b5aef80`;
oracle 1,357 expected / five skipped / nine unexpected / 47,106 not run,
SHA-256 `72cdf773d4ba9b2ee4b75e3277a78b8fcc66d2a20cdc4d7afdeff1ad248daf58`.
The executable-path test first accesses upstream private
`_playwright._defaultLaunchOptions`; candidate fails there before calling
the public method. Oracle reaches that method but its browser executable is
not cached on this Mac. The fixed whole-corpus audit labels the three run
Library and six unrun Bidi variants `implementation-internal` with
`private-client-access`, not a proven public API discrepancy. It changes 72
classifications, no outcomes; both new 48,477-row classification SHA-256s
are `f5e63b043f85437a2cc3e6b5b5972a9db98ea0fb1b23f588aa84614f1eeae3d3`.
Separately, source inspection confirmed Pwright had no public JS
`BrowserType.executablePath()` implementation or TypeScript declaration at
that checkpoint; the unchanged upstream test could not measure that gap
before its private precondition.

Core450/451 browserless launch-argument parity: the exact-source manifest
SHA-256 `1e89c9547baaf719bfc288259f68ee6bbd8d082acf31858679435437b1ac9d49`
selects 12 public-API invocations in eight fixed Library/Bidi suites, without
browser/CDP use. Core450 oracle passed 12/12; candidate passed 3/12, with nine
Firefox/WebKit profile/page-argument errors incorrectly rejected as generic
unsupported `args`. All 16 source-bound sidecars verify. Strict core450
candidate ledger: 1,657 expected / six skipped / 19 unexpected / 46,795 not
run, SHA-256 `49c9c000cdc4e32eaf4ec0886deaa2a05bec0aa1c2123821af98bdaa11b502c0`;
oracle: 1,369 expected / five skipped / nine unexpected / 47,094 not run,
SHA-256 `f6ee9a5b3c26c31bd8a0c6c6dd0d1440d9a33e56d8aa2a4562a809c395ebb5c1`.
The pre-fix candidate source/addon is archived at
`.work/official-conformance/pwright-node-owned-core450-before-preflight-fix.t1ghwh/`
and its eight candidate sidecars reverify there. The narrow Node preflight
fix plus five AAA units now passes the exact same 12 candidate invocations
in core451 (12/12), matching the 12 prior oracle rows. The core451 candidate
ledger changes exactly these 12 source-bound rows and has 1,666 expected /
six skipped / ten unexpected / 46,795 not run, SHA-256
`a864e42b300b8a98dfbb65880bed7f1e9fce2cb1aaaab24acd5fc797a673b38e`.
Eight new candidate and eight unchanged oracle sidecars reverify; all 12 rows
remain public-API candidates in the corrected 48,477-row audit, SHA-256
`f5e63b043f85437a2cc3e6b5b5972a9db98ea0fb1b23f588aa84614f1eeae3d3`.
The current source/addon archive is
`.work/official-conformance/pwright-node-owned-core451-launch-preflight.oPZl90/`.
No positive Firefox/WebKit launch is inferred.
The five remaining core451 `unexpected:public-api-candidate` candidate rows
are all browser-backed and also oracle-unexpected: cached-image response wait
timeout (one), unavailable legacy device-orientation/motion initializers
(three), and an extra-header XPASS against upstream's expected-fail annotation
(one). No candidate-only public discrepancy is proved by these five; keep the
original outcome statuses pending dedicated Chrome/CDP fixture diagnosis.

An opt-in Node `browser-registry` profile now exposes synchronous public
`BrowserType.executablePath()` through the existing Rust registry leaf and
types it as `string`. Its release addon SHA-256 is
`5f8e08938a76769e783254d72447a991b83cf40ecd4ee48b9005150ea1dcfc32`.
Native path tests pass 4/4, adapter units 13/13, and default, owned-launch and
browser-registry package suites each pass 490/490; TypeScript, native-load and
dependency closure checks pass. A browserless direct public-method comparison
against pinned `playwright-core@1.58.0` matches expected paths for all three
engines (3/3), with no browser installed/launched. This is not an official
core449 promotion: its private precondition still fails before the candidate
public method, and the oracle's pinned browser cache is absent. The selected
feature adds only the registry leaf, not install/download or owned launch;
the addon already depends on `node-host`, so this is not a host-free binary.
That first binding read registry environment variables per call, unlike
upstream's module-load snapshot. Its successor captures the registry inputs
at import and delegates path composition to Rust: release addon SHA-256
`641dd2867c3ccb5a3961020bdd2dd3e738cee2a15da141c654b1261e2cb4dedb`.
Pinned public-method fresh-process differential tests pass 13/13, Rust leaf
9/9, and all three Node profiles 511/511 package units; native-load, types
and boundary checks pass. The stale two-argument addon is rejected. This
still does not promote the private-hook core449 row, install a browser, or
prove cross-platform registry parity. Both strict ledgers are unchanged.
Separately, Node persistent-Context preflight now accepts boxed `String`
`channel`/`args` as pinned protocol strings. A dedicated 9/9 browserless
source-derived suite includes seven public-method upstream comparisons;
`pnpm run test:persistent-context-preflight` runs it with the pinned Core path
provided in `PWRIGHT_PINNED_PLAYWRIGHT_CORE`. No official ledger row was
added, and broader validation ordering, TLS and proxy parity remain open.

Core452 browserless persistent-Context preflight: the exact-source manifest
SHA-256 is `084e6070d99aff63e6ea80b6372d9f7366ccb8427aa1de689fd3349f68a86f85`.
Nine fixed Library/Bidi projects each ran the same two pinned selectors;
oracle recorded 17 expected and one upstream Firefox Library skip, while
candidate recorded 17 unexpected unsupported-boundary failures and the same
skip. All 18 paired sidecars verify; no browser/CDP was launched. Immutable
48,477-row strict ledgers change exactly 18 `not-run` rows each: candidate
1,666 expected / seven skipped / 27 unexpected / 46,777 not run, SHA-256
`db08893e67ca7e7f49449d48c1a1d5130d34d087166d4609866fbc6d6c20af58`;
oracle 1,386 expected / six skipped / nine unexpected / 47,076 not run,
SHA-256 `7b86ace6318713fbb73a96635bdb5559fa45361aecd0c38d9ad5af4ebaddfa4a`.
Both full audits pass at the existing classification SHA-256
`f5e63b043f85437a2cc3e6b5b5972a9db98ea0fb1b23f588aa84614f1eeae3d3`.
Nine page-argument rows are ordinary public-API candidates; nine certificate
rows carry `public-api-with-private-test-helper` due to an incidental file
import. The pre-fix candidate source/addon is archived at
`.work/official-conformance/pwright-node-owned-core452-before-preflight-fix.QcGtKL/`.

Core453 repair of the same 18 pinned invocations: a separate JS preflight
leaf checks client certificates and engine arguments before the unsupported
valid persistent-launch boundary; the unchanged official preload conveys
fixture channel defaults through a non-enumerable inherited setter on the
candidate CJS export. Candidate now records 17 expected / one same skip /
zero unexpected, matching every core452 oracle selected outcome. All nine new
source-bound sidecars verify; exact current package/addon archive is
`.work/official-conformance/pwright-node-owned-core453-before-next-fix.UWHQui/`.
The new immutable 48,477-row candidate ledger has 1,683 expected / seven
skipped / ten unexpected / 46,777 not run, SHA-256
`14c3ffac26ae15897c93d81b35a9b7853add367448484de4a7bb96cac6207cb7`.
Exactly 17 unexpected rows became expected, while the shared skip retained
its status with refreshed evidence. Full applicability audit remains 48,477
rows, SHA-256
`f5e63b043f85437a2cc3e6b5b5972a9db98ea0fb1b23f588aa84614f1eeae3d3`.
Default, owned-launch and browser-registry Node browserless package suites
each pass 507/507; TypeScript and boundary checks pass. This is pre-spawn
validation parity only, not successful persistent Context or TLS parsing.

Core454 closes the two remaining Firefox/WebKit Library `proxy.spec.ts:23`
invalid-numeric-proxy keys without launching a browser. Pinned exact-source
oracle and candidate reports each show two expected / zero skipped / zero
unexpected; all four schema-2 provenance sidecars verify against the exact
manifest
`tests/official/playwright-core-browserless-invalid-proxy-remaining-engines-manifest.json`
(SHA-256 `f4eb2595970997e1ccb166f229bfcc07c257fa406526c02c2616469a8dff96dd`)
and the
candidate source/addon archive
`.work/official-conformance/pwright-node-owned-core454-before-invalid-proxy.wOW0gt/`.
Strict 48,477-row ledgers change only those two `not-run` rows to `expected`:
candidate 1,685 expected / seven skipped / ten unexpected / 46,775 not run,
SHA-256 `b2362075a2f4f748388e4eabdd4a1fc9b799fc52babf816e9a2d3fa70a719292`;
oracle 1,388 expected / six skipped / nine unexpected / 47,074 not run,
SHA-256 `e4d41296a636a6d60000a0692e4f76ae39635af8967dd0f90ce71b0e1d2bf596`.
Both complete applicability audits remain at SHA-256
`f5e63b043f85437a2cc3e6b5b5972a9db98ea0fb1b23f588aa84614f1eeae3d3`.
No positive proxy/browser behavior is implied.
The next pinned Core `browsertype-launch.spec.ts:64` dummy-Node failure row
is prepared but unrun. Its schema-2 manifest SHA-256 is
`22e42850d6e287d6e4532ddcb7e3c08a8de9ae735625bf8492d20b9c78f8a86a`;
the generic runner rejects it before a subprocess, while a separate guarded
wrapper can produce only diagnostic evidence without OS spawn attestation.
Python policy tests pass 6/6 and Node VM guard tests 6/6. No official result,
browser/dummy process, or ledger promotion exists for this row.

Fresh-owned Python close-event repair: `Browser.close()` now emits each
retained Context's `close` event once after native close succeeds and before
returning. The behavior is limited to public fresh-owned launch; CDP attach
and persistent paths are unchanged. Async/sync/fake-CDP regression units are
green, and the Python package suite passes 308/308 with Ruff and Pyright.
No official close-event case has run yet. Exact pre-repair ABI107 package,
shim, runtime and capture tooling were archived at
`.work/official-conformance/owned-launch-abi107-pre-close-event/` so earlier
schema-4 sidecars remain source-verifiable after this façade edit. The
candidate-only verifier `--package-root` override maps the exact recorded
package to that archive, rejects symlink/escape and verifies package plus
original native artifact hashes; both captured ABI107 candidate sidecars
reverify with archived tooling/harness/package. Verifier units pass 35/35.

Owned-process shutdown correctness: the Rust host now transfers the unique
browser lease to a one-flight Tokio task on first close. Concurrent closes
await the same retained `ChromiumShutdownReport` instead of mistaking a taken
lease for completed shutdown; cancellation of one waiter does not cancel
the provider shutdown. Two fake-future release tests and the all-feature
release check pass without Chrome; rustfmt and diff checks are clean.
The host retains but does not yet expose that direct-child report to Node,
Python or the official runner. It does not prove Chrome descendant reaping or
temporary-profile deletion, and kill-on-drop before close remains unjoined.

Fresh-owned host ID guard source: generic command admission and direct
Context/Page/Frame entrypoints now check publication metadata before serving
guessed hidden-default IDs. The guarded direct paths include route,
init-script, binding, console/history/network, worker, timeout, pause,
file-chooser, download and service-worker operations. Earlier source passed
13/13 focused release tests before a memory-stopped rerun. The current Rust
source now passes an all-features release check and 76/76 focused browserless
release tests across command admission, Worker, Worker console/handles,
storage-state and FrameTrees; format/diff checks pass. At that checkpoint the
source-tree Node/Python native modules were not rebuilt. Async event publication and full hidden-ID
entrypoint exhaustion remain unaudited.
A new isolated ABI107 Python wheel from the final host source, SHA-256
`77b485d083121a08f0dea4fe1b9183c65ebda2b9a7e4ff3aa74f4d8726718c3a`,
is archived under `.work/official-conformance/builds/python-abi107-host-guard-20261010/`.
It imports from an external Python 3.12 venv with ABI107 and passes 308/308
browserless package units. The exact ABI109 source package, shim/runtime and
four source-bound tooling files were then archived at
`.work/official-conformance/python-abi109-exact-archive.yO61kG/`; an ABI109
seven candidate sidecars with that package digest verify against that archive.
The wheel's ABI107 extension
and both bundled OpenSSL dylibs were installed into editable Python, which
imports from the source tree and passes 308/308 units. The extension SHA-256
is `e3ebe18e4b1577ee18de03a72b5d9b72f320d46986e3f5e92a38316f6f28c465`.
No official Python ledger key advanced. New Python official sidecars use
schema 5: candidate `pwrightRuntimeLibraries` lists every adjacent
`shuozeli_pwright.dylibs` file with absolute path and SHA-256, oracle records
an empty list, and verification checks exact membership and content. A
`--package-root` archive must carry that directory beside `python/pwright`.
Focused provenance/verifier tests pass 57/57. The pinned browserless Python
glob test then passed candidate-owned ABI107 3/3 across all three engines;
all schema-5 sidecars verify live and against the exact package/tooling/harness
archive at `.work/official-conformance/python-abi107-schema5-glob-exact-archive.YE9OWt/`.
No browser/CDP launched. A new immutable 6,526-row candidate ledger refreshes
only those three passing keys, keeping 1,258 passed / ten skipped / four
failed / 5,254 not run; SHA-256
`778425cb062d329ef6ee0b3b24e62ce0f97696819abfe09944272dbaa1178639`.
Full audit SHA-256 is
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`;
the three rows remain implementation-internal/non-applicable. Historical
schema-1 through schema-4 sidecars retain their original scope.
A subsequent read-only audit identified conditional hidden-Page gaps. Current
Rust source now filters fresh-owned Worker owners before capacity use
and Worker/console notices before delivery by committed public Page, and
gates public Page/Frame admission on a published non-internal Page. An exact
task-local storage-state permit keeps private scratch operations and cleanup
possible; unpublished indexed Frames now select the scratch fallback.
Focused browserless release tests pass. The rebuilt optional Node addon and
editable Python extension pass browserless checks; publication timing and
live hidden-ID reachability remain unverified.
A bounded direct-entrypoint audit found no further high-confidence operation
bypass, but Rust-only `HostSession::page_snapshot` still reads live topology
and could expose a non-internal Page before public collection publication;
Node FFI reads the published projection. That timing window is untested.
The hidden default Page may also consume one of 16 total topology Page slots;
public-capacity semantics need an explicit contract test. These findings do
not change the official ledgers.
The old opt-in Node addon and exact package source are archived at
`.work/official-conformance/pwright-node-owned-core449.dJYI61/` (addon
SHA-256 `9231bb5f4e0407c7984dcdbc77b8fb08d979f76c5e6df31b5b15e68ee22d4b43`),
and all nine latest core447–449 candidate sidecars verify against that archive
after the `owned-launch` addon rebuild. The new addon SHA-256 is
`bfb401ad7a097cebf63f7d453732b18d401d3db41407e6482973f1a5044bf4ae`.
Its native-load smoke passes 1/1, owned-launch adapter 11/11, package suite
477/477 with either default or owned-launch profile, and TypeScript/boundary
checks pass. The profile-sensitive contract unit now asserts both distinct
rejections rather than skipping either. Three older owned-launch sidecars
still lack their earlier source snapshots and are not made reproducible by
this archive. No official browser launch or Python editable-extension rebuild
is implied.

The first four pinned async/sync Python `BrowserType.launch` preflight
functions are now frozen in
`packages/pwright-python/tests/official/browser-type-launch-preflight-manifest.json`
(SHA-256 `edc46b5f30f159ed5619bcb8e82f97caccecb10ca17e29db860fd45b57261279`).
The exact v1.58.0 source checker verifies the selection, commit and fixture
sources. Both lanes have 12-row collection reports; only the four Chromium
error-path rows have executed so far. This is a fixture-requiring launch
selection, not a browserless parity claim.

Earlier launcher ownership and Python empty-profile safety checkpoint (2026-10-10):
the pinned v1.58.0 Core/Python source and existing Pwright Rust/Node/Python
surfaces were audited without launching a browser. Rust has an opt-in
persistent Chromium process lease, but public Node valid launch/server and
Python nonpersistent launch remain absent. Both existing official Python
launch adapters rewrite the operation to CDP attach; their reports are not
owned-launch evidence. Python source now rejects empty-string persistent
profiles before native work instead of resolving them to the working
directory. Two new sync/async regressions pass, current-source Python units
pass 299/299 against installed ABI106 native, and Ruff/strict Pyright pass.
The earlier official report/ledger digests below remain historical snapshots;
this guard did not promote any official row. Design and limitations are in
`design-tasks-v2.md`, `upstream-playwright-conformance.md` and
`python-official-conformance.md`.

Core 12-key BrowserType preflight and final-source regression (2026-10-10):
`tests/official/playwright-core-browserless-browsertype-preflight-manifest.json`
SHA-256 is `e197f6a0aad744b0f65cb04120d07e3f758cd58dec832f6b560602ac3d2c4a81`.
For report stem `playwright-core-{mode}-library-browsertype-preflight-{engine}-20261010-v1`
under `.work/official-conformance/results/`, result/provenance SHA-256 pairs
are candidate Chromium
`537bcbb34098c73df6fa57ccaa6d609427622248887f20e384ea502a6ba7a71e` /
`62948a501571cb17a6f8326126bb615868dac7a4df93268184e63b163e774f0c`,
Firefox `4903e29e00a10760226f8defdb73aacff1c37af839f8b4d3617f410f4fc186c5` /
`542da274240a1d2b5fcceacf6212e1463a304744738ea2afec09f733470cf929`,
WebKit `d8096505ec16069d2a06ae52101096536b1a7bf0262f6018e12eba9043d11f93` /
`82d6de2bcd7d087c864e8def413bcbfe0445980d19fa6c45da9c0a872ec59aa1`;
oracle Chromium `b71a9e07211f092744ef82c8647893c547a5355e52e13bc187494a67cbbf834c` /
`0faf5ca13ee3433cb6228702f67433d91c7a6efa19cc8b2969c26bf58fd1b8ef`,
Firefox `350f5c6748b2485bf2c05ab03d0040cafc91e1ff92ceb410afc8dd6013e664b9` /
`571699e59b43cef0857fb8bcb71eb24e7cb7ab2098449e47f6c259878105c0ef`,
WebKit `be07e04c63277a4aacac514e011470b66f076af8912b10f1d04219ef2a81dd10` /
`edf5218c827e9c80a2bbb787a25b9b2c17b1a33ca1d5f00b17e1943318b9444e`.
All six source-bound sidecars independently verify; candidate/oracle each
pass 12/12, promoting exactly 11 previously `not-run` rows. Valid launcher,
persistent-context, server and connect requests remain unsupported.

Under the same final candidate-source SHA-256
`69f3083cfbffe7b51ba8ccc5228db0aabf00bc1769e82bb8e7717b909f9660cf`,
the pinned 15-key matrix was recaptured with stem
`playwright-core-{mode}-{suite}-20261010-preflight-regression-v1`:
Library candidate `b4a22082615d93d787f55b430c6536f3543613ebd7dd63d0feaa6f08543f7a77` /
`62a281227364577811180301608dd87204a84b50c6bd8214b4114fad6cd113c6`,
oracle `4a6e37f1246c1b969a695165df4e1768038be3ed2ef879448252afaa9db3b51c` /
`e1cab51a4dcf7e99e813bdd2bcae6c404cef866f8ff8917d67953109d26cfc2f`;
BiDi candidate `b8ab8539706bbe69f323482abd8673555486e558cd83b6b31a8da3d2b325906d` /
`12a8ab701b4abf857c7213d2a8fbd05d06ad968183ed400d56d02ae17757284d`,
oracle `ced4c4b3638e63a804cf64c3a9680e299a374b08f5d3a32996eaa3963bf5cbe4` /
`144ba4d80cf074ac9b992864230ed6d94eb7c5c3fced4a76e3602015be2536d3`.
All four source-bound sidecars independently verify; both lanes remain
11 expected/four skipped. The latest core442 48,477-row strict ledger
SHA-256 values are candidate
`f9d400c3aa5804fdb5375df3c4900fee0a83dec711e0ca76df126d7ed5cd380d`
(1,603 expected / six skipped / seven unexpected / 46,861 not run) and oracle
`449f2b33e4587794d85c2de89ea38281ac04c9ba56b5d3700f0486588d6ab7ea`
(1,306 expected / five skipped / six unexpected / 47,160 not run). The
classification audit remains SHA-256
`927498c7f312c32dfcaff5ee8b58b597f5af8f879f1a5cd5bcbb19341fdb4412`.

Python provenance schema-3 tooling checkpoint (2026-10-10): only exact pinned
v1.58.0 manifest-selected `BrowserType.executable_path` execution opts in.
Capture binds raw resolver environment, mode-specific package/cache root,
official `browsers.json` SHA/revisions, exact paths and existence with
pre/post/pre-write drift rejection. Other captures remain schema 2; all six
earlier ABI106 metadata reports remain schema 2 and unpromoted. Frozen
schema-2 capture tooling and verifier were archived byte-for-byte under
`.work/official-conformance/browser-registry-abi106.bhaA0F/tooling-schema2/`;
all six old sidecars passed verification with both current and archived
verifiers using that tooling root. Focused browser-free tests pass 66/66;
Ruff passes. Schema-3 verification is intentionally sensitive to current
environment and executable existence, not portable archival attestation
after installs or environment changes. No browser was launched.

Core post-`launchServer` BrowserType matrix regression (2026-10-10): the
unchanged 15-key manifest SHA-256 is
`de14b5a54017e9886da68ac69d5ad6575611a0463d1babf648a92d7f49e24486`.
Fresh result/provenance SHA-256 pairs under
`.work/official-conformance/results/` with suffix
`-20261010-launchserver-regression-v1` are Library candidate
`c912b13b2d99958446033dbb9a091dbfd0cfba2e26b30088ea1c7cfebab34332` /
`c563cf98f352b840b2bcb57b3e71c0995acf1d9bc3f1f9bfe6af273f92ca48db`,
Library oracle `dccccc807b51547fcdc5a901c49b9c77847e28198dd07105b8849508f94ef797` /
`e4dc431c093913582cdc1a69db5fb6c8e1765766532fe05c8294ebf27d6efaab`,
BiDi candidate `19ca8ca90fe95438411a5aa69c58d5109952fc58c057aba8af9d411e8ea1a9da` /
`dc2b7373b581447427570f7d2da0bfe7f85c48d8899f09f4d65031ff538ce65e`,
BiDi oracle `99e4e3fa15abb16a5f1ef4500aadd60947370bce65095280507cd922c036c8ea` /
`1769a3726542994edee5fe9865c568e27b803934e2bf41250adcd9e1fef520f9`.
All four source-bound sidecars independently verify; both lanes again account
for 11 expected / four skipped, no unexpected. The matrix candidate source
SHA `d6a1a8ed599c5f6993762e1f43112c50dba83f31a4e3923b926876d187c3c25e`
matches the new `launchServer` preflight report. The core440 ledger SHA-256
values are candidate `77506afa5d1e55dcf762d41e1f7e09a2a8aaef8d1884ee26ec50b757d10c9519`
and oracle `c12b227cbc066e31ce324d9eef3aad825b171b6e02622d4d6fe39dd956ef1940`;
only 15 result-file references changed from core439, and all outcomes remain
the same. The regenerated audit digest stays
`927498c7f312c32dfcaff5ee8b58b597f5af8f879f1a5cd5bcbb19341fdb4412`.

Core BrowserType `launchServer({channel: null})` validation checkpoint
(2026-10-10): the pinned exact-row manifest SHA-256 is
`6845bcc03f0e3e6e7753b07edce877338c6c3f56c7de95dee3e5f400e1862594`.
Candidate result/provenance SHA-256 is
`51ca33fd645dde07f63f4cdf5eab593a8a52fb368ea47d510fd38c551b4b06e9` /
`a338bd85248c436069f1264c58ae2de0fc904876190378f18e3bce0ae468db43`;
oracle is `e3c2aef58362932d6658e8739a8a30c1349dfb83eaa705868eadb6bc7a73d35e` /
`92d9a5ea964585a36850ac30adbaa3618b870ce760204474ee0a955d5d7f6970`.
Both source-bound reports verify and pass 1/1; no browser/server/native launch.
The Node facade reproduces only invalid-option preflight; valid launchServer
remains `PWRIGHT_UNSUPPORTED`. Node units 450/450 plus types/boundary pass.
The exact one-key core439 ledger SHA-256 values are candidate
`49b053267f0a9f1e4c10702457ddc35c34fbc4aeef43db7810b71b26be962c33`
(1,592 expected / six skipped / seven unexpected / 46,872 not run) and oracle
`138c163a8779a18d9fd0436c94163fbf84d50eb7d112128b6d4d2e3b8d02194c`
(1,295 expected / five skipped / six unexpected / 47,171 not run). The
classification-only audit digest remains
`927498c7f312c32dfcaff5ee8b58b597f5af8f879f1a5cd5bcbb19341fdb4412`.
Accumulated ledgers span candidate snapshots and are not a single-build
full-corpus pass.

Python ABI106 browser-registry diagnostic checkpoint (2026-10-10): the opt-in
pure Rust registry pins official Playwright v1.58.0 `browsers.json` SHA-256
`6b6e0538d6e46eef71d3e1a0dcb65f8f5ea7914033c8150529d8e207afb6030c`
and registry `index.js` SHA-256
`15c1a6174cc9d631ffcc908ed8d040f3f5ee543a77711a8062fa3037f7cda75f`.
The isolated pre-final-format ABI106 wheel SHA-256 is
`fe1637d8ccc957f6ab12040b7ab1280109f215e3c6c20a51107ad739ba683688`;
297/297 units and release feature, dependency, Ruff and strict Pyright gates
pass. Two later frozen-production-source wheels SHA-256
`d9b7c15db53b3cf0e19e9f68b52fb3645174b1c025da7c0a4cbc505de71d2e16` /
`22e75ee9502558fe70bbe59b898b7254f5c967d5b985195e4a33c00a6c1cca90`
have identical 16-entry normalized payloads; raw CycloneDX serial/timestamp
and dependent `RECORD` hash differ. Native library SHA-256 for all three is
`806f05c4e140d6e9762136ba43ee7973f2369d4345d7237eee996b5bd4327515`.
A post-build test-only Rust root-clamp regression passes, but no exact
post-test-source wheel was built while another task's large Rust build owned
memory. For unchanged official 12-row metadata manifest SHA-256
`65005ab446c31b0871e114d29a9699173ffd5f0539b50936743e4695858a1274`,
each candidate and oracle per-engine run passes two name rows and fails two
executable-path existence assertions; returned path strings match exactly,
but v1.58.0 browser binaries are absent. XML/provenance SHA-256 pairs under
`.work/official-conformance/browser-registry-abi106.bhaA0F/reports/` are:
candidate Chromium `fcdd739e791c74b824235b629400e849606d890dd51191973139e069086d4f62` /
`b6cb6d3644f698df28363680a9f77dc4a33025c408dd0f6706be3481cf93bacc`,
Firefox `deb7d42ab0732ef598f2c843605a1a80109a4e34a879dc40946c4b4526a91b13` /
`95780b8ec5d29eda5863d89b65db2e292c35e9991f19fafb8556f242e4192592`,
WebKit `25a3ac9ee7caad070d75fe84f192398f906768ec289ef362e4c5475aafad6f1c` /
`5c35fb85a0b16c0db0fa16520efd95a25fd1d5374f5e19704fcc06b627516b30`;
oracle Chromium `912f635b6bade64b5f06397baca6c3354d2c967ed9abaaf501de9fa38749d964` /
`969f65d7c7f908472fa3b4edcb437a5180597d1ce69582844f3c45931a0699db`,
Firefox `a5a7f1c64de644256b1bc56103dd86072eabe6632488ba94642bb7cc363737ee` /
`44754ff26fa28de3f9275e627c8734643bc12634c7e65ddb861ed5309be0020a`,
WebKit `5eca59f6c2812a37a37369ee59f9bffd507adede5b15d680fcb42b7aebb33ae1` /
`e79f7a7c3d876e6bb2e31686ead9a0910416d36cfde3de43e124e0e633fa81f0`.
All six schema-2 sidecars independently verify. The six red path rows are
diagnostic only, not strict-ledger promotions; capture-tool source binding
does not yet bind browser-cache environment or installed revisions.

Browserless BrowserType matrix and provenance checkpoint (2026-10-10):
`tests/official/playwright-core-browserless-browsertype-matrix-manifest.json`
SHA-256 is `de14b5a54017e9886da68ac69d5ad6575611a0463d1babf648a92d7f49e24486`.
The fresh source-bound Core v3 Library result/provenance SHA-256 pairs are
candidate `e1f620cc6ac0dfedf6afe5ccc75e837442679d6ee9c1129a754868c24c9d4caa` /
`a48fb2632fedc538472e30045ace2cad0e3434463398c512d13959bf602c1cbf`
and oracle `1758a5718c37e7c11d82ffdf4c3926e9ea93b2ef24572260fba663cfd42d79a5` /
`1da4baccccfad91f1aa2d4f2bd12e9d590ab62ddba209dd5466ccc0dc70b8039`.
The BiDi pairs are candidate
`e247ca98d818251b5ba5aa5d6e1d1c2a3366a67073495f2a9b23fa63c1db7b0b` /
`ec20a538de8d907b0b37aeba72fd98cf7ee45d0ee9055f139b9c8d27a5be0b64`
and oracle `e18245e4fc6a4df19eb1f7f88b6926c68e8d6586d369978d1196069097346252` /
`5e345fc2df3c462532df9a0f9a09c54b23cb20835f7a47cc4b01931118e48134`.
All four schema-2 sidecars verify against the frozen capture-tool sources;
both lanes account for the identical 15 keys as 11 expected/four skipped.
The exact 48,477-row strict ledger SHA-256 values are candidate
`9f3e4e2efea5126d04acc6508e4ab4bf9f58497582e4010a6ff2fcfcba5e68d6`
and oracle `f82dc473dffc1dbb5caf981742a3063853b3a1c5cc65199736ba4608800edc0f`.
`scripts/audit_playwright_official_invocations.py` now gives every one of
6,148 product-infrastructure invocations an explicit source/role reason while
preserving all classifications and outcomes; the v2 audit ledger SHA-256 is
`927498c7f312c32dfcaff5ee8b58b597f5af8f879f1a5cd5bcbb19341fdb4412`.
These tags are not automatic exclusions.

The Python Firefox/WebKit async/sync `BrowserType.name` result/provenance
SHA-256 pairs are candidate Firefox
`c25bee4eb11d5c871fd6bae4037f6192384ed4bf1b433f544f21ad0b8a4e855b` /
`ba6cd61571fb92843da95a1384e91746a97ea96b5ed6c55bcda8fd64dccbb549`,
candidate WebKit `c0425f3a5a8ddb6342031278be6394ba14c42a7346073c819c79695e7f04878c` /
`b07942158ddd474f7c3d499a18ec7d08215fba6d4bbfd08b2dc6b3fd159cc338`,
oracle Firefox `305cbfeb84451d288d432a3edb53f685a2bafe69845bd598b9c8c33568d5aa2e` /
`d49bd65e90226c593c608b51ac0e86e1d4876cb2e49d945f1613dd366af65af3`,
and oracle WebKit `288c561e3d9b44bf0dca4aec337afdb5dc9647d164d09e220dd4b23a96e3936d` /
`140eacf45beb596e79823ccd9cdaa6a7e6557cae00838f854fb9ccb0e18e4230`.
Only these four new passed rows join the 6,526-row ledgers: candidate SHA-256
`6afce40303ca829b69e71caa3274ef60b028a7fba4225c19ad7cfc974fa905b6`,
oracle `62b38b62de425e89b4d88bb3e075984d24699cd933b59c3b7ca77ed0144a1759`.
The separate 12-row metadata diagnostic manifest SHA-256 is
`65005ab446c31b0871e114d29a9699173ffd5f0539b50936743e4695858a1274`;
its six `executable_path` rows remain red and unpromoted. New Core/Python
capture runners and verifiers emit/validate schema-2 fixed tooling-source
hashes, while schema-1 reports remain `legacy-unbound`. The new schema does
not bind browser-cache environment, installed revisions, or process execution.

Dispatch-event documentation checkpoint: the complete 17-case Core manifest
SHA-256 is `4bb9714f113992367330d0578f2862b0ced17d766c69310faaca655310b06137`;
the 12-case Python async manifest SHA-256 is
`acbe71f2b9922e5040b17c2bcbcb3946b9096964f314de0f63ddc909353d0128`.
The pinned Python tag has no sync `test_dispatch_event.py`. The distinct Node
artifact SHA-256 is
`ff8e5db65ab83dca3e7f53fb22b493c81167ece9f7afcdb40e1424d2327ef34d`.
Node unit/native/type gates and freshly installed Python ABI105 wheel/unit
gates pass. The ABI105 wheel v3 SHA-256 is
`3892b19253d7ba4b050c14c437e26c2508eb8d9294d9e1a6a7debb3d3a9a0d4d`;
installed units pass 287/287. The official Core oracle v1 and candidate v2
each classify the selected 17 as 14 expected / three unexpected, with identical
per-spec outcomes. Oracle report/provenance SHA-256 is
`e5cc8d623a30203ebf336790c98fe3d19ca241de4d4196a6c21959c612fc627f` /
`c1b0486ccefc1a170d252678a1ee9306bb3495f015d4666807aede7253e7e95d`;
candidate v2 is
`c62ed0843bc3e0cbe225bd2277d550eac6bb7d706959605af783023d2414e1b9` /
`88ce1497a6ec6d6d53c743778b0d2baf0e02bafc805c188641ec3dff8afda478`.
After the HTTP v2 dispatch merge, strict 48,477-row Core ledger SHA-256 was candidate
`c75186cd78e28259a48247e553b566a514944f9470bc103e98d57b8de6411f4b`
(1,579 expected / two skipped / seven unexpected / 46,889 not run) and
oracle `65ec860c18b4b5e3e82e96f87167eb0fd36f170ca7f0c1e1e2f51b6886298f39`
(1,282 expected / one skipped / six unexpected / 47,188 not run).

The official Python async oracle and candidate each pass 12/12. Oracle
XML/provenance SHA-256 is
`45356b260b29ee850e5d32b38d0db94c07aea8332ddb27bc99c94490a33bcb1f` /
`3e8cfaa4e381ea3b8d5febeab9b5d65852ad123ea2a839af1c85c9ca25c469fd`;
candidate is
`71c1be307ec418904fadb30bb93edff99200ea86cb144936aaed5a35da233b36` /
`b742d93ac50ab46c1c93a0190719f6563c737aff2cfeeadff7ea86efbe8ac83f`.
After the async dispatch merge, strict 6,526-row Python ledger SHA-256 was candidate
`c5a31bf3be6d848b804a910a1de6409b09ded8fc8c361db8b1a12ad60c9039de`
(1,238 passed / eight skipped / four failed / 5,276 not run) and oracle
`bcc3d23b593cc99fc1d753a6350e89f16b06dea3047afd495ee55fedffafcdef`
(792 passed / seven skipped / four failed / 5,723 not run). Core's HTTP
fixture adapter collapses `PREFIX` and `CROSS_PROCESS_PREFIX` to one origin;
the cross-origin-titled pass therefore does not validate distinct origins.
The three shared sensor failures occur on an insecure HTTP fixture. A secure
fixture lane now has a staged trusted-Tailscale-TLS dual-port manifest/preload
(SHA-256
`efc2f9f131c245baf7ddca262cd08ca3b5cee3d9f8d3d7a4248cbe55a5a03de2` /
`f3243c9f73c33a07a8eb62428505bd612f6248384f02ded14ffe2d42d219b8c2`),
but **no secure browser execution**: the earlier trailing-dot TLS preflight
failed before tests and shared Chrome is occupied by another task. Different
ports give distinct origins, not necessarily sites or processes. Candidate
Core v2 is historical; a post-lifecycle-patch candidate v3 again records
14 expected / three unexpected, identical per spec to the HTTP oracle.
Candidate v3 report/provenance SHA-256 is
`693db5b080912f67831f382c472bb17e7387d340ab33489307c5e23273b69522` /
`aca608efc75caa1497e07b6635f7b95f6aec968a6f06f82d3bf942d071df3b03`,
and strict candidate ledger SHA-256 is
`cc21a501b7a12e575c870ff1f74eb0a3e3db89b9b8b0a0aa438bf129bc2f4560`.
Node package units pass 442/442. V3 provenance verified at capture, but
the runner changed afterward for the secure lane; current-harness
re-verification is not claimed. Core and Python fresh-run entry points now
reject occupied result/sidecar destinations before process launch, with
Python isolated resume handled explicitly; the secure Core validator pins
all 17 distinct spec IDs, not only a count. Historical sidecars do not hash
runner/helper source, so this does not retroactively prove a fresh run.
A separate browserless two-row `chromium-library` BrowserType manifest
(SHA-256 `6845bcc03f0e3e6e7753b07edce877338c6c3f56c7de95dee3e5f400e1862594`)
records `browserType.name` expected in both lanes, and invalid-channel
`launchServer` expected in the oracle but unexpected in the candidate.
The exact 48,477-row strict ledgers now hash to candidate
`556eafa00527dbf41cd7d494dcb2f2a9423272581af614ab6ea1a7f09f93faa7`
(1,580 expected / two skipped / eight unexpected / 46,887 not run) and
oracle `1b08b44e3351da8442652333694aef39a0ac7ab07ff866dea7d3e5b5f10fdb3c`
(1,284 expected / one skipped / six unexpected / 47,186 not run).
Lifecycle cleanup remains unresolved after a scoped 17-lease retention report.
The routing entries below cover the affected design, conformance and package
README documents; neither full-corpus parity nor complete semantic parity is
claimed. A separate formal browserless Python sync-lifecycle execution mode
uses manifest SHA-256
`dbad4ede06a7e3ae07d994a75e02a8868bc82191145e6c3746a880eb2fa2081f`
and same-thread asyncio wrapper SHA-256
`7a089d2a16cbeec46926374571e96cf72cd8d3a513c76b0281c988be470e63c7`.
Candidate ABI105-v3 wheel and oracle each pass 6/6; candidate
JUnit/provenance SHA-256 is
`6b08f1df918824cc60e95d9231ba4c6c84f2b0eb5bd81639e9890e88d4c85880` /
`085841dbbf0993c31a4b4c9942770e243115d03ecbe975b6c3b6cc46503c2e1d`,
oracle is
`c1f3a1c464d78c225cca5cd10568017d1b6c0a27d9d10cfc7dd0810fbfddb2cd` /
`0c191f41c487998880d90608060c5fd3c8fc68f5e84612dab45e6cd956a01326`.
The earlier ordinary-pytest 3-pass/3-fail paired baseline is preserved.
The exact six-key formal-loop merge advances the candidate ledger to 1,240
passed / eight skipped / four failed / 5,274 not run (SHA-256
`e7cd534dbd40309ed7d258431face26a1b40d0f2c14c27834b493ec3fff5695e`)
and oracle to 796 passed / seven skipped / four failed / 5,719 not run
(SHA-256 `07cf927aa0a241b2075d9ec0f26c88e58daff0bf50e3d6cd646588ba056fc1d8`).
The exact two-row Python async/sync browserless `BrowserType.name` manifest
SHA-256 is `e34e466352e5bb94304ba5dfa21655255877f6330b5ba236313c99dc97e0dac8`.
Candidate/oracle each pass 2/2 with independently verified XML/provenance
SHA-256 pairs
`de0479b59b4650342c82006b13406470241390cc25ebc34cab6665e0d1d142f2` /
`0703d2ff4b7ec441f1eec22553a10ac1de1dd5e43cd8fd8edf0a92bd8159a9e2`
and `9ed208f5c431d5b689e11eecdf7516695e961db965d1ae1545920ee4a6047611` /
`3bb3b3a9cfd6cd02f4c91b7f6a754e6ba94682c98777511dbdfde79df37df21d`.
The source checker permits this exact source/fixture-hash-pinned closure,
not other browserless fixture-argument cases. Only Tailscale-bound HTTP/HTTPS
fixture servers ran; no Chrome/CDP. Exactly two keys advance 6,526-row
strict ledgers to candidate 1,242 passed / eight skipped / four failed /
5,272 not run (SHA-256
`ed31f1074ab675e75b8828e27a141a858144e8f85e408b9955d36fd1f2c6e4e8`)
and oracle 798 passed / seven skipped / four failed / 5,717 not run
(SHA-256 `2c02dffe0d102fe0d2dd660b30318097e455018c074140f73db5707067511852`).

Page check/uncheck NODE-434 pins all 18 unchanged Python v1.58.0 Chromium
async/sync cases in `packages/pwright-python/tests/official/page-check-manifest.json`
(SHA-256 `7e9b34f4ed0b0c193e213375ad2fc8dad6fbd13b622cda3f77c5d2ecf51809ae`).
Candidate v1 is 0/18 at absent Page methods while oracle passes 18/18;
their verified report/provenance SHA-256 pairs are
`e7bd53be1ebb5598ad2c32b81b48815ed45938c109d457436f8b76fcae46774c` /
`c7f5a90298b02b16bed002db9765289a53a41f7d0542d686ec3cdbaf37b4028d`
and
`ed02b29dc95adc83c2f08d3584fa8b4a041dbfd2b0385061bca8504e10ece72e` /
`f12a1290f16071659a873fabe7937162e41d26c386802569c5be3d593f74c0ac`.
The red candidate 6,526-row ledger is 1,208 passed / eight skipped / 22
failed / 5,288 not run (SHA-256
`7181ca1d518c24c69f9db6a0c9a85d85fd6384feca748139b9287216b9790e70`).
The oracle was rebased by 25 pristine example IDs to the profiled inventory
(SHA-256 `a937e7f5400ed2a0a565673893f2c132aaf7f9be29ff0916078a6d1740bdb045`)
before its 18-key merge: 780 passed / seven skipped / four failed / 5,735
not run (SHA-256 `1bc6a8b0dbaa1a79471e60d4c7576e0512043c737e2f70dea80db56fe13cea47`).
The repaired `abi3-py39` wheel SHA-256 is
`85913b81bcc9d9c65533554b4b6f7f9ca423216ea08f43d682bc1244b2d5d46b`;
installed units pass 271/271. Final candidate v2 passes 18/18 with verified
report/provenance SHA-256 pair
`2bc0177b7a6cc8f1ca7c1d4a6e20acfd728dbf819224ee88e69a7f07ad633e14` /
`8c374ce922b748f613d82980ba3aa6be3eb68cdcde76071d19af7e36c8a44573`.
Its strict ledger is 1,226 passed / eight skipped / four failed / 5,288 not
run, SHA-256 `82c8161800ec3acd18938d0f9d0ba80edf76ce66b0918d563a6923b93ef6ee74`.
Python Page facades delegate to the existing Rust-owned Locator checked-state
action.

At the 2026-10-10T05:01Z source snapshot, two final-source `abi3-py39` wheels
hash to A
`11d0a7f348145e8c4e2150a78dc3afc5638571cc271e938558f62093090a4adf`
and B
`0086c99813ee9f8dad7804c32235f272473d5de3d483fa24f36a2b681a793474`.
Normalized payloads reproduce apart from CycloneDX `serialNumber`/timestamp
and the dependent SBOM hash in `RECORD`; a fresh installed wheel passes ABI104
and 271/271 units. The official 18/18 report used the earlier `85913b81...`
wheel. These 05:01Z builds have no official browser rerun because a foreign
nonblank tab occupied shared CDP Chrome.

The matching Core tranche pins 17 unchanged `chromium-page` cases in
`tests/official/playwright-core-page-check-manifest.json` (SHA-256
`41787c922fd4690cc96cbf778e918132af57c778321afd7c470acec2ff8432b5`).
Candidate v1 is 0/17 with 17 unexpected failures (16 missing Page methods and one
missing `ElementHandle.boundingBox()`); its verified report/provenance hashes
are `8ea21fc35c11196cabf5ee8a51ae2faac34ac97ce7ede4def10cc82a43bd31c1` /
`854f70c28119023ab35abef4ab165a0351cb1ffd63896613e5b136a71533955c`.
That 48,477-row candidate ledger is 1,548 expected / two skipped / 21
unexpected / 46,906 not run, SHA-256
`c9990d41d41b1f622d5ce99e8dbb0796e30d6b0c8ba6cd2678e463fb1e0524d1`.
Oracle v1 passes 17/17 with verified report/provenance hashes
`16b0b069164778d38e91b8979a0b7104aa1f8f2736f11abf32c1f69209689547` /
`f8dde0aa3d09f2132e794f8e86e5f1e00b40c201777409afe29f1f97107701d3`;
its strict ledger is 1,268 expected / one skipped / three unexpected /
47,205 not run, SHA-256
`d4a2878876e885d77297e47fe3f6bdaa91e72a4bc735448305c0cb205f8ea0a2`.
The original baseline `.node` remains
`f229cbbc36691afaee8b8ba65d6dfffad2b3a8f2576647a1e186d5e558215384`.
The new `official-conformance-page-check` artifact is
`a991b225d0a3dece24d0a457b05898e6aacb8f6217eb9d79f5d710c33ea30531`;
final candidate v2 passes 17/17 with verified report/provenance hashes
`58a5de733c8d052f64919a5ad6a738672ca059f72e787ee0a54a14b321d19375` /
`72895a0bdd0432f5d2f80877a98a98f47550d0caf0fbd56b06344f5b617d6a41`.
Its strict 48,477-row ledger is 1,565 expected / two skipped / four
unexpected / 46,906 not run, SHA-256
`f3176185ef59fc45cc49228c3bec688dcea6c935ab2451f21e138962156cdad8`;
both classification audits retain SHA-256
`32520af75e4ac14352a9060e6f7156900bb52c4a3d99fe927311ba7490328af0`.
Node 424/424, types, dependency boundaries, selected host release 617 passed /
14 ignored, and strict addon Clippy pass. Node Page actions delegate to
opt-in Locator/Rust; retained-Handle bounding-box geometry is not proven
equivalent to Chromium `DOM.getBoxModel`. Full Python/Core corpora remain open.

Python BrowserContext storage-state adds
`packages/pwright-python/tests/official/browsercontext-storage-state-manifest.json`,
pinning all nine Chromium cases in the unchanged async and sync v1.58.0
files. Candidate v1 fails nine because `BrowserContext.storage_state()` and
`Browser.new_context(storage_state=...)` are not implemented. The independent
oracle passes seven; its two IndexedDB assertions compare upstream's hardcoded
`localhost` to the Tailscale fixture's MagicDNS origin. The opt-in Rust owner,
Python facade and repaired wheel now rerun the exact nine with the same
seven passes and two fixture-address failures as the oracle. Verified
provenance and strict nine-key merges preserve the 6,526-row ledgers:
candidate 1,208 passed / eight skipped / two failed / 5,308 not run; oracle
745 passed / seven skipped / two failed / 5,772 not run. A first repaired-wheel
run timed out after a task Chrome tab close, while a one-case control and
full rerun of that same wheel passed; preserve the anomalous raw artifact and
its unproven cause. Exact hashes and remaining storage/lifecycle gaps are in
`docs/design-tasks-v2.md`. Full Python/Core corpora remain open.

The existing 19-case Chromium BrowserContext add-cookies manifest was also
rerun in both lanes with the repaired wheel: each has 17 passes and the same
two hardcoded-`localhost` fixture-address failures. Verified exact merges
advance the oracle ledger by 17 rows and record the two shared failures in
both 6,526-row ledgers. Evidence hashes are in `docs/design-tasks-v2.md`.

Core Page network-event NODE-433 adds
`tests/official/playwright-core-page-event-network-manifest.json`, selecting
the complete unchanged seven-case `chromium-page` file without a title filter.
The preserved candidate v1 baseline has one expected pass, five unexpected
failures and one upstream Chromium skip; the independent oracle v1 baseline
has six expected passes and the same skip. Repaired v2 and post-GC v3
candidate/oracle pairs each pass six with the same skip and zero unexpected
outcomes. All provenance sidecars verify; strict merges touch only the seven
manifest keys, and both
48,477-row classification audits retain their hash. Exact result, provenance
and ledger hashes are recorded in `docs/design-tasks-v2.md`. Selected host
v2 release, focused host, Node package, types, native load, four standalone
profiles, boundaries, Clippy, formatting, diff and memory checks pass. The v3
focused host (28/28), selected host release union (839 passed, 22 ignored),
Node package (414/414), types, native-load, boundary, four standalone profile
build/load and strict selected Clippy gates pass; final memory/diff checks
remain open. The
implementation reuses opt-in `request-history`; no separately named Request-
lifecycle profile is released. Rust retains a bounded one-way predecessor
owner and Node reconstructs a collected wrapper on demand; live-browser GC
stress and exhaustive AAA coverage remain open. Full Core/Python corpora remain
open; the pinned
nine-case Python storage-state baseline and paired rerun are recorded above.

Core Page response-wait NODE-432 adds
`tests/official/playwright-core-page-wait-for-response-manifest.json`, selecting
all nine `chromium-page` cases from the unchanged upstream file with no title
filter. Candidate v1 1/9 and candidate v2 7/9 are preserved; final candidate
v3 and oracle v3 have the same eight expected outcomes plus one shared
Chrome-154 cached-image unexpected result. Both provenance sidecars verify,
strict merges touch only the nine manifest keys, and both 48,477-row audits
retain their classification hash. Full-corpus execution remains open.

Core Page request-wait NODE-431 adds
`tests/official/playwright-core-page-wait-for-request-manifest.json`, selecting
all eight `chromium-page` cases from the unchanged upstream file with no title
filter. Candidate v1 4/8 and oracle v1 8/8 are preserved; final candidate v3
and oracle v2 pass 8/8 with verified provenance. The candidate preload's
Context init script reproduces upstream per-document UtilityScript builtins and
is explicitly test-only. Strict merges touch only the eight manifest keys;
both 48,477-row audits retain their classification hash. Full-corpus execution
remains open.

Core Request post-data NODE-430 adds
`tests/official/playwright-core-network-post-data-manifest.json`, selecting the
complete unchanged six-case Chromium Page file with no title filter. The
runtime uses independently selectable `request-events` and
`request-post-data-json` profiles over the bounded Rust network observer and
parser. The manifest preserves two upstream Chromium expected-failure cases;
their attempt errors must be inspected rather than counted as automatic
candidate compatibility.

Final candidate and oracle each record six expected outcomes with matching
real errors for the two annotated cases. Both provenance sidecars verify;
strict ledger diffs touch the same six rows and both 48,477-row classification
audits retain their prior hash. Full-corpus execution remains open.

Core extra HTTP headers NODE-429 adds
`tests/official/playwright-core-extra-http-headers-manifest.json`, covering the
complete two-case Chromium Context file and five-case Chromium Page file with
no title filter. The paired final reports contain six expected outcomes and
one identical candidate/oracle unexpected pass under Chrome 154. The new
`response-request` profile exposes a Rust request-history snapshot through the
shared Request facade; it is independently selectable from `extra-headers`.
All four provenance sidecars verify, exact ledger diffs touch only seven target
keys, and both 48,477-row classification audits retain their prior hash.

Core Touchscreen NODE-428 completes the unchanged nine-case Chromium Library
`tap.spec.ts` file in candidate and independent oracle lanes. Raw Page touch
input and selector-driven tap remain independently selectable Rust
compositions; Node owns no browser behavior. Artifact `5fb122e...b91` and both
provenance sidecars verify. Exactly the same nine not-run rows become expected:
candidate reaches 1,521 expected (`d084416...8bac`) and oracle reaches 1,219
expected (`1589687...8b0`); full-corpus completion remains open.

Core Touch NODE-427 completes the unchanged Chromium Library
`locator-dispatchevent-touch.spec.ts` row 1/1 in candidate and oracle lanes.
The Rust static event program materializes native `Touch` objects; the existing
bounded Context device registry is now selected through an independent addon
feature and thin `hasTouch` port. Artifact `69a91ea...de1` and both provenance
sidecars verify. Exactly one not-run row becomes expected in each ledger:
candidate reaches 1,512 expected (`7d0d844...1260`) and oracle reaches 1,210
expected (`fb3aef0...06ed`); full-corpus completion remains open.

Core Locator NODE-426 completes the final 24-case `locator-query.spec.ts`
file. Oracle is 24/24 and honest candidate red evidence is 12/24; typed regex
filters, ordered exact-string arrays, structural/deferred FrameLocator
composition and the bounded private before-query rendezvous remain Rust-owned
behind thin Node ports. A 23/24 intermediate isolates the final issue to error
timing. Final candidate/oracle pass 24/24 with artifact `c12153d...d1bc`.
Exactly 24 not-run rows become expected in each ledger: candidate reaches
1,511 expected / one skipped / two unexpected / 46,963 not run
(`083773b...7d1`), and oracle reaches 1,209 expected / one unexpected / 47,267
not run (`6961c30...eaf`). Both audits hash to `32520af...8af0`; all 123
Locator Chromium cases are now accounted for.

NODE-425 completes the 16-case second Locator miscellaneous file. Oracle is
16/16 and honest candidate red evidence is 12/16; thin Node ports now expose
the existing Rust viewport, Locator screenshot and DOM selection leaves. A
15/16 intermediate run isolates the final issue to the keyboard-specific
unknown-key mapper. Final candidate/oracle pass 16/16 with artifact
`3edcb5c...71db`. Exactly 16 not-run rows become expected in each ledger:
candidate reaches 1,487 expected / one skipped / two unexpected / 46,987 not
run (`8c78583...c841`), and oracle reaches 1,185 expected / one unexpected /
47,291 not run (`7d75835...4ef9`). Both audits hash to `32520af...8af0`; 24
Locator Chromium cases remain unexecuted.

NODE-424 completes the 13-case first Locator miscellaneous file. Corrected
oracle is 13/13 and honest candidate red evidence is 11/13; an earlier all-red
oracle run is retained but excluded because Chrome rejected setup before test
behavior. The existing Rust `locator-dom-actions` leaf now has a thin Node
`dispatchEvent` port, while the existing `file-input` leaf is composed only
into the explicit official profile. Final candidate/oracle pass 13/13 with
artifact `356f84b...c39e`. Exactly 13 not-run rows become expected in each
ledger: candidate reaches 1,471 expected / one skipped / two unexpected /
47,003 not run (`1afb6c7...ecaa`), and oracle reaches 1,169 expected / one
unexpected / 47,307 not run (`1a73f91...ccb9`). Both audits hash to
`32520af...8af0`; 40 Locator Chromium cases remain unexecuted.

NODE-423 completes the 22-case Locator FrameLocator file. Corrected oracle is
22/22 and honest candidate red evidence is 3/22; an earlier all-red oracle run
is retained but excluded because Chrome rejected setup before test behavior.
Thin Node FrameLocator composition now enters a lazy Rust recipe; Rust owns
frame replacement, retries, strictness, missing-frame reads, owner diagnostics
and terminal CSS `:has-text()`. A separate Rust `Route.fulfill()` leaf unblocks
the routed-frame fixture. Final candidate/oracle pass 22/22 with artifact
`fbc48ff...1382b`. Exactly 22 not-run rows become expected in each ledger:
candidate reaches 1,458 expected / one skipped / two unexpected / 47,016 not
run (`176cc90...a3b`), and oracle reaches 1,156 expected / one unexpected /
47,320 not run (`6106eae...546`). Both audits hash to `32520af...8af0`;
53 Locator Chromium cases remain unexecuted.

NODE-422 completes the 22-case Locator convenience file. Candidate red
evidence is 13/22 against oracle 22/22; thin Node descriptions and
`evaluateHandle` now compose with strict Rust Element resolution, bounded
deferred conversion, final-Handle-only publication and failure-safe temporary
cleanup. Final candidate/oracle pass 22/22 with artifact
`23f0bf4...390f05`. Exactly 22 not-run rows become expected in each ledger:
candidate reaches 1,436 expected / one skipped / two unexpected / 47,038 not
run (`6db265c...b8df`), and oracle reaches 1,134 expected / one unexpected /
47,342 not run (`61f077a...862d`). Seventy-five Locator Chromium cases remain
unexecuted.

NODE-421 pins and executes five complete unchanged Locator files: evaluate 6,
list 1, highlight 1, click 5 and is-visible 8. Candidate and oracle pass all
21 tests with verified provenance and no implementation or artifact change.
The strict candidate merge refreshes exactly 21 expected rows and remains at
1,414 expected / one skipped / two unexpected / 47,060 not run
(`06e0444...63546`). The oracle promotes exactly the same 21 rows and reaches
1,112 expected / one unexpected / 47,364 not run (`a813017...58324`). Fixed
manifests now inventory all 123 Chromium tests across the 11 Locator files;
execution of the remaining files is still open.

NODE-420 completes all five unchanged Chromium Page invocations in
`locator-element-handle.spec.ts`. Candidate red evidence is 0/5 against oracle
5/5. Thin Node `Locator.elementHandle(s)` methods now enter a bounded Rust
selector program that composes typed parents with XPath descendants, preserves
strict waiting for one Handle and one-shot ordered batch semantics for many.
Final artifact SHA-256 is
`8e704ccbf94524d85424914ea6aef548f29871e79437c84d034223ed80a6f00a`;
candidate/oracle pass 5/5 with verified report SHA-256 values
`7dfc27e89e46c0e3c85e027ecfc6b00623359e9d743e6cc931ca74ba7ae71a49`
and `0f1655db8726aabcbf7e222b9b1afb0c4b76527c36402c598e7771d2d045ae0d`.
Exactly five not-run rows become expected. Candidate reaches 1,414 expected /
one skipped / two unexpected / 47,060 not run (`6fa68d3...9db39`); oracle
reaches 1,091 expected / one unexpected / 47,385 not run
(`6da3adc...83d6e`). Full corpus execution remains open.

NODE-419 implements exact-owner ElementHandle attribute, HTML, input-value and
visibility/state reads in the optional Rust DOM capability. Page and Frame
convenience/actions remain thin Locator delegates. Artifact SHA-256
`6290125fb3f659e07c506f51a8248102175854b87237ac5262386a764e978218`
passes the complete unchanged official file 12/15; result SHA-256 is
`98fa228cd117457ca23c2fd7f52868483ebc81caf6b9d0e268dfa6626c2e87a9`.
Its three reds are the two rich previews and one `nth=0` chain.

Current source adds typed and registered Rust `nth=<i32>` composition and an
asynchronous Rust-owned rich-preview read over the retained Node. Node only
caches the bounded result. After the unrelated Page exited without
intervention, final artifact SHA-256
`02475436552ed68237548b0c4d5840993fde0a1db5abdea21e875cfe778db47f`
passes the complete unchanged file 15/15. The verified report SHA-256 is
`eee2d882cac73ba18522c244ee8706fadd2adb9aa4387818a6c0e15a6e214b68`.
A strict 15-row merge reaches 1,409 expected / one skipped / two unexpected /
47,065 not run, ledger SHA-256
`b5532ffbcd2819662585b698b766bb3c4e9a434a8d4540f4b790a482eedc5940`.
Host release units pass 609 with 14 explicit live-browser ignores; package
units pass 346/346, with strict TypeScript, native load, dependency boundary,
release check and full feature-union Clippy green.

NODE-418 records full unchanged ElementHandle query-selector 7/7 and
eval-on-selector 6/6 candidate/oracle passes plus the complete convenience
candidate 5/15 versus oracle 15/15 red baseline. All 28 rows are represented in
the 48,477-record ledgers. Candidate ledger SHA-256 is
`5c1cd26cc8a8bf22bf57436627de682344245b8eba68ca61dbf1e5d3506c5766`;
oracle ledger SHA-256 is
`8e87e050ba9b088b849d8296b19bed0856a92fce99c82ec2214a5aa28fc1add7`.

NODE-417 completes the full 17-row Chromium `queryselector.spec.ts` tranche.
The initial candidate passed 10/17 and failed only XPath forms; Rust now keeps
XPath as a tagged browser selector plan, compiles explicit and automatic XPath
plus mixed `>>` chains, and publishes Page/Frame `$$` through one Rust-owned
ordered Handle batch. Candidate and oracle pass 17/17 with verified
provenance. Exactly 17 rows advance the candidate ledger to 1,396 expected /
one skipped / two unexpected / 47,078 not run, SHA-256
`675ed26f79d073b8fe1511b54b496065779313e99f0eba100fec632531a58a38`.
The oracle ledger reaches 1,073 expected / one unexpected / 47,403 not run,
SHA-256
`fdd3dc71b4560a66c04336a2cf68a20bd00a8d63a69d940bfcac1c81a9f2d950`.
Full corpus execution remains open.

NODE-416 replaces the Page/Frame `$` count/read split with one Rust-owned
optional ElementHandle query. Exact null publishes no Handle, positive Nodes
use the bounded creation/adoption owner and non-Nodes are compensated. The
remaining five Chromium selector-registration rows pass 5/5 in both candidate
and oracle lanes with verified provenance. A strict five-row diff advances the
candidate ledger to 1,379 expected / one skipped / two unexpected / 47,095 not
run, SHA-256
`8be091c0356f65de9a05b89b5a826d07ca6090342884fabb8296128a34ee8ea6`.
The independent oracle ledger reaches 1,056 expected / one unexpected /
47,420 not run, SHA-256
`cb8be736213310b13a8f271913449e5ed693aea7d9b8eda1095f5ddb59e29418`.
The new 17-row query-selector manifest is ready, but its run safely refused an
unrelated nonblank Chrome Page and created no result.

NODE-415 adds Rust-owned positive registered-selector ElementHandle publication,
including utility-to-main-world adoption and bounded failure compensation. It
also exposes the existing Rust atomic `Browser.newPage()` implicit-Context owner
to Node for no-options / `baseURL` / `acceptDownloads: false` calls; download
acceptance remains explicitly unselected. Local package, type, native-load,
boundary, Clippy and release gates pass, and browserless official selector
regressions pass 9/9 across library and BiDi projects with verified provenance.
Page/Frame `$` also derives a typed `first()` recipe before native Handle
publication, fixing the deterministic multiple-match strictness discrepancy;
public selector diagnostics retain their official text, and package units are
now 329/329. The remaining count-to-Handle DOM race is still
an explicit atomic-query gap.
The rebuilt official artifact SHA-256 is
`6196f520808b3ceb2059ff2a88cfa5fc85faa90ee34d728f25d4c9336d072dfe`.
Fresh library and BiDi result SHA-256 values for that exact artifact are
`7906d3150e173216c4aa042403964ded687e13b1142e2c0894087458d5914fb4`
and `797c48a2daf8bb0a13ee9a55ed4603b5df6c160cc1d9502549f7d076ca593631`.
No ledger rows advance: the
official browser-backed run refused to start while an unrelated nonblank user
Page occupied dedicated Chrome, and that Page was neither reused nor closed.

NODE-414 adds the first paired browser-backed proof for the Node registered
selector capability. Candidate initially failed by treating `tag=DIV` as CSS;
the repaired candidate and Playwright oracle pass the same Chromium library
row 1/1 with verified provenance. Rust now owns non-strict `$eval` and
collection `$$eval` selector execution. Exactly one row advances the Core
ledger to 1,374 expected / one skipped / two unexpected / 47,100 not run,
SHA-256
`f363803e73aeae00ca2226c76bd5f5b53f74d9e947fc7c65725bb19198f4a465`.
Positive custom `page.$` Handles and the remaining selector rows are explicit
follow-up work.

NODE-413 adds a separately packaged Node `selector-registration` capability
over the existing bounded Rust registry. The initial candidate failed 0/9
browserless official library/BiDi instances while the oracle passed 9/9; the
repaired release artifact and oracle both pass 9/9 with four verified
provenance sidecars. Exactly those nine rows advance the Core ledger to 1,373
expected / one skipped / two unexpected / 47,101 not run, SHA-256
`60b1c6f95b95f7bddb2539aacf47c02496dd8e8b41238ab731f7bb63c829a3ab`.
The opt-in adds no Pwright package dependency and does not change the native
ABI. Browser-backed custom selector execution remains an explicit next slice.

NODE-411 extends the browserless launcher-metadata proof to every remaining
official BiDi configuration project. The same pinned `errors` and `devices`
assertions pass 12/12 in both candidate and Playwright 1.58.0 oracle across
BiDi Chromium, Chrome canary/stable and Mozilla Firefox regular/beta/nightly.
No browser is started or connected. Both provenance sidecars verify; report
SHA-256 values are
`0fe174854e824de943effc833a670790a9319c73dc3ccee76fec3d89d1e5f2d3`
and `f47172abcb9f91afebf86d368bb4855649fc5803d141fb0e360cffaefcccb83b`.
A strict diff promotes exactly the remaining 12 metadata rows, bringing the
Core ledger to 1,364 expected / one skipped / two unexpected / 47,110 not run,
SHA-256
`87d6753547bad1f5f21e84952a2a8ea7af56cc75921e0382ec1ea7486cd1245d`.
This is additional execution evidence for NODE-408's static exports; package
code, Rust ABI, native artifact and resource boundaries do not change.

NODE-410 closes the sync `BrowserContext.expect_page()` facade discrepancy
without changing ABI104/90, the native artifact or any Rust resource owner.
The initial installed-wheel candidate passed 7/9 selected BrowserContext event
rows and failed only the two popup rows that call the absent facade; the
Playwright 1.58.0 oracle passed 9/9. Async now exposes the existing bounded
Context `page` lifecycle waiter directly, and sync wraps both predicate input
and result with the canonical Page identity. Two AAA units cover the boundary.
The repaired installed wheel passes 252/252 units, Ruff and strict Pyright;
two auditwheel-repaired builds have normalized-identical payloads. Candidate
and oracle pass the same 9/9 rows with shared case-ID SHA-256
`ba6df80c7d4bd8b2a5d10980281a76df4c4b3649b491bb8f7816e4fbe2419487`
and verified report SHA-256 values
`cc5cbb5c3fea0801bde4e4be6eb49aa777fc3970278a90ad8f2a2e492144ef2a`
and `a0cf4a49b4c33f9cce3fbacae743b06ea9e17478fd7c935af3cbc11dd3058984`.
A strict diff promotes exactly nine rows, reaching 1,201 passed / eight
skipped / 5,317 not run with ledger SHA-256
`554cd9e63214f3b920489849cabadfe8da1559d0719705399c68f65bf3ddbaee`.

NODE-409 reuses the unchanged ABI104/90 wheel and existing Rust request-timing
owner. The complete four-row sync Chromium diagnostic passes 4/4 in the
candidate; the Playwright 1.58.0 oracle passes SSL and subresource but fails
ordinary navigation and redirect because Chrome 154 reports
`domainLookupStart=0`, while the old test accepts only `-1` or a positive
value. The verified full diagnostic report SHA-256 values are
`e76315db5a762729d44de3efce24fcf1f119e36088d6ed0901319e418fd56bbc`
and `86d3974a712dcd891776b57ff5a99dacc778b9d87b51a43c3f6b44fa5038c4e8`.
Only the two paired-green rows are rerun and promoted. Their shared case-ID
SHA-256 is
`b3f6e6dae964d66c5ffdf12b1513e6d37c6973c69bdbbaee57fa1e4f4803f6e1`;
candidate/oracle report SHA-256 values are
`9638463eed022460b7286e2c41bbe4ac94c0f2790de81f453332e39c5be03b24`
and `b47b3294f44af7824015e9880d4a4d5d1be65eeab9f0ae2c2f7a3f999bc7c0aa`.
All four sidecars verify. Exactly two rows advance the Python ledger to 1,192
passed / eight skipped / 5,326 not run, SHA-256
`79d512f3af52c78e8e8e0cf79feb41c77a1ca04cb2791538f0b453503132069e`.
The two oracle-red rows remain unpromoted diagnostics.

NODE-408 closes the first browser-independent Core launcher-metadata
discrepancy. The initial candidate passed the three `errors` instances and
failed all three `devices` instances while the Playwright 1.58.0 oracle passed
6/6. The Node package now ships the complete 143-entry upstream device
descriptor snapshot, SHA-256
`ceb47c6d0366b273151121ade0217f1e96a4043594c8188fcc12c0c9b30f9353`,
with its Apache-2.0 license and pinned provenance. CJS, ESM and TypeScript
export the same static registry; importing it does not load a native artifact.
The repaired candidate and oracle both pass 6/6, with verified report SHA-256
values `2a4ca9220591a7166d0a9dd74d0581631eb0400c40fda59299eb86f92af3ed07`
and `e5f6ceb209f5b00ad63ca513584861487e1755b62cf33ea4e4eae407b1650a75`.
Exactly six Core rows move from `not-run` to `expected`, producing 1,352
expected / one skipped / two unexpected / 47,122 not run with ledger SHA-256
`f382b081ad8346ace986db9cf8254370b34a67b8c514bca9379f573d3f40f906`.
The Node suite passes 312/312 plus strict TypeScript. No Rust ABI, native
feature, browser ownership or resource boundary changes.

NODE-407 changes no code, ABI, feature or artifact. The unchanged NODE-403
wheel and Playwright 1.58.0 oracle each pass the two remaining sync Chromium
service-worker policy rows 2/2 through the runner's trusted Tailscale HTTPS
fixture. Their shared case-ID SHA-256 is
`438c118112415b7d7c6ef2b5304035493eb61e0d6bbc16a06ba88b814f047fc0`;
candidate/oracle report SHA-256 values are
`9a9e21e364401bf7ceed7a9ced73ad87c8a43618496e8ca9a63ee317c1ddac18`
and `6c9afb0ba0eae0c23a3838920129dbfbe6436e78bd3ea02be9a364cb63cd5dfe`.
Both provenance sidecars verify the secure-fixture flag. Exactly two
`not-run` rows become `passed`, producing 1,190 passed / eight skipped / 5,328
not run with ledger SHA-256
`94f83882339cd9af0b64200beea2098b8b7ba3ee551c2228b52bf19e1ae61013`.
The async and sync Chromium policy files are now both complete at 2/2. Older
MagicDNS HTTP timeouts remain environment diagnostics and are not evidence.

NODE-406 changes no code, ABI, feature or artifact. The existing NODE-403 wheel
and Playwright 1.58.0 oracle each pass the remaining environment-independent
Chromium `Keyboard.press("+")` row 1/1. Their shared case-ID SHA-256 is
`e01f427d8ca9a3bc3e7415eac485c0f27582d5f6641d927962e2054f38d365cd`;
candidate/oracle report SHA-256 values are
`0c982237fedbda5cf2383b0f29e91e9a5efe70501365c6b769aadc5ce76510fa`
and `9d696c762ba7726965289b186a4bdc24fdab15eb53bca0c4aee2c7ddd4490e07`.
Both provenance sidecars verify. Exactly one `not-run` row becomes `passed`,
producing 1,188 passed / eight skipped / 5,330 not run with ledger SHA-256
`672a6829fc84a4d55ec587ad23880de96a9cef3dd318719a00f47caae3fe7c5a`.

Two intervening paired diagnostics remain unpromoted. Candidate and oracle
both expose a Page for the Chrome 154 `javascript:` popup prompt where the old
test expects `None`; candidate/oracle report SHA-256 values are
`d639905f5a620814aa7522c6bd37e1a28a7c2b3147cbd4b0c76b39d1473ba610`
and `92659ade74c40820917c7f01105a1fb4e25d68ce70bd1740d162d38cc763ea92`.
They also both return the mandated Tailscale fixture hostname for a fulfilled
response cookie where the old test hard-codes `localhost`; report SHA-256
values are
`8ec825676b02a603eee9eabbdfd6067b19d21f67d968c086e713ce3a9ba1a9ad`
and `0c7fb9b5e3aa52043b2582db604be3bce2afbe34cfba08cb699d2b443a3ff418`.
All four sidecars verify. Neither shared environment red is treated as a
Pwright discrepancy or outcome evidence.

NODE-403 keeps ABI 104, the 90-feature native selection and every Rust resource
boundary unchanged. The sync bridge now prepends the calling Python thread's
stack to an existing Playwright error stack only while propagating an error;
the native/async stack is retained, and successful calls pay no stack-capture
cost. One AAA unit covers this boundary. The repaired portable wheel passes
250/250 installed units.

The clean wheel-backed candidate and independent Playwright 1.58.0 oracle each
pass the same isolated seven Chromium rows from `tests/sync/test_page.py`.
Their shared case-ID SHA-256 is
`483df180239c145e831a2145491cdc52f606484fd910efd21b4c2bfda2a69698`;
candidate/oracle report SHA-256 values are
`3a22d05c8bdb2ec79e19211bd6fb577574fe7893c6d16f6b3d54efa7e7a6bf35`
and `b640c1136d468c6edd847723adf8f1c58ddb8b175ced6dc61f9707e59c66190b`.
Both provenance sidecars verify. A strict diff promotes exactly seven prior
`not-run` rows, producing 1,187 passed / eight skipped / 5,331 not run with
ledger SHA-256
`036aed9d37c7e88dfbce0ffe4210e7a16e11b2f06b860a8200015a2e99257eda`.

The preserved nine-row candidate diagnostic passes seven and rejects the two
`Page.pause` rows because the upstream fixture reports `headless=True` while
the required dedicated Chrome is visibly headed. Those two environment-
incompatible rows remain `not-run`; Pwright does not pretend that headed
Inspector support is installed. Primary/rebuild wheel SHA-256 values are
`ce59434415f403caa193ac686c5468271d902380df938587f7cffab57b270f5c`
and `e9745ab64a6d67ed2cabe1b522a6cd1f26644756ab76737bd4067daad898d2e7`;
their normalized payloads match. The installed native artifact remains
`6e3aef33a88b1e056931d4bbde336c57403d95cb88dc1fabaed5469d19a85d1b`.
The 108-package dependency proof, release check, formatting, Ruff, strict
Pyright and 22-file Python 3.9 runtime audit pass. Full Python and Core JS/TS
conformance remain incomplete.

NODE-402 changes no code, ABI, feature or resource boundary. The two previously
unrun Chromium rows in the unchanged sync `test_request_fulfill.py` file now
pass 2/2 from the NODE-401 portable wheel and 2/2 in the independent
Playwright 1.58.0 oracle. Their shared case-ID SHA-256 is
`ba72fca2dff7a5abd0c999df4bc5198fb1a5c059341346a5bd466788342f0280`;
candidate/oracle report SHA-256 values are
`b95188b98d62986ba046d643cf76d4429566b2627aac68b280c596dd89d3aa78`
and `637626c1e57deda856ccab3b3c000139934fba966c38c228e43f4e46949342dd`.
Both provenance sidecars verify. Together with the earlier override row, the
sync file's Chromium surface is now 3/3, matching the already complete async
file. A strict diff promotes exactly two `not-run` rows and nothing else,
producing 1,180 passed / eight skipped / 5,338 not run with ledger SHA-256
`1846062292632f04561166ad19d4e8265a624f3eb837aa969ec14ebf6db55d78`.

NODE-401 executes and promotes the complete official 25-row TodoMVC Chromium
profile without changing ABI 104 or the 90-feature native selection. Rust now
distinguishes a top-level JavaScript function from an expression containing a
nested arrow, so `JSON.parse(...).map(i => i.title)` remains an expression.
Python exposes the stable `BrowserContext.tracing` shape required by
`pytest-playwright`: stopping inactive tracing is a no-op, while trace output
and recording remain explicitly unsupported rather than fabricating an
artifact. The first wheel-backed diagnostic exposed both gaps and is not
outcome evidence.

The repaired wheel-backed candidate and independent Playwright 1.58.0 oracle
each pass the same isolated 25/25 cases; their shared case-ID digest is
`455cee73934771c213f5f55da7a4aa56e6cb802f5a5f40a84a7a47a4744bc8a8`.
Report SHA-256 values are
`19e27f3c63ecf13804581d9c079afc5393ca005560d7770d34db07be82b82967`
and `a8792fe4c0a1a19db6dcf3aaaf73e7cedb96c78edd70d14b9b0c2a4f59db8fa6`;
both provenance sidecars verify. Chrome 154 requires a task-owned
`about:blank` Page before the old oracle attaches; the preserved no-Page
diagnostic fails during `Browser.setDownloadBehavior` before a test body and
is not promoted. Exactly 25 prior `not-run` rows become `passed`, producing
1,178 passed / eight skipped / 5,340 not run with ledger SHA-256
`bfb8e95031d10240bc110b16ba18a800711f0e5e4cc513cef4e8cfd0f14198bd`.

The primary and independent portable wheels hash to
`5d0f288665f5709342f6abc4040e4c3521d9ceac3ea4abd4d0203fbc7471820d`
and `fa15e010538e71550d460ac282630aa11c3232c35aeb2b6df05a23b814eb1df9`;
their normalized payloads match. The fresh installed wheel reports ABI104/90,
uses bundled OpenSSL dylibs and passes 249/249 units. The Rust source-classifier
tests pass 3/3, and the 108-package dependency proof, release check, formatting,
Ruff, strict Pyright and Python 3.9 runtime audit pass. Full Python and Core
JS/TS conformance remain incomplete.

NODE-400 advances the Python native contract to ABI 104 while retaining the
same 90 selected capabilities. `Locator.to_have_value()` now accepts exact or
regular-expression matchers, and `Locator.to_have_class()` accepts exact or
regular-expression matchers both singly and in ordered mixed arrays. The
browser programs remain static; Rust validates regex flags, follows HTML label
controls for value reads, uses the SVG-safe ordered `classList.toString()`
representation and owns retry/deadline behavior. Node `_expect` delegates the
same `to.have.value`, `to.have.class` and `to.have.class.array` terms to Rust.

The four unchanged async/sync Chromium class/value assertion invocations pass
4/4 in both isolated candidate and Playwright 1.58.0 oracle lanes. Their report
SHA-256 values are
`ab19a7424ab038b1a1757966419b14aca7b92f449eb3e51843d860e9a201b826`
and `5fb83a36c9b1fb5a6e67d8cb2669a9a021330a2b20def1044e2237c49baa7458`.
Only those green candidate rows enter the cumulative ledger, now 1,153 passed /
eight skipped / 5,365 not run with SHA-256
`ab288ffacb37a4632adcf1be1d70b6beb4a7dee82f26f8525a43a0f3d922e020`.
The first candidate attempt is retained only as a setup diagnostic: it exposed
that history navigation was registered but omitted from the production
capability-selection count, so no test body ran. The corrected selection and
all-capability regression prevent that manifest undercount.

The portable ABI3 wheel and independent rebuild have SHA-256 values
`63d7c6387ca5c16b48e9df314ad9c3af66d6d46dd79a73322148bf61c7155c4d`
and `1028d54546a867cb29c4be24b7106adf96329eec337909612447cabfa13ed796`;
their payloads match after normalizing only CycloneDX serial/timestamp fields.
The installed primary wheel loads from `site-packages`, reports ABI 104 and 90
features, passes the same four official candidate cases and passes 246/246
units. Its installed native artifact SHA-256 is
`d753a8d77827d72bcb8a1cc8fb7c804657088449af628fe2ac351e7dbb19add2`.
Node passes 309/309, and the Python
dependency proof, Node boundary, strict typing, Python 3.9 runtime audit,
release checks and formatting gates pass. Full Python and Core conformance
remain incomplete.

NODE-399 adds the independently selectable `pwright.history_navigation` 1.0
capability and the `history-navigation` feature through the Chromium navigation
backend, aggregate backend, composition, Rust host, Node addon and Python addon.
Rust uses native `Page.getNavigationHistory` / `Page.navigateToHistoryEntry`,
selects the previous entry by bounded native history ID and distinguishes
same-document hash traversal from replacement-document lifecycle waiting.
Python `Page.go_back()` and Node `Page.goBack()` are thin option/result adapters;
the official native profiles opt in explicitly and default builds do not gain
the capability crate.

The same checkpoint completes the TodoMVC assertion prerequisites for exact
class lists, substring and regex text, checked, empty and exact value states.
Browser-side programs remain static and Rust owns deadlines, retries, result
bounds and collection ordering. The isolated official runner now treats a
profile directory as a path-segment-bounded selection instead of requiring a
single file. The 90-feature ABI remains 103. Current non-browser gates pass:
246 Python units, 40 Node profile contracts, strict TypeScript/Pyright, Python
3.9 cast audit, generated-schema regeneration, five history backend tests,
single-feature Node/Python addon clippy and the optional-dependency boundary.
The installed test artifact is now an auditwheel-repaired ABI3 wheel with
SHA-256 `5dd5bb093a84e6a71f9c6d90a5610a41f10abecf79af503a928bb124521d8979`;
its native module resolves bundled OpenSSL libraries through `@loader_path`.
An independent repaired build has identical payloads after normalizing only
the CycloneDX serial number and timestamp, and the repaired install passes all
246 Python units. Candidate and oracle collection runs each report all 25
TodoMVC Chromium rows and both provenance sidecars verify; collection executes
no test body and is not outcome evidence. The standalone and 25-feature Node
artifacts load with `pageGoBack`; their SHA-256 values are
`a3b6c3f189097cb4c1cb9da67f8cac8fc06c19fb0dd7005472ee2a81ea656774`
and `44a37f7126ed9ec81980eb815a752d64acfbc64c9c3bfe8a05b5f3f4773a8507`.
Live TodoMVC candidate/oracle evidence is still pending an unoccupied dedicated
CDP browser and must not be inferred from these implementation gates.

NODE-398 updates
`scripts/inventory_playwright_python_official_fixture_definitions.py` and its
15-test AAA suite for multi-profile collection. The current ledgers contain
6,526 bindings and 95 definitions with zero unresolved rows; all 25 TodoMVC
rows resolve the plugin Page provider. Binding, definition and summary SHA-256
values are
`f34cc45de0bbb8452c7b7917b403d4c7e21c0604066cca4b6b744d9045ef0d24`,
`26dd34a136b9f76778317cc89784b83342e6a79a57a84e543cce58a3d7da4309`
and `3a7daa387def3ac08613c4bd7a32ce48841bac703909034630ea63acddf8ce1a`.
An independent run reproduces both ledgers byte for byte.

NODE-397 updates `scripts/inventory_playwright_python_official_fixtures.py`
and its seven-test AAA suite to collect core and examples in their distinct
frozen environments. The complete closure ledger and summary SHA-256 values
are `3f317451434319c80dc643efc8f4be24ea097cdef2a0e0948eaa3a5c45e28903`
and `276d7c07b98e16215b1edd1199771d6a49985cb3b806cf89eb4f13c2539aecfb`.
Collection executes no fixture or test body.

NODE-396 adds the manifest collection profiles, the hash-locked
`examples-oracle-requirements.in` / `.txt`, profile-aware inventory and strict
outcome-rebase tooling plus nine AAA tests. The canonical inventory, outcome
and applicability ledgers have SHA-256 values
`3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41`,
`456fcb0f097261f71a24e4500308740684250a3c393cda5d3161ed52918c574b`
and `31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
The profiled inventory is now the default across collection, audit, runner,
fixture and result-summary scripts.

The following NODE-394/395 entries describe the preserved root-only diagnostic
artifacts. NODE-396 through NODE-398 supersede their example-fixture topology.

NODE-395 adds
`scripts/inventory_playwright_python_official_fixture_definitions.py` and its
14-test AAA suite. It resolves all Python closure names to 77 definitions,
retains override chains and semantic roles, joins applicability/outcomes into
fixture-induced execution bands, and isolates the absent TodoMVC `page`
provider. The binding, definition and summary SHA-256 values are
`36f993858ee5a01c0a11f002f6183bf8e521419fc1f857d7cc95e4e33cfa5ae4`,
`28340e32bf3c1ccdb6bdf72a3c9b03783ebe218512b0bcf4ceb14c89c99f80ad`
and `ab313b9a6d8bd13ffd67c4155ab6c2e222b95ec45a141dbee16b1ea1aeae180d`.
A second collection is byte-identical; no fixture or test body executes.

NODE-394 adds `scripts/inventory_playwright_python_official_fixtures.py` and
its six-test AAA suite. The collect-only plugin records direct, initial and
complete fixture names for all 6,526 canonical Python invocations, checks the
clean pin and inventory digest, and requires exact ordered agreement with the
NODE-393 source parameters. The fixture ledger and summary SHA-256 values are
`fde11844e88c4cadadbac096c0490ebb682eea9e97855806db13fb6925f3f988`
and `42c34b9584855113d75bfe7ad373c059081f2f10b310614019d741774c1d5260`.
It executes no fixture or test body and changes no runtime or outcome.

NODE-393 changes the Python expanded-invocation audit and its tests. Every row
now records source-declared pytest parameters; summary tables cross-tab all 36
direct inputs and the 12 parameterless rows against applicability and outcomes.
This remains distinct from the transitive/autouse closure now captured by
NODE-394.
The regenerated audit ledger and summary SHA-256 values are
`3c58942c03e7a1aefe41d472258ea0ed07a3388626e645587bce82f9cb93c2e3`
and `6c0eb8a7e7de67eb4d80b387808b1b83273630370fd4c6d05efb5ab5e804508a`.
Twelve AAA tests plus strict Python gates pass; outcomes are unchanged.

NODE-392 changes Python official-runner, source-verifier and provenance-
verifier tooling plus one fixed browserless manifest. A non-default execution
policy now requires exact manifest selection and explicit browserless
capabilities, while upstream's autouse `start_server` makes Tailscale fixture
servers mandatory. Nineteen focused AAA tests, Ruff, Python 3.9 runtime-type checks
and Pyright pass.

Candidate and oracle each pass three stop-lifecycle rows and share three
asyncio-topology failures. Only the green rows are selected, adding two new
outcomes to the cumulative Python ledger: 1,149 passed / eight skipped / 5,369
not run, SHA-256
`b7c38ba4cdb1cacb9b630e07e8f18b0a4f72f4fd8fc5cc92231a7ed613a35bf8`.
The complete audit remains 6,526 rows and 6,514 applicable rows; its ledger and
summary SHA-256 values are
`3c58942c03e7a1aefe41d472258ea0ed07a3388626e645587bce82f9cb93c2e3`
and `6c0eb8a7e7de67eb4d80b387808b1b83273630370fd4c6d05efb5ab5e804508a`.
ABI 103 and all runtime/package capabilities are unchanged.

NODE-391 changes only Core applicability audit tooling and documentation. It
adds the applicable `public-api-with-private-test-helper` class for fixtures,
platform oracles and result readers imported from upstream internals. Exact
EventEmitter, image-tool, parser, generator, debug-controller, unit-algorithm
and Page-leak implementation tests remain internal; internal-importing
non-public source trees become product infrastructure. No outcome byte changes.

The 13 AAA tests, Ruff, Python 3.9 runtime-type audit, Pyright and patch checks
pass. Complete counts are 32,148 public candidates, 444
public-with-private-hook, 6,279 public-with-private-helper, 3,458 internal and
6,148 product invocations. The helper queue contains 684 expected and 5,595
not-run rows. The audit ledger and summary SHA-256 values are
`32520af75e4ac14352a9060e6f7156900bb52c4a3d99fe927311ba7490328af0` and
`d2f43298716b10b67f9a15bd262c926a2f19796810c94da95121117a56453094`.
The full outcome ledger remains 1,346 expected / one skipped / two unexpected /
47,128 not run.

NODE-390 changes only Core applicability audit tooling and documentation.
Thirteen sources had 1,951 invocations classified internal solely because the
source contained private client access. The invocation audit now recomputes
direct private access per bounded segment and explicitly retains the shared
channel-scope and crash fixtures as internal. It restores 1,334 rows to
ordinary public candidates, 27 to public-with-private-hook and six to product
infrastructure; no outcome ledger byte changes.

The 11 AAA tests, Ruff, Python 3.9 runtime-type audit, Pyright and patch checks
pass. The complete 48,477-row audit counts 32,148 public candidates, 10,305
implementation-internal, 5,580 product-infrastructure and 444
public-with-private-hook invocations. Its ledger and summary SHA-256 values are
`f8aff8d62e4ff1a32862eaa5b51967d1312b1c6636fe1baab021c53d48a54971` and
`4223ac1278b23bd6344c524932ba0a202c76456f54d73a55f7dcf8a21274fc5d`.
The outcome ledger remains 1,346 expected / one skipped / two unexpected /
47,128 not run.

NODE-389 changes only Core applicability audit tooling and documentation. All
67 invocations from 48 `examples/**` files carry the pinned inventory's
`official-example` role and validate five example applications. They remain
in the ledger and remain `not-run`, but are now product infrastructure with
the exact reason
`product-infrastructure-role:example-application-validation`. Generated audit
rows now retain `role`; no outcome ledger byte changes.

The seven AAA classifier tests and strict Python checks pass. The complete
48,477-row audit counts 30,814 public candidates, 11,672
implementation-internal, 5,574 product-infrastructure and 417
public-with-private-hook invocations. Its ledger and summary SHA-256 values are
`fa4d18cf1a69e33a3b39b60fa03b6bfb5f920deec18a74f5c1a5f06b3a01b1c4` and
`53c1e34c4c8c6b543c602ef73d472e8864e1cc4f138fb196aa88e763cada8ea8`.
The unchanged outcome ledger remains 1,346 expected / one skipped / two
unexpected / 47,128 not run.

NODE-388 changes only Core applicability audit tooling and documentation. The
19 pinned `tests/library/events/*.spec.ts` files import the repository-local
`./utils` EventEmitter, so all 477 expanded invocations now record
`local-implementation-under-test:tests/library/events/utils`. Of those, 36
were already internal and 441 move from public candidate to
implementation-internal; no outcome ledger byte changes.

The six AAA classifier tests pass. The regenerated 48,477-row audit counts
30,881 public candidates, 11,672 implementation-internal, 5,507 product
infrastructure and 417 public-with-private-hook invocations. Its ledger and
summary SHA-256 values are
`74baa2e47b58e90a9e0eac1021bf47ea7f2b01f7aa38af5730ca1bbf9d3b5674` and
`72d5ba6f2397718c095ffc3d4f6fef5d68d38ca1623b5d76452d740df3c35843`.
The unchanged NODE-372 outcome ledger remains 1,346 expected / one skipped /
two unexpected / 47,128 not run, SHA-256
`a41de99dbaa2e2d0cd0a9b4f9ba6e4490f4a6affb818885e6ed447d6b797907c`.

NODE-387 advances Python to ABI 103 without adding a capability feature.
`element-handle-dom` plus `keyboard` now expose one exact-focus/input command;
Page/Frame typing, query and selector-evaluation facades reuse existing Rust
owners. The fixed 29-case candidate improves from 16 passed / 12 failed / one
skipped to 26 passed / two failed / one skipped, versus oracle 27 / one / one.
Both remaining failures are shared-environment evidence and remain unmerged.

The candidate ledger advances only 26 passes plus the upstream skip to 1,147
passed / eight skipped / 5,371 not run, SHA-256
`607b552d6435948ed6e66fb8c9d0d75c92e78de51de007c20fb35f4f5a03ab8c`.
The ABI 103 / 89-feature wheel passes 239/239 installed units, release Rust
tests, strict Rust/Python gates, the 108-package proof, normalized wheel
reproducibility and a verified 6,526/6,526 collection. Full Python and Core
JS/TS execution remains unfinished.

NODE-386 advances Python to ABI 102 with the independently selectable
`keyboard` feature. One PyO3 operation boundary exposes the existing bounded
Rust Page Keyboard to thin async/sync facades; Python owns no input protocol,
Page binding, queue or cancellation state.

The fixed five-case focus manifest SHA-256 is
`a8356082dfa848db418c89fa67739b6a3a4573bc5db5890e13ba9e77dd0d4e3f`.
The preserved installed candidate baseline is 3/5 and the independent oracle
is 5/5; the fresh installed candidate is 5/5. Final candidate JUnit SHA-256 is
`13adb094a397aae69bf2fad378b1b23b7a090034af44f9af90aea68085d86c3e`,
and both candidate and oracle provenance sidecars verify.

Candidate is now 1,121 passed / seven skipped / 5,398 not run with no recorded
failure, ledger SHA-256
`3cec1a0bd837082094831e4cf5b11e20625e5fb095c1d06ef6b5f643027f0fe0`.
The ABI 102 / 89-feature wheel passes 232/232 units and the 108-package proof.
Final-source wheel payloads reproduce after CycloneDX normalization; archive
SHA-256 values are
`13443f5f68f6da9e3fd3b2d60ac9e50dddcd61014248f7352cb3ac5fe84caccd`
and `4a999b838864f72df22a832bb9a2572c181d72b27fcda8daddccdff833b66c7d`.
Full Python and Core JS/TS execution remains unfinished.

NODE-385 advances Python to ABI 101 with the independently selectable
`request-timing` feature. One stable optional timing sidecar flows through the
bounded network model, codec, Chromium adapter, composition, host and PyO3
projection. Checked request/response notice sequences provide an exact
publication fence for navigation-like Python calls. Cached responses,
redirect terminal timing, stale snapshot merges, positive submicrosecond
values and exact-zero unavailable connection phases remain owned at their
native boundaries.

The fixed four-case manifest SHA-256 is
`46a5e4b59d760daaf5dd63f7cd31fd03207ed20bec8349541e808d76ca1418fb`.
The preserved installed baseline passes 2/4; the final fresh installed wheel
and official Playwright 1.58.0 oracle pass 4/4. Final JUnit SHA-256 values are
`cc8ec9dc6b49f40976d7f6672aea779f5ef70421549626117a09acd5b442a56f`
and `46b177107aca6c246073563ce466c28ab86bbb6223cc01ec96258ea8035bc04c`;
both provenance sidecars verify.

Candidate is 1,119 passed / two failed / seven skipped / 5,398 not run,
SHA-256
`d5d26ca3c234e119808548b48cb6ca96e5802718f48adb619cac73006e2cae95`;
oracle is 738 passed / seven skipped / 5,781 not run, SHA-256
`1ff080b72543b70bd04a7c327e8dcfcc2260aa844953ce9de2ebb4dba8b60ba3`.
The two failures remain the explicit missing-`Page.keyboard` focus rows. The
ABI 101 / 88-feature wheel passes 226/226 units and the 108-package proof.
Final-source wheel payloads reproduce after CycloneDX normalization; archive
SHA-256 values are
`4b5eae899bedc877edea4baf43bbb711c64938f646c38a1f3d19852667e71ec8`
and `ae9ef143fef21e943c1980eb147d2113f7c4b9a2fe86968f10e0a1037b7f9154`.
Full Python and Core JS/TS execution remains unfinished.

NODE-384 adds a fixed two-case manifest for the remaining Chromium rows in
upstream `test_request_fulfill.py`, SHA-256
`a8e7f4004b567fd6d12c9e4f0f9791405a8a35f548488f88414e46e98b4c52f0`.
The unchanged ABI 100 installed candidate and official oracle pass 2/2; JUnit
SHA-256 values are
`3fa160ad3813072712aa645cadb852962569e3e4f305103cad61131e690f2f98`
and `afc6b25477b04ed913b218fc99d1376cc22142539455eb3eae6233d2e9bfa85b`.
Both sidecars verify. Candidate is 1,110 passed / seven skipped / 5,409 not
run, SHA-256
`324dc918352b2afa46745ab26825f0b7d19db9f657b237b769c68ad2754585c1`;
oracle is 727 / seven / 5,792, SHA-256
`393bf16e0e001f6a7f264d1f852fdebd0abaa8fef4d6c6b47c464afc4f6a761a`.
No runtime, feature, ABI or resource boundary changes.

NODE-383 advances Python to ABI 100 and closes the fixed 15-case remaining
async Chromium JSHandle tranche. The thin sync/async facades only unwrap exact
public Handle wrappers while retaining bounded container identity. PyO3 and
the Rust host bind ordered native-owner slots into the bounded evaluation tree,
preserve the plain-value route, publish `get_properties()` as an atomic Handle
batch, decode Date values for Python 3.9 and preserve official native previews.

The fixed manifest SHA-256 is
`6e312d954906f7368d65ee2efe7edd8ab34ff4477586c0777953799407532718`.
Baseline candidate, oracle and final installed candidate pass 6/15, 15/15 and
15/15 respectively; their report SHA-256 values are
`888671c948ee114e471a948db16c548c3545d2796fc91abb99ef8f6660b58bb7`,
`e24c9e9c41cb8a7e41ed2543a720084fbdd55dcaa6a2c5e853e8a64853af265b`
and `f1c296d03e29f83b00f668c671bd3416329a3bd458f21d85cfb762df9d548b23`.
Authentic candidate/oracle provenance verifies.

Candidate is 1,108 passed / seven skipped / 5,411 not run, SHA-256
`3708cc9f5c27b9e1a4a40e37c1e12bf8fca016e55a7dfb0ad55462ca8be2611a`;
oracle is 725 / seven / 5,794, SHA-256
`01028d3a743ce7c31a6a8c517be3b6f8bf1c702a3c5209d86fb9e1b66a12f0f5`.
The ABI 100 / 87-feature wheel passes 221/221 units; two builds reproduce after
CycloneDX normalization. Full Python and Core execution remains unfinished.

NODE-382 advances Python to ABI 99 with the opt-in
`file-input-native-paths` leaf. It adds no package beyond `file-input` and
keeps native-directory classification, exact target preflight,
`DOM.setFileInputFiles`, source retention and cancellation cleanup in bounded
Rust owners. Ordinary paths remain payload transfers with the existing 50 MiB
remote-CDP guard.

The fixed 18-case manifest SHA-256 is
`fadafa8037196c9829461f666df58c0e1fbeff4713ebb0ffaba2da3f78cb5f53`.
The installed candidate passes 17/18; official Playwright passes 16/18. Both
reject the 200 MiB single file at the same official remote guard, while the
oracle additionally rejects ten 49 MiB chooser files that the native candidate
passes. All 16 oracle-executable rows agree. Candidate/oracle report SHA-256
values are
`53a396e74a23a615f99e931474a06b3d9b102a346e1860c2fdc7cedc346a823f`
and `9bcee12e021280b75bde6da43d180c421d0835c5ec2471a0dc2702cbf0a5ad0b`;
their sidecars verify.

Only green result subsets are merged. Candidate is 1,093 passed / seven
skipped / 5,426 not run, SHA-256
`2e29c50908b763d1cf39defafc540557aa16fd7e1091d0c1ea6f28270ac544f7`;
oracle is 710 / seven / 5,809, SHA-256
`5dd92c0935ec10748d6990aa04f4866e63e47520fe718870394a59461a2d8ee3`.
The ABI 99 / 87-feature wheel passes 217/217 units; two builds reproduce after
CycloneDX normalization. Full Python and Core execution remains unfinished.

NODE-378/379 advances Python to ABI 97 with the opt-in
`file-chooser-native-paths` leaf. It composes over `file-chooser-events`
without adding a package, resolves the exact chooser node in the utility
world, dispatches `DOM.setFileInputFiles` by `objectId`, and owns the bounded
object group through cancellation-safe cleanup. The accompanying locator
user-gesture, Page-binding publication fence and Python chooser-lease fixes
remain in their existing owners rather than moving browser behavior into the
facade.

The four unchanged official async input rows pass 4/4 in the source candidate,
official Playwright 1.58.0 oracle and fresh installed-wheel candidate. Their
source/final/oracle JUnit SHA-256 values are
`ce1724d501a14438fa7b32bca9571b0c4499a5f869c8ca93dd2a77b6c1b3fb90`,
`e99fe0edb6c9b50e5fed8ff4571bcee4d9fe27bf30e3c6b2a2c6f9d012ffcff8`
and `e2b8145706ad78aade6dd83abf851b017d01cb82a14bb3ef2d36d9335032e231`.
The final candidate sidecar SHA-256 is
`9d2c75ba2679bf8122647dd0fc0d4d3d23bed7a001b00e23b8d47474d580b3d4`;
all three authentic sidecars verify. Only the installed-wheel selection is
merged. Candidate is 1,076 passed / seven skipped / 5,443 not run, SHA-256
`0284d9c1750530e607c3b07daff759278ffae92964751c644f7244d801a80cbf`;
oracle is 694 / seven / 5,825, SHA-256
`d5fc04bc059fdab72dd20e88ea8157655aa1bf6e62ab4191c7152ec3fe635d69`.
Applicable outcomes are 1,073 passed / seven skipped / 5,434 not run; the 12
reviewed implementation-internal rows remain separately visible.

The ABI 97 wheels hash to
`4cb27befae61e6c3f99343d38e0f68519b361a2d70bd2e99de2fb7bbae10266a`
and `81b3fdff4a27c9ab3e7e8afb9474c53be0c6145ce8b7132164b2a6d44161c1c8`;
their payloads reproduce after CycloneDX identity/time normalization. A fresh
wheel reports 85 features, passes 213/213 units and produces a verified full
collection sidecar for all 6,526 expanded IDs (JUnit SHA-256
`e2f1426a46a3ab46ac47d69e55e12cefde1e27134263f6a6b87851e06f976af6`).
The earlier collection setup report missing `requests`/`objgraph` remains
setup-only and is not merged. Formatting, strict Rust/Python lint and types,
30/30 harness tests, Python 3.9 audit and the 108-package proof pass.

NODE-377 adds the full Python expanded-invocation applicability audit. The
new `scripts/audit_playwright_python_official_invocations.py` verifies the
clean pinned checkout plus inventory/outcome hashes, resolves all 6,526 pytest
IDs to source AST nodes and follows private annotations, helpers, imports,
attributes and selector syntax without modifying outcomes. Its syntax pass
finds 24 direct-private rows, and the fail-closed eight-base-test policy
reviews all of them: 12 are applicable public behavior with private assertion
or fixture machinery and 12 are implementation-internal. In total 6,514 rows
are applicable and 12 remain explicitly non-applicable. The policy SHA-256 is
`f8e33a8161af2eedd3274f4a17e37f15ba20af14ff37e229ac5f923138ff3231`;
the generated ledger SHA-256 is
`748a48d459053b03db92de1d048abfc16649bb5a4fc0841340056db834bd8ace`.

The first applicable direct-private checkpoint preserves the unchanged red
candidate, green official oracle, green source repair and green installed
release wheel for the async JSHandle evaluate row. Their JUnit SHA-256 values
are `4524ad75633b45728d7464f4e472c51e22051aca54fd018b575076eb7dbf4304`,
`b74e92d666619e9e5bb586bcbaa677f26acc46a25de03802aa290ae47e689a98`,
`1207db183abbe5eacf17b7be4069fb550b480aa5a297d7f5b84c210456f03dfd`
and `7746d54754e3372a0779871d0d5cfed546724769c959c28edd312b9254695f5b`;
all sidecars verify independently. The facade adds only a read-only
`_impl_obj` compatibility adapter and official JSHandle representation. The
candidate ledger becomes 1,071 passed / seven skipped / 5,448 not run,
SHA-256
`c408e3fcd2abc439830a570302227ea08b4b9729a876b24416928174cb6807e4`,
and the oracle becomes 689 / seven / 5,830, SHA-256
`9f8b5dce21bf2b02ccff5fd4adecae2b7a938828e7b4a6b815d0c6ac407b8d88`.
Installed-wheel units pass 208/208; two release wheels reproduce at the
payload level after CycloneDX normalization. Ruff, strict Pyright, the 11-file
Python 3.9 audit and all 30 tooling AAA tests pass.

NODE-376 adds deterministic sidecars and an independent verifier for future
official Python JUnit reports. Evidence now binds the exact interpreter,
pinned manifest/checkout, loaded official and candidate package trees, ABI3
artifact, harness inputs, fixture/browser selection, command and aggregate
result. Isolated lanes also bind inventory-derived expanded IDs and every
per-case report. Builder, verifier and runner pass 17 AAA tests; no historical
report is backfilled. Fresh candidate and oracle full collections now each
verify all 6,526 IDs. Their JUnit SHA-256 values are
`69df905bd92005f89fa2e7a59f01bb2b7095816f59791fb1aa57f63b04d0e535`
and `1dbc26cda8dc7b7a1740bd626b94bd1839ec6f20f70dda905a0aa6b3148fb24e`.
Collection remains identity evidence, not proof that the rows executed.

NODE-375 keeps Python ABI 96 and removes 22 runtime PEP 604 expressions from
`typing.cast()` calls so the declared Python 3.9 floor is real. The package
source audit reports zero remaining runtime union casts and CPython 3.9.6
imports both facades against the ABI3 extension. The 11-file audit also covers
the runner, provenance and invocation-audit tools. The official Python runner
now fails closed on remote CDP endpoints, invalid Tailscale fixture addresses
and non-MagicDNS server prefixes, and rejects existing nonblank dedicated-Chrome
Pages before every non-collection run. Twelve focused tests pass;
memory/browser-heavy gates remain pending.

NODE-374 adds strict offline verification for every future Core provenance
sidecar. The sidecar now hashes the pinned manifest, selected harness inputs,
candidate JS source tree and selected native artifact in addition to the JSON
report. The verifier reconstructs the command from the pinned suite and fails
closed on schema, endpoint, MagicDNS, adapter, source or artifact drift.
Candidate profiles reject path syntax and workers must be positive before any
run. Runner plus verifier pass 11 AAA tests. No historical report is backfilled
and the first runtime-produced verified sidecar remains pending a memory-safe
lane.

NODE-373 makes every future guarded Core execution write a sibling provenance
sidecar after its JSON result exists. The record pins upstream identity,
suite/adapter/project/filter selection, candidate path/profile, browser and
fixture endpoints, exact command, exit code and result SHA-256. Red reports are
recorded too. The runner unit suite is now five AAA cases. Existing result
files are deliberately not backfilled; runtime production remains a pending
gate while system memory pressure prevents another safe lane.

NODE-372 revalidates the complete unchanged Page init-script file 9/9 with an
explicit current candidate profile and adds a 9/9 independent oracle. The
candidate rows were already expected; exactly nine oracle rows move from
`not-run` to `expected`, producing 1,051 expected / one unexpected / 47,425
not-run, SHA-256
`5aa674c8f42d538f54b58caf0b506662f6cb3ced6b736c4052b4eaa0d010b842`.
Candidate/oracle report hashes are
`fc61da558be2f1a47f73487e220d89b53f1befd040417552ffd93a9aee46f6ab`
and `bde4ec0baa03113788407eecb02589f26d30227e140c67e3f0c4630c72d0be9e`.
The official Core runner now requires a named candidate native profile and has
four AAA selection tests; this closes the operator-error path that produced an
unmerged default-profile 1/9 diagnostic.

NODE-371 adds a browserless, outcome-preserving applicability audit for the
complete 48,477-invocation Core inventory. The new
`scripts/audit_playwright_official_invocations.py` verifies the pinned clean
source and inventory/outcome hashes, bounds each test from its collected source
line to the next collected test, records exact `__testHook...` dependencies and
cross-tabulates them against an optional complete outcome ledger. Its five AAA
tests cover the classification and source-boundary invariants. The generated
audit counts 31,322 public candidates, 11,231 implementation-internal, 5,507
product-infrastructure and 417 public-with-private-hook invocations, SHA-256
`25ec81b43f47cdcb41e8d04b04f0d199d6733ea90e714a6f732dbb893018a90a`.
Both current candidate unexpected rows are now structurally identified as
private-hook-dependent without changing their outcome or the NODE-370 ledger.

NODE-369 adds no source or dependency edge; it promotes the complete unchanged
Context extra-header file to 2/2 candidate and 2/2 oracle. NODE-370 adds the
independently selectable `pwright-node-addon/bypass-csp` feature, the thin
`packages/pwright/src/context-policies.cjs` selector and its four AAA tests.
The feature maps only to `pwright-node-host/bypass-csp`; the fixed Rust Context
registry and Chromium session initializer remain the sole policy owners.

The standalone/profile verifier requires the marker and actual native creation
method. Moving FrameLocator and console callbacks into feature-gated N-API impl
blocks restores compilation for default and unrelated standalone profiles. The
native class source audit now accounts for 14 opaque classes and includes
FrameLocator in every handles registry. Standalone and official release
artifacts build/load; Node passes 306/306 plus strict TypeScript, boundary,
formatting and whitespace checks.

The unchanged CSP candidate progresses from 0/4 to 4/4. Its baseline/final
report hashes are
`04ea4ea2f89767200229eb79e7137951b3b1a63de42c87a8ce116449fcd11450`
and `52762acf1fb0ba47924897e6827c3228df8fcb51700aba6de3d60d1a4ff89a2f`.
The latest candidate ledger is 1,346 expected / one skipped / two unexpected /
47,128 not-run, SHA-256
`67dd35b67d392963e1848192c88ae1bbd5e811dee4856244022f924958eae508`.
After the unrelated shared-Chrome renderer exited naturally, the guarded
NODE-370 oracle passed the unchanged CSP file 4/4. Its report SHA-256 is
`a60aba4f4226eb3d74eef621292a7dc059b81c8450e867c77057b2cb69dc2fbf`.
Exactly those four rows move from `not-run` to `expected`; the latest oracle is
1,042 expected / one unexpected / 47,434 not-run, SHA-256
`626e596ce9e0579fdcb8de5025060c51290cd6d78fb641286f64748d6d9b0a30`.

NODE-366/367/368 adds no Pwright package and keeps the native ABI version
unchanged. The official profile now selects two additive addon features:
`page-dialogs = page-events + pwright-node-host/page-dialogs` and
`page-routes = pwright-node-host/page-routes`. New owned-boundary sources are
`crates/pwright-node-addon/src/page_dialogs.rs`,
`crates/pwright-node-addon/src/page_routes.rs`,
`packages/pwright/src/dialog.cjs`,
`packages/pwright/src/page-routes.cjs` and their focused
`dialog-adapter.test.mjs` / `page-routes-adapter.test.mjs` contracts.
`packages/pwright/src/navigation-error.cjs` is a presentation-only adapter;
client-redirect lifecycle adoption remains in the existing Rust navigation
owner.

The unchanged official `frame-goto.spec.ts`, `page-click-react.spec.ts` and
`page-wait-for-load-state.spec.ts` selections pass candidate/oracle 4/4, 3/3
and 18/18. Canonical Core ledgers are candidate 1,340 expected / one skipped /
two unexpected / 47,134 not-run, SHA-256
`da4fe162d3dfeb7fa87017834c1d412fbbca6a84c026b5284d648156fdcca483`,
and oracle 1,036 expected / one unexpected / 47,440 not-run, SHA-256
`cd0ef619ca9c96955e5e283a5089a785e6a19214d2c846b601d28d256f50b0e0`.
The two candidate failures are unchanged implementation-private test-hook rows
and remain explicit. Node passes 301/301; the selected host passes 745 with 14
environment-gated ignores; strict TypeScript, native-load and selected
host/addon Clippy gates pass. Full Core and Python corpora remain incomplete.

NODE-365 adds no package, Cargo feature or native-profile dependency edge.
The existing `console-events` profile now exposes bounded synchronous Page and
BrowserContext subscription setters. Node remains the public `EventEmitter`
owner and mirrors only first-listener registration and last-listener release;
Rust remains the sole browser observer, subscription-capacity and publication
owner. Artifact validation requires both native setters, preventing a stale
marker-only binary from silently losing events. Owned Context Page/OOPIF
sessions also set Chromium's automation override without changing external or
default Contexts.

The unchanged official `page/page-basic.spec.ts` passes 18/18 candidate. The
external-CDP oracle passes 17/18 and explicitly records only the adapter's
`navigator.webdriver` launch mismatch. Canonical Core ledgers are candidate
1,336 expected / one skipped / six unexpected / 47,134 not-run, SHA-256
`cb22fece25ca573b34ed8a9ccf97aff18f8eb2f1966aa556b375f5b986fe085a`,
and oracle 1,011 expected / one unexpected / 47,465 not-run, SHA-256
`211d33f744085a2dd91fd13ed7ac3540b5abe619bda8f217a8e2752362ea6d92`.
Node passes 289/289, the exact selected host passes 710 with 14 explicit
environment-gated ignores, and release build/native-load, strict TypeScript,
formatting, diff and strict host/addon Clippy gates pass. Full-corpus execution
remains incomplete.

NODE-364 adds no package, Cargo feature or native-profile dependency edge. The
existing `handles` selection now publishes the already implemented
`HostFrameLocator` as a thin `NativeFrameLocator`; its derived Locator retains
the iframe owner recipe in Rust and resolves the current child Frame on every
operation. The legacy JS Handle/contentFrame resolver remains only as an
older-artifact fallback. The same checkpoint adds Playwright-compatible
bounded one-shot matcher timing, CSS closing-token diagnostics and the public
`APIResponse._fetchLog()` adapter without adding a browser, HTTP client,
selector engine or retry loop to JavaScript.

The complete unchanged official `page/expect-boolean.spec.ts` passes 86/86 in
both candidate and independent oracle. Canonical Core ledgers are candidate
1,335 expected / one skipped / seven unexpected / 47,134 not-run, SHA-256
`9ba07010bec8caae7a8a7129ae95a3b7c07684f10a03287b0572d8ca9c8b3eb5`,
and oracle 994 expected / 47,483 not-run, SHA-256
`94c37b2a46e9e86a9d1706ebe9b4d3e7397026abe4ab15d5d17b2a5accb4c705`.
Node passes 286/286, the selected host passes 709/709 with 14 explicit
environment-gated ignores, and release native-load, strict TypeScript,
formatting and strict host/addon Clippy pass. Full-corpus execution remains
incomplete.

NODE-363 adds one independently selectable package,
`pwright-api-request-connector-proxy-bypass`, plus additive Node features for
request bypass, Context-owned request clients and Context proxy retention. A
typed CDP Context-creation option carries the normalized proxy server and the
Chromium bypass list (including Playwright's forced `<-loopback>` entry), while
the bounded Rust host registry reuses the exact Context proxy for
`browserContext.request`. `page.request` has the same owner identity, live
Chromium cookies are read and written at operation boundaries, and Context
close disposes the request owner only after native close succeeds. Credentials
remain redacted in Rust diagnostics.

The `context-api-request-proxy` artifact is an exact union of existing request
features plus the Context and proxy leaves. Its executable dependency proof
adds exactly `aho-corasick`, the three proxy connector leaves,
`pwright-plugin-chromium-frame-lifecycle`, `regex`, `regex-automata` and
`regex-syntax` over `api-request`; no JavaScript HTTP client is introduced.
The unchanged official Chromium Library rows `fetch-proxy.spec.ts:64` and
`:82` pass 2/2 candidate and 2/2 independent oracle. Candidate/oracle report
SHA-256 values are
`224228d64055330d5b042456070532fc0d90751e7e9731e3783297c55a8bdcb7`
and `a5e7e6ff1b2ab5caeb3f01558e0efe669f3692681f9509d3610032deba590ed3`;
the manifest hash is
`7f87753ddc704d11eb519aaaec5b3e1eb3d4577c06fb1dc10f1e21917d5dcb76`.
Canonical ledgers are candidate 1,311 expected / one skipped / 31 unexpected /
47,134 not-run, SHA-256
`3fb4d1fef2ffe6863d192eef139e185d16c9efaea25e952f8f2e6a782170d7a4`,
and oracle 994 expected / 47,483 not-run, SHA-256
`83509103cc80e4a16b83a1dd87d0ab9dc9889ea2c383228325d5b52ae753f6c4`.
Twelve request crates pass 119/119 release tests, the selected host passes
526 tests with eight environment-gated ignores, and Node passes 286/286 plus
strict TypeScript, Clippy, formatting, lint, dependency and native-load gates.
The complete corpus remains incomplete: browser page-traffic proxy
authentication, nondefault Context defaults not yet represented by selected
capabilities and non-Chromium backends remain explicit future work.

NODE-362 adds no package or ABI edge. It adds a generic tunnel-TLS port to the
transport package, implements it in the already-selected Rustls leaf and
injects that capability into the independently selected HTTP CONNECT and
SOCKS5 leaves. The two combined Node profiles are exact feature unions; the
dependency proof rejects any closure beyond those unions. The unchanged
official client-certificate-through-HTTP/SOCKS rows pass 3/3 per proxy in
Library and 6/6 per proxy in Bidi, totaling 18/18 candidate and 18/18 oracle.
Candidate report hashes are
`8c87099c9acd9e59cd94bb040aa4b4043018e64b611d1cc426c89c420d5b6331`,
`78a650374205f5bbe74876fe627459570d761b4f4951274385318b02831a9a2e`,
`0c956deb3e2f5a6bf2ff04955005ea8e42a38d299b807dc3c65a2121b019b8b3`
and `9500fe2f5a84a55309430c922fa79e02d89f5dae94b23c317aa08f057b32b168`;
oracle hashes are
`d7506ebd85d61a3941d634fff2faf2c16bae2e84a1926d71b58cf6afdcdf681e`,
`60141488c43eae248e157ebf916f7f6c6d118056130c7a650863e631a5b5979c`,
`a313807bed3dc7a43d4b37b2821743711c04c9e1472ed92a2c9396307a66d5da`
and `d14fc8e00ed7edd1523aeb9904c28f8e2842e6f9539dc2702314cd6339e4ad7e`.
The browserless manifest SHA-256 is
`288f9bdddb43bb6961f30ed7e6799c233315bc5733ce18535775061b92d6c434`.
Canonical ledgers are candidate 1,309 expected / one skipped / 31 unexpected /
47,136 not-run, SHA-256
`cc0a0c565820c7716a806d78bfea26b8d9c477b65f12d67bbd6d90ee14355399`,
and oracle 992 expected / 47,485 not-run, SHA-256
`a28e1e3e553e05292860d374ceb368336f7a33da4bbe0ff826e6aa7cb877c6a1`.
Eleven request crates pass 113 release tests and Node passes 282/282.

NODE-361 adds two fixed browserless Bidi selections and no Cargo package,
feature, ABI or runtime edge. The unchanged client-certificate validation row
passes 6/6, and seven direct-request rows pass 42/42, in both candidate and
independent oracle. The selected native release artifact was rebuilt and
load-verified before candidate execution. The browserless manifest SHA-256 is
`a1e82102dcc296a714b39abae240c22078c1a2f5247dba08f147099a5ff8f559`.
Candidate validation/request report hashes are
`709d386639f3d22ca09cf643da3be960f5479989dede78da32b3ad0ca7d8681b`
and `1aaad17f43aeaaa0266e89ec4d6fc3eeaf1eb064ec4b9bfc51b0f8f4a1c75af7`;
oracle hashes are
`64d8d2325899c2513d2079a3f894f88e321d1c8fd6c8161bf41c560c9483c231`
and `80c69567642c724d1078c2d949ef4a185a03494ffd46d2ba9c1e11fdbbe1742d`.
The canonical Core ledgers are candidate 1,291 expected / one skipped / 31
unexpected / 47,154 not-run, SHA-256
`45e50cac8c73c0fb08a84e85200a16599d08e8549e438f1e91ad1883fca454a6`,
and oracle 974 expected / 47,503 not-run, SHA-256
`cbc14a9f83502e1495fa752f5152f667d607263f11dd0615520051b4e870ac9b`.

NODE-360 adds three fixed browserless Bidi selections and no Cargo package,
feature or runtime edge. The unchanged authenticated HTTP CONNECT, HTTPS
proxy-server ALPN and SOCKS5 rows each pass 6/6 candidate and 6/6 independent
oracle across the Bidi project matrix. The browserless manifest SHA-256 is
`eb83e9b2ac407290b477cdd09664f896d2235974813d357d99cfc354e3a12744`.
Candidate report hashes are
`62dad11b718eb7862072ae21856247363983e1bb272ec44d97dcbc6fb3d3350a`,
`80f51f86472309ce8bc785eeb90279b84e73689d06755e269eae6afdc7b734b1`
and `de2cc43b6a23d4d453cd6e0658bf09362027860ce941e2e47b91f4e811ecbddf`;
oracle report hashes are
`ee9367190de82fbc0f2e28be01894feb2a0454c16af4ea46908875d12d4fe934`,
`7ad6386ef1e37d81634a0f88cd76969a8daabc163e8951fb427aff90a591d2a0`
and `87d6d8ff090abd7e2182e690a9d6f3b6f5442a0d6a0e63fb7b7c850f214b4f18`.
The canonical Core ledgers are candidate 1,243 expected / one skipped / 31
unexpected / 47,202 not-run, SHA-256
`139a38098b6f813f7b5d5c5609edadb91217dd605698cc68b3494a4e1e29c34f`,
and oracle 926 expected / 47,551 not-run, SHA-256
`478bd8a36d8f9791f4d27fee90b99649cc1cd166947a825930ba72ed72ce7182`.

NODE-359 adds one independently selectable Cargo package,
`pwright-api-request-connector-socks5`, and one Node addon feature/profile,
`api-request-socks5-proxy = api-request + connector-socks5`. The leaf wraps
the injected connector and owns bounded SOCKS5 URL parsing, a no-auth greeting,
domain/IPv4/IPv6 target encoding and exact-length reply parsing that never
reads into tunneled application bytes.
Plain `api-request` and `api-request-http-proxy` do not select it. The first
leaf deliberately rejects credentials and HTTPS request origins.

The unchanged official `tests/library/fetch-proxy.spec.ts:133` row passes 3/3
candidate and 3/3 oracle. The browserless manifest SHA-256 is
`e014123d1cd614864880e448d79a2b409f163295b0cf47185bd76b1e5be0a7aa`;
candidate/oracle report SHA-256 values are
`19af5d94d8ccc7196d8200f229b7eafec4f0954c687dddf9c9309ef5d2428438`
and `7926ab68bdb6f9614781b8365b047d59176203adb913d5fa168a98edcbfb87e7`.
The canonical Core ledgers are candidate 1,225 expected / one skipped / 31
unexpected / 47,220 not-run, SHA-256
`8909256aa8074921a1782515809b57883b9dc2405991f0d5c6a4b37412632690`,
and oracle 908 expected / 47,569 not-run, SHA-256
`c1814cf9894b0e5b39d201c7e12962b7eb9b89f4f7d17cdcf382c1d5abe50e25`.
The eleven request crates pass 110 release tests, the Node package passes
279/279, and the selected dependency proof adds exactly this one Pwright leaf.

NODE-358 adds no Cargo package or feature edge. The selected CONNECT leaf now
accepts an `https:` proxy URL and delegates outer TLS plus HTTP/1.1 ALPN to the
already injected Rustls connector. The unchanged official ALPN row passes 3/3
candidate and 3/3 oracle. The browserless manifest SHA-256 is
`fddf3bc2a25b84c2926baf0378822634052304ab20f471074b0fe1c5b4581bc1`;
candidate/oracle report SHA-256 values are
`6dba12195eed6bf3f7ba71b3bc2713d97acf83449db4ed28e350aef5b821d74d`
and `429f9fa3ffec495127cc0f7310815d6eca336a488b74d197f74f97f11a5621c8`.
The canonical Core ledgers are candidate 1,222 expected / one skipped / 31
unexpected / 47,223 not-run, SHA-256
`9d7b1785ed238f69fbf2931d604aa5bd25e19d0076ecbd9e2a1234f61ceed927`,
and oracle 905 expected / 47,572 not-run, SHA-256
`041648606bf73c419e2d40dd967983063616593bf81d1969756f912dbd873afc`.
The ten request crates pass 104 release tests.

NODE-357 adds one independently selectable Cargo package,
`pwright-api-request-connector-http-proxy`, and one Node addon feature/profile,
`api-request-http-proxy = api-request + connector-http-proxy`. The leaf wraps
the injected connector, owns bounded HTTP CONNECT parsing and redacted Basic
credentials, and deliberately accepts only plain-HTTP request origins in this
checkpoint. The base request profile remains proxy-free. A shared source-only
official fixture adapter binds both proxy and destination servers to Tailscale
and rewrites only their infrastructure addresses. The unchanged official
global-request proxy row passes candidate/oracle 3/3; the canonical Core
ledgers are candidate 1,219 expected / one skipped / 31 unexpected / 47,226
not-run, SHA-256
`29c364c5da543b6a26da51ac48a3f9fe0d1481b1273871af3d12fbb59fde075a`,
and oracle 902 expected / 47,575 not-run, SHA-256
`a6170d5449f0024d04e9a51fe835f40de1f3e809040a68e43553b7d916cc299d`.
The ten request crates pass 103 release tests, the Node package passes 276/276,
and `scripts/check-node-api-request-http-proxy-dependencies.sh` proves that the
selected profile adds exactly one Pwright dependency leaf.

NODE-356 keeps ABI 96 and adds no Cargo package or feature edge. Rust remains
the owner of fallback matching, field-wise override composition and native
settlement. Python normalizes the already-published header snapshot to
Playwright's lowercase public map, exposes async/sync `all_headers()` and
case-insensitive `header_value()`, and retains the Rust-published effective
URL/method/header/body snapshot across later native Request publication. That
identity-only retention is capped at the same 64 live calls as the Rust route
owner and is retired on `requestfinished` / `requestfailed`. Sync Page and
Context route callbacks execute in a worker while the bounded bridge loop stays
available for nested sync calls.

The fixed 55-case request-fallback manifest pins Playwright Python v1.58.0
commit `47a5d35ef4f815a2021349f86ae391f7c20c02d6`, hashes to
`126f1bf54d8189c344eedd70a4e5d271d2bed14d7a831c8bb0a3dc88c61a5b00`
and covers every Chromium row in the four async/sync Page and BrowserContext
request-fallback files. The clean source candidate passes async 26/26 and sync
29/29, report SHA-256 values
`fcd1e06378b3b834c4d48aa32238d3df42865ced2895b9916033ac24e8f4062d`
and
`2fea136fd2b975e40ed40b068be28e85967ae129af13f2402a34051529ff89c5`.
The independent official oracle passes the same 26/26 and 29/29, report hashes
`381118f1b6a604963831996a3e2ff60e8baa8b76f3e14d19cabd364442e84b9e`
and
`003fb2ef0c59bcdf41b2a65dd54874cc5e5e1597a95f376135dc0588b3fbf0ad`.

The provisional cumulative ledger is 1,070 passed, zero failed, seven skipped
and 5,449 not run, SHA-256
`14f2e04ad35fbfaff95109c326ff6064d955d737f192bee21eea60c062623256`.
Installed release-wheel units pass 207/207, report SHA-256
`0f9c3da2197961a357420a1c6013a5fd0ebd2fd169170c51782bfb9fddbd389b`.
The two release wheels hash to
`5d204e51bc55abe004c02972651719f1f8994e533a7e1faf0a076edc42b1ba4b`
and `049af7ea076ceaf93ddf8c11ee6b11c798b6492c637aae5bce0711ca91455a72`;
their normalized payloads match except for CycloneDX serial/timestamp fields.
Ruff, strict Pyright, source validation and wheel reproducibility pass. Final
installed-wheel browser promotion is pending: the shared dedicated Chrome CDP
endpoint became unresponsive while another task was concurrently using it, and
the browser was not restarted so unknown tabs would not be destroyed.

NODE-355 advances Python to ABI 96. The separately selectable host
`context-api-request` feature is exactly `context-cookies + device-emulation +
context-http-credentials + ignore-https-errors`; the addon feature also
selects the existing `api-request` owner. `route-fetch` is exactly
`context-api-request + page-routes + route-fulfill-api-response`. Both retain
the same 108-Pwright-package closure. Rust owns the live Context cookie bridge,
bounded browser settings snapshot, typed query/body preparation, request
lifetime and route settlement. Python retains callback and public value
identity only; it does not implement a protocol client or encode wire bytes.

The fixed BrowserContext-request and route-fetch manifests pin Playwright
Python v1.58.0 commit
`47a5d35ef4f815a2021349f86ae391f7c20c02d6` and hash to
`94067ae8c5817c7c21e7e18186065cb34501a56938fb302eee6b721983958f86`
and `5c2a3ca541d9eb221a8440be377d06d6c1f35e457730999e0d4103e82722fbfd`.
Final installed candidate passes 51/51 async Context, 47/47 sync Context, 6/6
async route and 4/4 sync route rows. The four report hashes are
`8d66e0c9c0d073785e89f46715fc06c1eb3b0a4e196f69e6b65795e3596e96e6`,
`997bfe17a86e4fa3c369279632ccbcf7d7776f0aa70b6c760430dcf844361f60`,
`51b4fe021eae9743f633016443c885fd30ed8d28520f779e05ed0f0d0226d7bd`
and `37317a39bce5fde6d18b3ed23f8bc6439f22d54cc03cd305ae0549f8207a994d`.
Independent official Playwright passes the same 51/47/6/4 rows; its report
hashes are
`2596ef7801dfe83b0196097c535353c9ccb5b65d3d1a4c515dc7950b27aa8436`,
`4e0afca48b0788f412a2fef68017b40cb1eb2f8d1f1d8c2d79d0c419dc19bbe7`,
`6fbd944c9a2a22ae597db992c22f3009f040d2695b1b0a3f65801f5b3c17edf3`
and `fdaf7d44e58099673501c6aed947d6e82e561d291c14dbb1ae9cf78b33066921`.

Exactly 108 previously `not-run` rows become passed. The cumulative ledger is
1,015 passed, zero failed, seven skipped and 5,504 not run, SHA-256
`a0c33f08557de51ee7665720a393469fe3538c9c8f7aa421cc3f5b684e57e178`;
system-Python regeneration is byte-identical. Installed-wheel units pass
203/203, SHA-256
`f6211c5d3de7f848180fd9e541720fbe88ec54aca0140647712ef53c90151212`.
All-feature host tests pass 1,038 with 26 intentional ignores and addon tests
pass 2/2. Formatting, strict Clippy, Ruff, strict Pyright, source validation
and exact dependency proofs pass. Final wheel hashes are
`6662eeca620742ff25d49d2aa168f7c7a165cd30f15256180f315476c3fae43a`
and `ca86fdf2db491f38be544131f0cb8a0d542769f0105f81d482ccd937de4f9f1e`;
their payloads match after normalizing only CycloneDX serial/timestamp fields.

NODE-354 advances Python to ABI 95 with a separately selectable
`service-worker-policy` capability. Its dependency closure is exactly the
base 108 packages plus the already-defined `init-scripts` feature; selecting
it adds no package. Rust installs Playwright's exact guarded replacement for
`navigator.serviceWorker.register`, rolls back the exact Context if
installation fails and retires only that Context's init-script registry state
on close. Thin async/sync Python facades accept only `"allow"` or `"block"`.

The fixed four-row async/sync Chromium manifest pins Playwright Python v1.58.0
commit `47a5d35ef4f815a2021349f86ae391f7c20c02d6`, source hashes
`2c6cb8652304ae6a06862ca1be2bf3debbb6e1b59546e621b6653ae832f989ff`
and `ab3936debb427cc54a281eadea3ae7907409688222ac5aa676b5135ff5a54eae`,
and hashes to
`ddaf461669b0da9cbf71239b4ebeacc5b7770128eabd83eb1ff2d604ccbe03f6`.
The final installed candidate passes 4/4 with the secure HTTP fixture,
SHA-256
`48a4f35bd1a6be2620c2024a0d0e32755436910a67e4945ffb0222c56b9643a3`.
Separate-process block-policy stress passes async 20/20 and sync 20/20,
SHA-256 values
`3bff2c465dea7e4d443575e4533bb5b877cdc4f49427c085ba1ee959a2d81219`
and `d43771ed1699022b5b383b86bb6dbd0f0e9e9c4863eede42882e27860f1254ef`.
The mixed pytest-repeat async/sync diagnostic is excluded because its shared
event-loop teardown corrupts the harness; it is not product evidence.

The independent official oracle report contains four setup errors, SHA-256
`586021d0d08843052c32781ad2d00f5a0786346c5e8a2b9e22db067cf0b72da2`.
Official Playwright 1.58.0 issues `Browser.setDownloadBehavior` while creating
its CDP default Context, and Chrome 154 rejects Context management before any
selected assertion runs. No oracle assertion has therefore passed or failed.
All four rows remain `not-run`; the ledger is unchanged at 907 passed, zero
failed, seven skipped and 5,612 not run, SHA-256
`4a6c94e0a40e533243c0c19419685fdc30cb5f4fdbc9cde593917695879fc962`.

Installed-wheel units pass 201/201, SHA-256
`023ceb61089f17e1d36fa91e646aea90a7c934a9b41bb5a89fad0b925e85ac68`.
Fresh source validation and collection remain 6,526/6,526; the collection
report hashes to
`2b1ca4b5b9e816b566638c20fe082e9ce66fe3ca2edc94f652288b31d843c511`.
The Context init-script control passes 1/1, SHA-256
`45962620b65d2eaea8dd801e8c3fd03a39d90781ff6f56071792ad6822560f4c`.
Rust host tests pass 1,037 with 26 intentional ignores; Python-addon tests
pass 2/2 when Pyo3 is pointed at the final wheel venv. Formatting, Clippy,
Ruff, strict Pyright, dependency proofs and reproducible-wheel inspection all
pass. Final wheel SHA-256 values are
`585b730bc06939ad33467f9062b897ad4f749592858b38d1e62d27d5b715fe0d`
and `58c3504a3495bc82a937f9a031906535af66b3a94f4d3188def7ff0371fb3203`;
only CycloneDX serial/timestamp identity differs.

NODE-353 keeps Python at ABI 94 and repairs popup event composition rather
than adding a second lifecycle path. A popup blocked by an opening dialog may
publish its provisional Page, popup link and Dialog before recursive world
initialization settles; the same exact-session initializer resumes after the
dialog closes. Immediate-close console handles settle at the observer boundary
before detach compensation, and host teardown revokes all still-retained
Dialog actions so their CDP clients cannot keep the transport alive.

The fixed 12-row BrowserContext-event manifest pins upstream source SHA-256
`3fc5a5edcfcd82548a376039812a40da6c8b1b4c94fced6f806d36457fbec927`
and hashes to
`a0715af7ce9c9558df72b0823dabcb8e4a4f8161638ae19eb32b7b001f8fcc61`.
Final installed candidate and independent official Playwright each report 11
passed / 1 failed. Their shared failure is the unchanged JavaScript-URL popup
assertion that expects `dialog.page is None`; both implementations expose the
popup Page on Chrome 154. The clean 11-row selections pass 11/11. Only those
11 rows advance the ledger to 907 passed, zero failed, seven skipped and 5,612
not run, SHA-256
`4a6c94e0a40e533243c0c19419685fdc30cb5f4fdbc9cde593917695879fc962`.
The source/oracle-drift row remains `not-run` and is not counted as a product
pass, failure or skip.

NODE-352 changes no runtime code. Its eight-row BrowserContext cookie-read
manifest pins upstream source SHA-256
`1e8ffedbc7fa8588b6d73b07d53a7173ae1b09b4c35021eaf51dbe6d3b6b213d`
and hashes to
`922304de6c73bdcdf4eb90e3edcd21a6580a5c2f125097ff95238d31ec50ae79`.
Final installed candidate and independent oracle each report 5 passed / 3
failed, with full-report hashes
`00d579903e77ed04611ceff854d00e0fa6990b357ae021162825fc8460edafd2`
and
`42b09c172684640e738f9ed215c4bca91b6155326438d14c51548c15cdf8bbcb`.
The failures are identical `localhost`-domain assertions under the required
MagicDNS fixture. The clean five-row candidate and oracle selections pass
5/5, with hashes
`7accee72800f3168d5a19d370d671a62bfc5aa3c26382364fab3160ce699ba20`
and
`08cfe9bc6366a47e10e5b24df97c7a2f38f31978325b3663225da405744bbafd`.
Only those five rows advance the ledger to 896 passed, zero failed, seven
skipped and 5,623 not run, SHA-256
`330786f82e6d9682e744904c6a3a985b1a25f3f9711db1dd851636fb139dc3b4`;
its summary hashes to
`28fbde77c43c733d2b9168ea3551e1d1047427a4d79d9efd28a9187c2a498d39`.
Installed-wheel and system-Python ledger generation reproduce byte-for-byte.

NODE-351 changes no runtime code. Its seven-row BrowserContext clear-cookie
manifest pins upstream source SHA-256
`f2e002736d5f0b5c813b3fd2a7b79e9cd98e275db680a9bf492dd71bd07cbec6`
and hashes to
`d83292f138c49e3a0c2734dc27e0a3f0ef152dc39b55d6267327688b0bfaed4e`.
The final installed candidate and independent oracle each pass 7/7, with
report hashes
`23ea53565b1aee911f4bb7e7fa7cbbd9fe5ce9e9700e7f8bcda292034565d74c`
and
`b0ce0b93dace22054fd92e9638c8a30748d78d4250a8c3f9c171bf613d98991e`.
The cumulative ledger is 891 passed, zero failed, seven skipped and 5,628 not
run, SHA-256
`fd8aa762af6f316feb60d2f9a777ba5432c02f4256b01baf0f5085dff7b962c1`;
its summary hashes to
`78620004ebf6e357f37800ee258453fcaca1e864e0fd064f8edbdfe8b5813c6f`.
Installed-wheel and system-Python ledger generation reproduce byte-for-byte.
Full final-wheel collection remains 6,526/6,526.

NODE-350 advances Python to ABI 94 and corrects its native error hierarchy.
Rust still classifies `PWRIGHT_INVALID_ARGUMENT` and owns the bounded error
message, but `_native.InvalidArgumentError` now subclasses Playwright's public
`Error` instead of Python `ValueError`. Pure Python signature/type preflight
continues to raise `ValueError`, preserving the language boundary. A direct
AAA unit test pins the inheritance relationship.

The source-pinned 19-row BrowserContext add-cookie manifest hashes to
`7c2cdf04985d4c7b869dc5ad79557a8383bcd499f2946388d4fa83f9a4109ca0`.
The ABI 93 candidate baseline reports 15 passed / 4 failed, SHA-256
`b3b9f3b81f92c4cadb4775f8ec105db5bf691c338974023b743b8e9d66905484`;
the independent oracle reports 17 passed / 2 failed, SHA-256
`9ef326fa7c755f545ebcc02ff7ac41f5bcee7eb16d1fbb1841e1de191744e034`.
The two candidate-only failures are the blank-page and data-URL invalid-cookie
rows, whose Rust messages were already exact but could not be caught through
the official base class. Final installed ABI 94 candidate reports 17 passed /
2 failed, SHA-256
`33d7f504f75363d4f76c6910a732739a9ec5ffc68544dd35828f4ade4071d177`.
Its two failures exactly match the oracle: a `localhost`-scoped path cookie is
not sent to the MagicDNS host, and the reasonable-defaults assertion hardcodes
the `localhost` domain. The clean candidate and oracle selections pass 17/17,
with hashes
`fb9f72a89c2c687fce19f4a693bc4b370cf511dfb45a2c94914af99c24700199`
and
`bbc72c07709dbfd408ff5200f267c408bb4994a8edf767624a722f6a19684283`.
Only those 17 rows advance the ledger to 884 passed, zero failed, seven
skipped and 5,635 not run, SHA-256
`193d0cfe5e6674effc3bd876ec3fc8299b7e0256986554a3785f66f9f4c0850d`;
its summary hashes to
`f99969cc4f4e1c36d58efb123100b7bf0a94534cc87bd5f5f93d33e5c3302d65`.
Both ledgers reproduce byte-for-byte under the installed-wheel interpreter and
macOS system Python.

Installed-wheel units pass 196/196, SHA-256
`c8c3ddc6ba97a34381171b805ba6c19f09b23e13b50b4e0ca508d711dfaa270e`.
Full collection passes 6,526/6,526; its JUnit and complete collection-log
hashes are
`0833ff6a28455892b9b4545944c5af23368446042957444e708496ef8e33ff25`
and
`5bc922a98486b97894957b54d6eeaddc692ab7e7f6272bb201715214c72a10e2`.
Repaired wheel SHA-256 values are
`9f8b9a615e57c024916e3d6768cc6f735d39a9d2229abf8c2d00c7ff54e2fbdf`
and `1923b6d482c6c59796bebc25d7e54ff063659af7cf658352569cc7ca1055627d`;
their payloads reproduce after normalizing only CycloneDX identity. Formatting,
Ruff, strict Pyright, release check/Clippy, the two addon Rust tests, source
pinning and the 108-package dependency proof pass. The two MagicDNS rows stay
`not-run` rather than being rewritten or reclassified as product passes.

NODE-349 keeps Python at ABI 93 and fixes duplicate Page-session ownership in
Chromium topology. An explicit Page is first synchronized through its manual
`Target.attachToTarget` session. If browser-root auto-attach later reports the
same target, Rust now detaches that later session before a second Fetch
observer can publish the same request. The existing bounded initialized-
session table carries the discovery bit; no parallel registry is added, and a
true discovered-Page replacement still passes its regression test.

The fixed BrowserContext-route manifest hashes to
`5a3fb169be43b5d0a7b3daebd422de7031e30d20d8dcf5c6a4485cdbeea05d94`.
The final installed candidate and independent oracle each report 8 passed / 1
failed over all nine rows, with hashes
`f547df09ef65e66c8c6d7e57c0a704d77099359eafffc9c03f61fc0d7edac559`
and
`12d22f336a5efccfeeeafd079066d8cb93c1728ad3a529f0019941c85a09671e`.
Both failures are the same fixture-domain applicability issue: the actual
cookie domain is `macbookpro-yuacx.tail8f3b66.ts.net`, while the unchanged
upstream assertion expects `localhost`. The clean eight-row selection passes
final installed candidate and oracle 8/8, with hashes
`8199affe3ce2c9cf55b5894484a4df0d5bfbd9f3c60dcd598dce5f8b5f03fa67`
and
`526e4502eda4fff5e301fc459c7fb50112e743e8f27aed41fd0756f290dfedbb`.
Only those eight rows advance the cumulative ledger to 867 passed, zero
failed, seven skipped and 5,652 not run, SHA-256
`2b6b9e6dc39344b6a86b334473b3860dd0207c49686a6bcd610e9a06b1bb8b3f`;
its summary hashes to
`3074b6a1098a9339e4e56856559087688d9083281763e0da7fea5dc8699bc4c6`.
The cookie-domain row remains `not-run` rather than being reclassified.

Installed-wheel units pass 195/195, SHA-256
`71839549f9a9b5a5630068a7c55d7b2b5f7c630c27de703ea9a57b4305b16e61`.
Final-wheel collection passes 6,526/6,526; its JUnit and complete collection-
log hashes are
`a0ff96069b525b81da67813ea35d9e7a59287b7814641ec8e47042d5ccaf320b`
and
`ba3d1157702498b1f991bad4c977b53b3d316ae1298f9fc193cb7bab01518766`.
Repaired wheel SHA-256 values are
`350405dc638873ec17c4890a85b9492e2a693cb843a05e629b34c17fc06b26fe`
and `e01417e339557ea835e3773763d7769e179a4413170326fcc1a94854d8e8fc52`;
their payloads reproduce after normalizing only CycloneDX identity. Focused
topology tests, release check, strict Clippy, source pinning, all three
dependency proofs and the 520-pass Page-routes host profile are green. The
complete composition binary reports 403 passed, three unrelated current-
worktree failures and 16 declared ignores; it is not reported as a green full
suite.

NODE-348 changes no runtime code. It fixes the five remaining async Chromium
Page-route rows in a source-pinned manifest and executes every row in an
independent pytest process. The manifest hashes to
`90045191709fd968ecd2759f1203ddcc10fc2a357aab8694a99144cb70378c73`.
Final installed candidate and independent oracle pass 5/5, with report hashes
`05acf90a02672a0801ce6bff512961f876795b1984caa4390cad3d49c891f8e8`
and
`3582d4e1fc6da9a79a2239f5a54b439df965633f0fd80dc8fd40a33ef97f5cf8`.
The cumulative ledger now records all 44 async Chromium Page-route rows as
passed and contains 859 passed overall, zero failed, seven skipped and 5,660
not run, SHA-256
`2f75e41c034568da65f7e0cc10fbac364a54f5778dc9ccc6ea5c875778435194`;
its summary hashes to
`937bcee4bab6d530df9637877082c920566e28ac7e6a217ff8a20b98b36f44c1`.
The ledger reproduces byte-for-byte under the wheel interpreter and macOS
system Python. The earlier whole-file run that lost CDP responsiveness is
retained only as diagnostic evidence and is not recast as a completed run.

NODE-347 keeps Python at ABI 93 and closes the selected Page-route
service-worker-precedence discrepancy without creating a service-worker
owner. Chromium topology now calls filtered `Target.setAutoAttach` at the
browser, discovered-session and synchronized-session boundaries. The default
selected types are `page` and `iframe`; `worker` is added only when the
selected frame observer retains worker targets, and a final catch-all filter
entry excludes every unselected class. The fixed manifest hashes to
`3a457d2007b5d0c04c0bcf74edf40f421f44ac96fa89dd4790759f5c66cf4f76`.
The final installed candidate passes the unchanged official row 1/1, SHA-256
`c6989b441f7078268b3c874511a7bda9f3ff5fac6a3b16f01f00383779ae1453`.
Installed-wheel units pass 195/195, SHA-256
`0cac7ecf1ebf20f7d2d0a81685ebd3196354bdbb06f2ce0696296b1ccf45c565`.
Full collection is 6,526/6,526, SHA-256
`c00afb74aceaaf561f9bfb0b1eda9c4802f5b5cb931bd19441eb5e42740579f8`.
The cumulative ledger is 854 passed, zero failed, seven skipped and 5,665 not
run, SHA-256
`5bc1fbb17929fb0dab2845f3041317382b3b7cab906d7d024198228b9d51b850`;
its summary hashes to
`06a52762ce5352a2d741645a99b116e1cbf9b9f8046eb93051eebce61ddd5c8a`.
Repaired wheel SHA-256 values are
`cfe2f625bfdfdcd2a43f5a50284f776cec7d7d9f89d1b5f75bf801799733e83a`
and `d02fe4cec54133d7eb9e84f7e41e8faac8fbf346e37bf774ec79316eaf556935`;
their payloads reproduce after normalizing only CycloneDX identity. Strict
Rust Clippy/checks, focused topology tests, Python format/lint/types, source
pinning and all three dependency proofs pass. The complete composition test
binary reports 402 passed, three unrelated current-worktree failures and 16
declared ignores; those failures are not hidden as a green full-suite claim.
The official oracle attempts are excluded because a stale service-worker
target without a browser Context ID aborted connection. A later complete
Page-route diagnostic passed 24 isolated cases before CDP became unresponsive
during an unrelated redirect-header case, so it contributes no 44/44 claim.

NODE-346 advances Python to ABI 93 and closes the two Page-route redirect-chain
discrepancies. The separately selectable `route-redirects` feature is exactly
the dependency union of `page-routes` plus `request-history`. Rust
auto-continues redirected pauses without another callback or finite-use
decrement, retains at most 64 explicit header overrides, removes `Cookie`
before replay and retires terminal chain state. Exact native request-history
identity backs thin Python `redirected_from` / `redirected_to` facades. The
fixed manifest hashes to
`0d745e50edccc9db198df552fd0eae83947c53e2a4a6706326ae7d873f6eef14`.
The exact ABI 92 red report fails 2/2, SHA-256
`5395181269a7dace0d878dfd9f91b7ff88fb5183175b08f212b5ecf2f707edd8`.
Final installed candidate and independent oracle pass 2/2, with report hashes
`dcd31ae1b4590970cb7c46d5aed5df923902ecbefe6983e1dd460a759e9d2339`
and
`a96f71c0e5b63e71e37c02df982895951ba63711f68c47f2bf910a04a08153cf`.
The complete Page-route file is 43 passed / 1 failed, SHA-256
`3f3ef3d3f0f8b3823451e4ce8f76129e9a43d324ff482a8fa2ab3f813822cac9`.
Installed-wheel units pass 195/195, SHA-256
`47e070ff47cbdbce1b40c79de85677a4818e89cb6c5ac027003c9b863f1ab500`;
all-feature host tests pass 1,030 with 26 declared ignores and addon tests pass
2/2. Focused tests, strict Rust/Python, source and exact dependency-union gates
pass. Full collection is 6,526/6,526, SHA-256
`d8744abdf5035700aef6b32d821c0fe51fbc168c52c64c208dc2c032cb3f0961`.
The ledger is 853 passed, zero failed, seven skipped and 5,666 not run,
SHA-256
`f4dca295f673c6b680020e02c4f99d8a7a8cd2dd261696dcdf7638a32ce9a362`;
its summary hashes to
`d18de51495c3cd3c109afddedd418f78e3c65782587ad1a0f3c8f314915bc0fc`.
Repaired wheel SHA-256 values are
`95512f3ea04a9c8147fe6441acae1b95705d00b99597cf77186a4c62bb2bd4bc`
and `9833b1fa82f1659adb555f627fce2a8f302627006a51526670c93b728da8fcf2`;
their payloads reproduce after normalizing only CycloneDX identity.

NODE-345 advances Python to ABI 92 and closes the unchanged global-fetch
`Route.fulfill(response=APIResponse)` discrepancy. The separately selectable
`route-fulfill-api-response` feature is the exact dependency union of
`page-routes` plus `api-request`. Rust inherits status, headers and content
type, rejects bodies above the existing 1 MiB route bound before cloning and
applies the existing route action; Python passes native response identity and
owns no body copy or registry. The fixed manifest hashes to
`1a1ec11a564aee020317fdcb62e220fbe5b6f950e1687622e2f428b8d737f42c`.
The exact ABI 91 red report fails 1/1, SHA-256
`6e224bf6ebca1f40440f79062103e8c32dc13c357078570e5e1506889219f300`.
Final installed candidate and independent oracle pass 1/1, with report hashes
`1f03ff84db73ce99fe0b3c60122d087b1682a7431121318889771336f7354a2a`
and
`ee4517f65b628199cc353ae00aa31837280ab452d201e2121cd44d69aa47eed8`.
The complete Page-route file is 41 passed / 3 failed, SHA-256
`8362386e58f0fcf6641ef5fdb80410aa840f516a7335be84da3e973dd7f8c76e`.
Installed-wheel units pass 194/194, SHA-256
`e2bcb1f6dfa243b1ca3a561f13a6240b9ff20f6a8b9325cea0366bc02cef3bad`;
all-feature host tests pass 1,026 with 26 declared ignores, addon tests pass
2/2 and the API-request crate passes 22/22. Focused tests, strict Rust/Python,
source and exact dependency-union gates pass. Full collection is 6,526/6,526,
SHA-256
`1c0a18e6a8f361627a2fba0bd1ebff8e9c64b6480289865feceb5717ec01f26d`.
The ledger is 851 passed, zero failed, seven skipped and 5,668 not run,
SHA-256
`09089b9c798ccf3a4e3f93645839257783b16a39c03b8c737ade9f34b0d40d48`;
its summary hashes to
`f04485e49607318120ef4753ecb61f902187eb8ddc7ac571ddffb8a96fdc399a`.
Repaired wheel SHA-256 values are
`20cddd19a6d96ab92b9c93d4d24b27c473cffb67682ecbef6f98e3a6b343c041`
and `a171ac67d754fe24767c4a9d65dba550fb42bb62d687db885b1cd4c48edf5b3c`;
their payloads reproduce after normalizing only CycloneDX identity.

NODE-344 keeps Python at ABI 91 and closes the two browser-generated CORS
preflight discrepancies without exposing an extra route callback to Python.
For a matching registered route, the Rust route owner recognizes a bounded
`OPTIONS` request with `Access-Control-Request-Method`, returns Playwright's
204 response with the echoed origin, method and optional requested headers,
and preserves the user route's finite-use budget. Ordinary `OPTIONS` requests
continue through normal routing. The fixed manifest hashes to
`8c17ca7e9acffb0fdad0f3523a4f83eadf2f3649371a3c42d889f75e921c01d2`.
The exact ABI 91 red report fails 2/2, SHA-256
`87541e590afec054f4d0d6fbc49ddd80104de19c6d55e119f1f8726ca3081694`.
Final installed candidate and independent oracle pass 2/2, with report hashes
`61ae975f214727ccd92c77ba760d8e2b6b9936d1a249c32091c07fcb99d8ee4e`
and
`f596e5b7e0c4b44df14a7f2c15e86396e81a0c3e87661a6d2a31cdbc03dd2991`.
The complete Page-route file is 40 passed / 4 failed, SHA-256
`91158df512ed9ba3b86c511b037758ffb6e82c193380604849622cd11ac8da5d`.
Installed-wheel units pass 190/190, SHA-256
`3c65b99599a6969e81bf1e769f911c8681c55dcd1bed7f2a0c8d5e028dfd5590`;
all-feature host tests pass 1,026 with 26 declared ignores and addon tests pass
2/2. Focused tests, strict Rust/Python, source and 108-package dependency gates
pass. Full collection is 6,526/6,526, SHA-256
`c9150dc8b476679a5b659cceafd84cdf6b7093f34df891bed514f6d5ff2bf132`.
The ledger is 850 passed, zero failed, seven skipped and 5,669 not run,
SHA-256
`5a0465a5fdeef895cae08b30b2babca1b794b28474a5cf32118962e1556ab0d4`;
its summary hashes to
`10cfc323fd8bed8b02d91d3ca0c7a1f8f5db2485b36b93dc9942df92340c32bb`.
Repaired wheel SHA-256 values are
`60468918bafa82f30f78a95268939ff118de0c5286d8bdf8ffc6e4c2ac0a16b5`
and `fda32335a6717e0ae4a67ad15ba91e6ddd72b3938b3edaffb977c88af60f2bbb`;
their payloads reproduce after normalizing only CycloneDX identity.

NODE-343 advances Python to ABI 91 and closes the routed
`Request.response()` encoded-URL discrepancy without introducing URL matching
or a binding-side registry. A route call carries Chromium's bounded
`Fetch.networkId`; the Rust request-history owner waits on its existing
bounded records for the exact native identity, observes Page/session
termination and returns the settled response while Python preserves the same
public `Request` object. The fixed manifest hashes to
`bb1d45e9ed7cce9eb55e00f1b90d07c96cb6f8d6fcf3b772f6844a2229afe83f`.
The exact ABI 90 red report fails 1/1, SHA-256
`e49f5cc30cbdcbe4a5f17c52b354d5719eaf859917602d160790887f1e752492`.
Final installed candidate and independent oracle pass 1/1, with report hashes
`1b35b052b10603a7a11d2976e8b7dbb71d23781919e99f3023317170c1ab3142`
and
`be3df59d21eac14f3587b90a52d1df8291b5ac7664a8794091c3c29b49bb4d3f`.
The complete Page-route file is 38 passed / 6 failed, SHA-256
`876cbfb7d06cbdff10d5485425f3d0ddbfed8f3c175fdf784c1a5862a68ea108`.
Installed-wheel units pass 190/190, SHA-256
`37c173a84740b5e8f628c01477344a0ebac431f31d772cdbda583ca920baa61f`;
all-feature host tests pass 1,023 with 26 declared ignores and addon tests pass
2/2. Strict Rust/Python, source and 108-package dependency gates pass. Full
collection is 6,526/6,526, SHA-256
`ce7d3084b28837993310a53bb6635d49d25cdf60f042f683c645321fc6ee6bd4`.
The ledger is 848 passed, zero failed, seven skipped and 5,671 not run,
SHA-256
`58c32ca19ac776657dcfd4e823ca58d9d57acf519b5c47d371f3c2deb12520d3`.
Repaired wheel SHA-256 values are
`8854082265f32fb36a8de0b8507fcde80a354b7c8968b88cb8d4e18ad6b138e4`
and `b7bbd2251e26ee574d9521c87b0418b74955e1c7451277c3fb99f0ca89d5544d`;
their payloads reproduce after normalizing only CycloneDX identity.

NODE-342 advances Python to ABI 90 and closes two more unchanged Page-route
rows. Regex and predicate filters retain Python matcher identity while a
bounded Rust catch-all route owns fallback traversal, settlement and deferred
native retirement. The separately selectable `route-fulfill-path` feature
reads only regular files through async Rust I/O, bounds paths to 4 KiB and
bodies to 1 MiB, and derives `Content-Type` from the extension in Rust. The
two-row manifest hashes to
`4090d9b980004ab051b3f3078d4fee0fec7f2360fc7c7d802788cd8b45e72dad`.
Final installed candidate and independent oracle pass 2/2, with report hashes
`35a123a706cf2c645b7db4afab7d67b50f8253aaa76b359a13b508fb2289d451`
and
`0ba256ac00477dd62dee767a5e7b513e2e49fc60e889629e5382a4b318304d35`.
The complete Page-route file is 37 passed / 7 failed, SHA-256
`489b44b15c599c0e7bf7ae83c5a6d5057fded99c8ee8f23b30e57424451c3dfa`.
Installed-wheel units pass 189/189, all-feature host tests pass 1,022 with 26
declared ignores, addon tests pass 2/2, and strict Rust/Python, source,
108-package dependency and normalized wheel-reproducibility gates pass. Full
collection is 6,526/6,526, SHA-256
`705b261e7ad2ded37e160f55c8efb4ef226121869df74d636b1c3e7395567254`.
The ledger is 847 passed, zero failed, seven skipped and 5,672 not run,
SHA-256
`e423606ad57b9f2e0bdb95f4e4a74e71d65ea9582890b864e86ba76837817028`.

NODE-340 advances Python to ABI 89 with exact Request Frame identity across
ordinary network events and Page routes. Rust owns bounded native Frame
descriptors and cursor-relative availability, while Python resolves the one
portable ID through the existing Page Frame registry. `Response.frame`
delegates to its exact Request and `ElementHandle.content_frame()` reuses the
existing Handle-to-Frame owner. Final candidate/oracle pass the six-row
request-frame manifest 6/6 and the route-frame row 1/1. The full Page-route
file is 35 passed / 9 failed, installed units pass 185/185, full collection is
6,526/6,526 and the ledger is 845 passed, zero failed, seven skipped and 5,674
not run, SHA-256
`673973f03e77d0291596cfbc971579c70d98831f634ab74606a43b37282375c7`.

NODE-339 advances Python to ABI 87 and extends the existing bounded Rust
request-event owner from request-start notices to the complete Page
`request`/`requestfinished`/`requestfailed` trio. Terminal failure text and
exact native request correlation stay in Rust-owned records; Python reuses the
portable request identity. The fixed two-row manifest hashes to
`f5cb093c7321da1f8a0ad597fb108cbe26f636dabe37956495cefc2ce3c5e0d4`.
Final installed candidate and independent oracle pass 2/2, with report hashes
`daa8311542664a25242b7c0877c98b03302b5b7542f13c0b05c4ca6cad050811`
and
`a8f35888e8ba83c2b8eca51df9ef6b975c860084a77ebdf571a502d86c1b281c`;
five candidate repeats pass 10/10. The complete Page-route file is 34 passed /
10 failed, installed units pass 182/182, the all-feature host passes 1,043
tests with 36 declared ignores, strict gates and the 108-package dependency
proof pass, and repaired wheel payloads reproduce after CycloneDX
normalization. The ledger is 839 passed, zero failed, seven skipped and 5,680
not run, SHA-256
`b8c7bebbeb3f2c16880fb7fc08dd30b15d56d504d091c264b732641ef8e81e0f`.

NODE-338 keeps Python ABI 86 and adds the Playwright status-to-reason mapping
to Rust-owned `Fetch.fulfillRequest` serialization. The one-row manifest
hashes to
`5db202122947113cacf31d4ac4854f9c6b2715d092a09cd36efd1303936a0d79`.
The exact ABI 86 red selection fails 1/1, SHA-256
`64085cc07ee62904297a5ebd94dc72aff436dffa04fe2c7d9338d9a2779e2614`.
Final installed candidate and independent oracle pass 1/1, with report hashes
`03c00a71727f7ec0bf7d0e33c902176c9a079a79b0888c5af87a138dc057b3d4`
and
`40cc799e2e10b0938defd95d5e22de0b4aafb9e34711f9e23a3d211455473cba`;
five candidate repeats pass 5/5. The complete isolated Page-route file is 32
passed / 12 failed with no regression, SHA-256
`b1cbd5689624ca8b3e24a5c301d5522b4a511781f0bbbd58dca02ba9611e6769`.
The `page-routes` host profile passes 517 tests with eight declared live
ignores, installed units pass 180/180, all-feature strict Clippy and the
108-package dependency proof pass, and repaired wheel payloads reproduce after
CycloneDX normalization. The ledger is 837 passed, zero failed, seven skipped
and 5,682 not run, SHA-256
`8a6d6a9169086e482fbd8acf6d81162ef42ab209f18c2c174238e5feb85e2ef5`.

NODE-337 keeps Python ABI 86 and changes no runtime code. The fixed native
`Referer`/`Origin` public-shape manifest hashes to
`1e705cd72b24e45d93d4242d8698c43d55c302c2676fcb0736beaec1c1b724ce`.
The exact ABI 84 red selection fails 2/2, SHA-256
`70f093d70526a1aa906dd0a64f044f230993dccc0db334b69bb1ee5fc71c0f9a`.
Final installed candidate and independent oracle pass 2/2, with report hashes
`c40e734175d114ff40ba403eb90072d38bc2f20fceb90e484f7ef5eaa38f6676`
and
`ea9d449ef2876572f092f361adeff15c70ee982ae7292edc979b55385d9cc4f2`;
five candidate repeats pass 10/10. The complete isolated Page-route file is
31 passed / 13 failed; the two known unstable green rows are not promoted.
The cumulative ledger is 836 passed, zero failed, seven skipped and 5,683 not
run, SHA-256
`eb6b687f25461fa8d8f4182455215dbcf565a47296427680357866df79cc8553`.

NODE-336 advances Python to ABI 86 with thin async/sync Page and
BrowserContext extra-header bindings over the existing bounded Rust owner.
The fixed three-row manifest hashes to
`53eca6f3debffcd3cc338be1fc229effc2e5d6440c3087060f76bc1fb3b40c86`.
The exact ABI 84 red report fails 3/3, SHA-256
`774e9c951f6ce2f2e784f6a6f4d5d1fb984bd31c8666f1fc5b88007a385be4fd`.
Final installed candidate and independent oracle pass 3/3, with report hashes
`ac965349486711d65999665466d6509c4fdc76eb6347c563bc47bb4488f98cd1`
and
`9c6b093150fac06db4032e62446601a7510035f96782967ae04e50b6363135e5`;
five candidate repeats pass 15/15.

The cumulative ledger is 834 passed, zero failed, seven skipped and 5,685 not
run, SHA-256
`a0c1caa9d38619302ae8b0eaa49450d1c9db9a5db80cb44e4fa6fb5e69c4b783`.
Installed units pass 180/180, SHA-256
`e900850fb3d13d3763e58db2bb8414e84597724700c692f884104d730961b770`.
The exact composed host profile passes 552 with eight declared live ignores;
the all-feature host passes 1,018 library tests plus 22 integration probes,
with 26 plus 10 declared live ignores; addon units pass 2/2. Strict Clippy,
formatting, Ruff, Pyright, source pinning, the 108-package dependency proof,
normalized wheel reproducibility and full 6,526/6,526 collection pass.
Collection SHA-256 is
`88818b94cdd28d394110a631df95a7d31f272db6da91320ebb626e9ef5ec26a1`.
Repaired wheel SHA-256 values are
`d93da13b571f0e7dd98a772ebdbe742018fa04c1f372e047b5a57318315e1828`
and `12e057935602b03ae915d66b4f96fd00087490a1f5ff4355e5ae0cbc476cd512`.

NODE-335 advances Python to ABI 85 with bounded Rust-owned fallback traversal
and handler-specific Page/BrowserContext unroute. The fixed eight-row manifest
hashes to
`ae8c779a13000d341ed5a6ec32c82249c096f2ac0bfb317f5df2a7fd309676c3`.
The exact ABI 84 red selections record seven failures and one pass; Page and
Context report SHA-256 values are
`dc8b0f6daf9036adeba4cf8ea439a85fede9d72f4770644eaa16c707be01e787`
and `8fcfb19ada84f4004f458ff21122416a03ee86ea1ed5d7506c15511bcb0dcb34`.
The independent oracle and final installed candidate pass 8/8, with report
SHA-256 values
`be6df6716b3b913e20d40a0cf94185ecdf6d750d5ca67489a7bc46afbc2fd379`
and `07784087e2516f57e39a0d5567c2fcc8266c58842f6d6f4fb939c0e67734179f`;
five independent candidate repeats pass 40/40.

The cumulative ledger is 831 passed, zero failed, seven skipped and 5,688 not
run, SHA-256
`4303259b2a6813aa7e3fc8017d89d1cc5d62a38bed6b66eacc3af5600d576722`.
Installed units pass 176/176, SHA-256
`8f38b5ad24b08b483aa0c536ec7b218b4fe6125da59252d6e945ff50ea639b87`.
The all-feature host passes 1,015 library tests plus 22 integration probes,
with 26 plus 10 explicit live ignores, and the Python addon passes 2/2.
Strict Clippy, formatting, Ruff, Pyright, source validation, the 108-package
dependency proof, normalized wheel reproducibility and full 6,526/6,526
collection pass. Collection SHA-256 is
`985e35d5b9645cf76942d0922d72231beb7ec8b43cd188094fdeca09dac4dcc6`.
Repaired wheel SHA-256 values are
`4039442d77a5fcb04f11a8da765b98bac70e315c40fb010172a674e70acd4c0d`
and `d71031952acd05d0286f0baf2ce9659036f47ef9898e5eec5e0d951933e67995`.

Rust owns the traversal cursor, effective request overrides, rematching,
terminal settlement and Fetch refresh. Python owns only public callback
identity and wrapper invocation. Regex/predicate matchers,
`unroute_all(behavior="ignoreErrors")`, remaining Python and complete Core
JS/TS coverage stay open.

NODE-334 advances Python to ABI 84 with Rust-owned finite route-use budgets.
The fixed three-row Page/BrowserContext manifest hashes to
`9c90aed2f4cb8c20c3d61bf5905935d8d7bd2a5d87e7634208c346713f4435db`.
The ABI 83 candidate fails 0/3 while the independent oracle passes 3/3;
report SHA-256 values are
`08ca9be33001a14352dfab498a58370a45f74cdca9fde1f4c4b5d2ce6f675c80`
and `c980906a95028171bb31e63cd68a9443c53476f52da01dd68c0187a179d9feba`.
The final installed candidate passes 3/3, SHA-256
`d8e9a73a823ebd2134de4b21dfaed9687426af02e17572d6ce386d35b81e0faf`,
and five independent repeats pass 15/15.

The cumulative ledger is 823 passed, zero failed, seven skipped and 5,696 not
run, SHA-256
`26023955d6cd01d96099cf7b07ef479d028e86a9fd60038e14c786479f467439`.
Installed units pass 173/173, SHA-256
`49c9ae5de882c25efe2993c331ef299c41542804e3e733e9e1a99eb997cd5e6b`;
the all-feature host passes 1,011 with 26 explicit live ignores. Strict
Clippy, formatting, Ruff, Pyright, source validation, the 108-package
dependency proof, normalized wheel reproducibility and full 6,526/6,526
collection pass. Collection SHA-256 is
`1bb48d3ca72bd53ff1cda493a1a58f3250cf1295196e194ea4c514ae0e2e5acc`.
Repaired wheel SHA-256 values are
`9f8328d6c8411fe8587e0824ba0085b748b4c026c14a2ec3bf271b0b42eb3b92`
and `fc41f6950008b306a323595d00f57e52f501de26ae0b6299a5aec69fe345b961`.

Rust atomically decrements/removes finite registrations, refreshes the
affected Page/Context sessions only after the successful terminal action and
continues a transient stale-pattern pause natively. Python receives one
expiration bit to release callback identity before invocation; the direct
negated assertion aliases delegate to the existing `.not_to` implementation.
Regex/predicate matchers, route fallback and handler-specific unroute remain
open and no such row is reclassified.

NODE-333 keeps Python ABI 83 and makes `?` literal in both Rust route matching
and bounded CDP Fetch-pattern compilation. The fixed one-case manifest hashes
to `9b8fda4e880a63e197a938b79cc32816f04a77ef56c5e167137083f5007b19ca`.
The red candidate fails while the independent oracle passes; report SHA-256
values are `e18f320aad2fdc7d251ebdc778151c6236ba07d7c006fd41c7cc79cc90bc0428`
and `151de8226947e0f5b3c6a1c5b1678b334acca6b415e24a15986fc57c443051f5`.
A matcher-only intermediate correctly exposed a Rust/CDP pattern disagreement,
SHA-256
`e92ca82802ab51fd848ec1362f83a2e7339a523f725af2470dd5e99b3927093c`;
the final installed candidate passes 1/1 and repeat 5/5, final report SHA-256
`54ddbab3174694286c83fa4b3c44c4e133ba507aa28ea734a929a532c625d0bd`.

The cumulative ledger is 820 passed, zero failed, seven skipped and 5,699 not
run, SHA-256
`aaa91984391a096225cae1684c0f1879249b41feedd8efad7a5203053087e6d0`.
Installed units pass 172/172, SHA-256
`234f7f3a8aad8d520cf326493de4fca456d9b2c12b061c3cd1e1dc17f1e9dd6f`;
the all-feature host passes 1,008 with 26 explicit live ignores. Strict
Clippy, formatting, Ruff, Pyright, source validation, the 108-package
dependency proof, normalized wheel reproducibility and full 6,526/6,526
collection pass. Collection SHA-256 is
`a824e7875ad974b05de1dd79b14e897fc9ed9ad7823962aeba700a91fa96b80b`.
Repaired wheel SHA-256 values are
`23ebb82ef7e1b4eea5e7e332083464e5c004ae0be1be08f0966e2caf055859cf`
and `15b1e9d884678d65676b1d492031840fba1152c1cfbf285872023dcb14e3513c`.

The final 44-case Page-route diagnostic records 22 passed / 22 failed,
SHA-256
`14dd384d5c67a345acc0a7f95edf6d1acc50a6438d70604f734728ced2830c44`.
The two apparent losses relative to NODE332 reproduce on the unchanged
NODE332 wheel in contemporaneous controls; they remain explicit unstable
diagnostics and are not reclassified, skipped or booked as NODE333
regressions.

NODE-332 advances Python to ABI 83 with bounded Rust-owned
`Route.continue_()` URL, method, header and POST-data overrides. The fixed
ten-case Playwright Python 1.58.0 manifest hashes to
`3a53fb8247e6e84bf7b4bfc29d332dd9b7e7794f3bab5e024c28b73763b49a31`.
ABI 82 records 2 passed / 8 failed, SHA-256
`d8667b8e430005fe7de4ffe9296339dad7f69c5f302db315a2f7ccd5ce4de4de`;
independent oracle and final installed candidate pass 10/10, report SHA-256
values `cf8e21961e1ddda7a958a4e95d7b016bdc4cfa5f7ea5555df13442a674ea49c9`
and `94d93a7649c66c65076ea7f15d500cf9fefe9cb4aa1bbaa3cd13cd60e88449c3`.
The cumulative ledger is 819 passed, zero failed, seven skipped and 5,700 not
run, SHA-256
`43861212714734aa05343ef0ff48a87977136adbaecfe9d0c3c461bfb50af2f6`.

The complete 44-case Page-route file now records 23 passed / 21 failed / 0
errors, SHA-256
`f1d26be3d9e204963ec16219b0c4a21aa5e7ea2050fcc7155a8547e451ca7139`.
Installed units pass 172/172, report SHA-256
`5ce602f9b1a6127c3a3f2bad3f88234c4d11f4eebaa457530d3fae6f56ba8c2b`;
the all-feature host passes 1,007 with 26 explicit live ignores and the addon
passes 2/2. Strict Clippy, formatting, Ruff, Pyright, both source-pin checks,
the 108-package dependency proof, normalized wheel reproducibility and full
6,526/6,526 collection pass. Collection SHA-256 is
`ebbf1cd86f71077380330a6bb5510df1fac9f0b111caeea76400e8dd27bcc6a7`.
Repaired wheel SHA-256 values are
`65a3a83043e0451138a76fd0ff425e6c5f42a8696a2d45e10fdf9676e555eb71`
and `428a666c90e5102411cf20be196140f1aee6569475f3df7451ebaed45b15cdad`;
their only normalized differences are allowed CycloneDX serial/timestamp
fields.

NODE-331 keeps ABI 82 and runs the full unchanged async Page-route Chromium
file in 44 isolated subprocesses. Its preserved candidate baseline is 18
passed, 25 failed and one error, SHA-256
`1f40d74f878e0729ddbb7acf7dbd0a3f98c11cabc1530d78860a91ef983a50f4`.
The 18 green rows are fixed by `page-route-existing-coverage-manifest.json`,
SHA-256
`da5aacd59a19ee1852ffe7d7c0918a32cd8115895ce770ee409523297f8cdece`.
Installed candidate and independent oracle pass that selection 18/18; report
SHA-256 values are
`262b71d1c0a5b814bd0e4a8560d84077d19a60e29cad6ebb00f8783c06981102`
and `bf78f4ccd0177fada706551b22190f7c3af3bdd6db00a62a2bde20121908070e`.
The cumulative ledger is 809 passed, zero failed, seven skipped and 5,710 not
run, SHA-256
`0408aef3e6fa958f60ce74998e4326e40aef2bc906851efa35e9b656e9b5940c`.
The other 26 rows remain explicit candidate discrepancies; none is skipped or
reclassified.

NODE-330 keeps Python ABI 82 and promotes six more unchanged official
unroute-all rows on the final NODE-329 wheel. The default-removal manifest
passes candidate/oracle async 2/2 plus sync 2/2; the in-flight wait manifest
passes candidate/oracle 2/2 and proves that removal does not continue the
request before its active handler fulfills it. The two manifest SHA-256 values
are `f8b3136e07261dcad3ea631ace025ba68309c8db5c6e1f1b3ddc37be73a58c00`
and `329eee6cd757a0fa7c8f03f0b61343c7df627bb9717a9b4f0bf143eb47816998`.

Candidate async-default, sync-default and wait report SHA-256 values are
`fc09e9ec6ac97192b57d196370a7d36538ee3b2a9b8eb95b769b3b8dac0f3784`,
`aa3a0118bfd83ae64365e373ffca2296b6a89aeae34b208d777f1e813dc8d195`
and `6ae928c2191dd83930721db5ae54b65e62901637d5e95d7b56067242d1274505`.
Oracle report SHA-256 values are
`fd9b71ac2c245daf83ada3c94fb05eb86ae18742e1a807990d219c95127f83e4`,
`1d48ce9de9358807754f32653326ccafabae9e32e553bdf39e64d745856118f3`
and `c1b746273ecf5984442759529e21a8b47c27b8a64f2532d0759f8dd9be9363d4`.
The cumulative ledger is 791 passed, zero failed, seven skipped and 5,728 not
run, SHA-256
`d7255f3d41c76a26528036c0eaee54b788a48b2323650b3700f2da7441f7cb05`.

NODE-329 advances Python to ABI 82 with async/sync Page and BrowserContext
`unroute_all(behavior="wait")`. The two unchanged official HAR-abort rows pass
final installed candidate/oracle 2/2; candidate repeat stress passes 20/20.
Installed units pass 171/171 and the all-feature host passes 1,004/26 ignored.
Strict Clippy, formatting, Ruff, Pyright, source validation, normalized wheel
comparison and the 108-package dependency proof pass; full collection remains
6,526/6,526.

Manifest, candidate, oracle, stress, installed-unit, collection and ledger
SHA-256 values are respectively
`9288dc65838089a70eae6b2cf258e6efaf82727286da0432d6f0125ac0cee429`,
`d00a82c9a9c449d4f74e322273aae6c6be4fd2cfe4283d764bd1a063385b96ed`,
`e336fb07d6f408e376fe1f83256c419f1d4137cae151d4238a938ca63651dbe1`,
`873eb71d2854082eb257add3d3ea9a40753c74bb8a16a66d199f2b53793b5b04`,
`47a06e7fcc8ddb619ff4c6780478cb46fa7f96ae795046322900f7a5ea44bb1c`,
`d019672bb39ca41a5c91f97150722632dc0edc1d4d57b090cca522fb915ea7b7`
and `b58f55b19a62286fdf0c1b06040c017c1d9b00666b3e139847eaf8cc42027b6f`.
The cumulative ledger is 785 passed, zero failed, seven skipped and 5,734 not
run. Repaired wheel SHA-256 values are
`7323c7265dd3c3ac5d230391c57e7547d95de459ded20bd8009f9a84157c91b8`
and `af51f4861ec3723518203610fa673b948ce38ebb52f1e62ef0772140c58d1c9e`;
their identical native extension is
`d3175b42fe1ebf395c0c07e9285a99d104d17fee19b77162cc980e9c26df7465`.

NODE-328 advances Python to ABI 81 with async/sync
`BrowserContext.route()` composed with Context HAR URL filtering. The four
unchanged official rows pass final installed candidate/oracle 4/4; candidate
stress passes async 20/20 plus sync 20/20. Installed units pass 169/169 and the
all-feature host passes 1,003/26 ignored. Strict Clippy, formatting, source
validation, normalized wheel comparison and the 108-package dependency proof
pass; full collection remains 6,526/6,526.

Manifest, candidate, oracle, async-stress, sync-stress, installed-unit,
collection and ledger SHA-256 values are respectively
`18679b37e9b90eb925e3732f129d06eed488211246b912eb2770a803ed1622cd`,
`9d47307120b6db7534e0d472e5e9c7fd7d460daa993dceb091e0cc0bf7639648`,
`a122f8861f9e1ec313e426e40d876bab7581c4272d1274a9b028f590c1507af3`,
`58b080019c7abdfaedc5180ad2b7f4c32f86c9a4748d4692dde45e7146ae16d9`,
`326124751fe9e276c07af296f25f6dd68f384ace35f6f295fcfd08b998d61502`,
`81d61edc55aa158cf023229182cf6aa8497ee12ffd5dfb65426f2931aa227158`,
`c745b3f8bd9deebc6deb2e36a4567137b54e9589f5a28910bb822f6df8d086d0`
and `c0c16280690a496ffdc374fa77ad1792aedd6670a9c61026d70fc042bce77b17`.
The cumulative ledger is 783 passed, zero failed, seven skipped and 5,736 not
run. Repaired wheel SHA-256 values are
`8e094a57d7b1e407fbad9a4c8adb3fe119f59f856e53d2cca905db20cad7108f`
and `f446b28128ff4d9dead74b4abc2f3d3613459e9fa5e90a0468f2df8dcab2fc56`;
their identical native extension is
`0a3f95aaddc7d406562603d533f8933c7603ac566c0c5e4b58b9f0d7770a2155`.

NODE-327 keeps Python ABI 80 and treats missing HAR `log.entries` as a bounded
empty archive while preserving every other parse and resource error. The two
unchanged async/sync fallback rows pass final installed candidate/oracle 2/2;
candidate stress passes async 10/10 plus sync 10/10. HAR policy tests pass 5/5,
installed units 168/168 and host 1,003/26 ignored. Strict static gates, source
validation, normalized wheel comparison and the 108-package dependency proof
pass; full collection remains 6,526/6,526.

Manifest, candidate, oracle, async-stress, sync-stress, installed-unit,
collection and ledger SHA-256 values are respectively
`6d26bdd0caa16f37de43ec9c45ddc6d3534260f12d54dea5cf2e39349ceccc6a`,
`407c2a8b3c60d4044c56cd3cab6c3ecc2bb2185980a7f24ab324d0359313cc3b`,
`09b8ac3378150efea8cfe01f0d09cc81169eb9ba520e1d78a7a966c69297f500`,
`db2cfd2bb97faa0327784954bd913377ceb5d86b2a7f2f8d229750193cbc0d5b`,
`e7df792c9dabb9a14aef8bb5eeb71e825c458b164a13ee412840d2a595303a0f`,
`d17371760e41463a92414619034300932367f6beef78373b37d2648c2b346589`,
`eaa5f82eb7652df1d0c02f97a182dd27eb988a676225b3403787ab7de8b3cb04`
and `a545a96742ef986a132be551627450e08c8d1fd0ecbfca8a1fe8e5ab142328c8`.
The cumulative ledger is 779 passed, zero failed, seven skipped and 5,740 not
run. Repaired wheel SHA-256 values are
`c0c288d765c1afa1099093bd63743b5b5be72b26c1296561d374c60140733d74`
and `c15e0eb94e6d7ed6701e0dcc7948119c996cee4b9f0cfa9409d93754f51d2fe2`;
their identical native extension is
`d5c19d2c34536a8dd6a0c6a8ccc72b4d168fe8dd0b0124847a6fa7eef473b7be`.

NODE-326 advances Python to ABI 80 with async/sync
`BrowserContext.route_from_har()`. Rust now owns explicit Page/Context route
scope, Page-over-Context precedence, future-Page/OOPIF initialization and a
bounded 64-session bridge for Fetch pauses received before topology
publication. The six unchanged method/redirect, fallback-continue and default-
abort rows pass final installed candidate/oracle 6/6; candidate stress passes
async 30/30 plus sync 30/30. Installed units pass 168/168, the all-feature host
passes 1,003 tests with 26 explicit live ignores, strict static gates and the
108-package dependency proof pass, and fresh collection remains 6,526/6,526.

The fixed manifest, candidate, oracle, async-stress, sync-stress, installed-
unit, collection and cumulative-ledger SHA-256 values are respectively
`7ed9f484469317a0f4e979d9314ad7b4d203a30e056ea9f2d4e4a24b5219c4ea`,
`c1e79d7c9ad98826a6d39460e70161390de47d45b64dcd0dbc5acd6aacf8608d`,
`2d52861f82472f55e8d56f0ae286a322564921f0f2053ed6e75683402c058914`,
`b49a090ca4da67775d9d1ebbd2ea028ba43039841a1fbe8c1a921c3d03e0566a`,
`3acb6037da23c23c6b1505e8e4b7b3091cb9ea8f8f9e563b39f30ad80ffb8a92`,
`e3ca609d17b6c108dc059d20bf2e656f3e70620d3c15d7fe19afa4109e3fa034`,
`c77f442acb9602f67b5f036a9905b92808d88cdb5f9f599e68718da3fba6abba`
and `c52d8ae2a549fc2cfa75e745969863363c8e10705daea947e46a9446d58b89a7`.
The ledger is 777 passed, zero failed, seven skipped and 5,742 not run. The two
normalized-payload-reproducible repaired wheel SHA-256 values are
`615eaa8e7106b726d966ab9b2340cdcffb16e9a7bfb8ca381750377fa9203ca7`
and `e964e43fba00fead4e473fc7bbf4aa474adc9da808a4cdf2375af6c23e93a745`;
their identical native extension is
`381914acb56091eafcc140cb85644900e1d1897b60954e517fd3008fb2d52e02`.
External/ZIP/update HAR, regex/predicate filters, ordinary Context routes/URL
filters, `unroute_all`, remaining Python and complete Core JS/TS coverage stay
open.

NODE-325 keeps Python ABI 79 and promotes the complete unchanged async/sync
Page HAR URL-filter/no-fallback pair on the NODE-323 installed wheel. Each row
also composes the internal HAR route with an ordinary `page.route()`, proving
that nonmatching traffic reaches the next route. The fixed manifest SHA-256 is
`ad9293c598db398137c56ccb9405311db2ef4fff9bfe13cecd61a260f58067fe`.
Final installed candidate and independent oracle pass 2/2. API-separated
candidate stress passes async 10/10 plus sync 10/10.

Candidate, oracle, async-stress and sync-stress report SHA-256 values are
respectively
`eca2cd0c052686b795e94d5dbdd59a17468e0b952a33dd073a39745ffeba1e94`,
`807bb7adf649c77448c0963b6f4542c22a674371b341160beffb68a1a0dc23f0`,
`f460365ce417c1e205cdb6435708fee906c4d2fb9f2afcbb6808d92d64840474`
and `7ee65e04701576ef9b4d90058bcf8fadb364ec4f46a849e75b06684963b43c19`.
Fresh collection remains 6,526/6,526 and the cumulative ledger advances to 771
passed, zero failed, seven skipped and 5,748 not run, SHA-256
`fb11fa9a34ed364ec06254672ac32ef54a7a790362eeaa4f6a562887c2d1f03e`.
This checkpoint changes only fixed conformance evidence; code, ABI, dependency
closure and the reproducible NODE-323 wheels are unchanged.

NODE-324 keeps Python ABI 79 and promotes the complete unchanged async/sync
Page HAR method/redirect pair on the NODE-323 installed wheel. The fixed
manifest SHA-256 is
`51e666d86132e4d88790025a98dff4ddb114eec22eb3df71e51bf2a23afd7f33`.
Final installed candidate and independent oracle pass 2/2. Candidate stress is
split by public API and passes async 10/10 plus sync 10/10. The mixed-API repeat
report is diagnostic only: after the first pair pytest-asyncio calls
`Runner.run()` from inside the sync bridge's running event loop; it is excluded
from the ledger.

Candidate, oracle, async-stress and sync-stress report SHA-256 values are
respectively
`2340bfc952cf7c2ec843e2e6166372a32d531e46587f8c080b04aa1235ec9958`,
`950219a444ee63edccce9d63cc8fade9323dce703b9c05bf97b43c6162437654`,
`17bea22d89ca4dc5cb862ba3820b8ad39dd42aa5bcb43684d6ebcc4b6f86f964`
and `ad8c1a66288ec6d2cde1dcdc7ba7e42cac4d5f5c9983ee48d895fa90405004ec`.
Fresh collection remains 6,526/6,526 and the cumulative ledger advances to 769
passed, zero failed, seven skipped and 5,750 not run, SHA-256
`dbc28e3a20089d56d2b937d7175f19e4562db76da59477b789e8c14fc375cefd`.
This checkpoint changes only fixed conformance evidence; code, ABI, dependency
closure and the reproducible NODE-323 wheels are unchanged.

NODE-323 advances Python to ABI 79 with independently selectable plain-HAR
replay and the exact computed-style assertion needed by the unchanged upstream
row. `pwright-har-replay` is backend-neutral and bounded; the host composes its
internal decisions with the existing Page-route Fetch owner, so Python retains
no archive, matcher, response body, pending route or CDP session.

The exact upstream selection is recorded in
`packages/pwright-python/tests/official/persistent-context-har-manifest.json`,
SHA-256
`a3e57ce38826a608a730a6e382c60e65967cc4b81891aa6cf5118cc0790adf08`.
Final installed candidate and independent Playwright 1.58.0 oracle pass 1/1;
candidate stress passes 10/10. Installed-wheel units pass 167/167, the
all-feature host library passes 1,001 tests with 26 explicit live ignores, the
HAR policy crate passes 4/4 and the addon passes 2/2. Strict Clippy, Ruff,
Pyright and the 108-package dependency proof pass. Fresh full collection is
6,526/6,526 and the cumulative ledger is 767 passed, zero failed, seven
skipped and 5,752 not run, SHA-256
`e390e350e8995a48569133ed5be757cd9040a6d1592ab1b86f0ad5ca046476e2`.

Candidate, oracle, stress, installed-unit and collection report SHA-256 values
are respectively
`74bc73ec033aa2bdd914f94d752846615ea2a4b0801f8e8c9e3ef390fcdafc31`,
`b48a9c6aa3325a2663fe987ae118058586b0f917c27babf9ba0cc2609893cc02`,
`300abda8e41433e285970a456c32acd412688f194af3d57e8e77abda36861f5d`,
`3c33650324d96ed74eaf5562bf795c15fbc5c6bad9e3117d817b43b9806277df`
and `a3619b07314bb55898cbd6110bb25eb4af3b746c95216f36fe1371dfc9059799`.

Two normalized-payload-reproducible wheel archives have SHA-256 values
`c1954548f523af59abc861656595738e8b6ae82c8b02b8f2deaa4e2de6616473`
and `d0bcdc98b69ddaf9fdcf2d5df482e32fcd2801b9ec43b28f8aa314dae72ebde8`;
their byte-identical native extension SHA-256 is
`243f6a6771e86e4d8bce4b7bd96293ea37c558fd2ef79344dfd76b683b3cd2c3`.
Maturin still identifies Homebrew OpenSSL dylibs as external dependencies, so
these remain verified Mac-local artifacts. External `_file` content, ZIP HAR,
update recording, Context-level routing and the broader HAR corpus remain
open. Two original default-Context cookie rows remain explicit MagicDNS versus
hardcoded-`localhost` applicability gaps; the remaining Python and complete
Core JS/TS corpora remain open.

NODE-322 advances Python to ABI 78 with independently selectable ordinary and
persistent-Context HTTP credentials plus navigation response-body retrieval.
Rust owns the fixed 16-Context credential registry, four fixed 64-entry
authentication collections, selected-session Fetch composition and an
eight-operation/4 MiB response-body leaf. Page routes and authentication share
one reclaiming event cursor; Python retains no credentials, challenge history,
body cache or CDP session.

The exact upstream selection is recorded in
`packages/pwright-python/tests/official/context-http-credentials-manifest.json`,
SHA-256
`154cd2e468af69fd4a8365e46a8959e3fb84f933626dff388c4e64970a241150`.
Final installed candidate and independent Playwright 1.58.0 oracle pass 11/11;
candidate stress passes 110/110. Installed-wheel units pass 165/165, the
all-feature host library passes 999 tests with 26 explicit ignores, 22
non-live integration tests and 2 addon tests pass, and strict Clippy, Ruff,
Pyright plus the 108-package dependency proof pass. Fresh full collection is
6,526/6,526 and the cumulative ledger is 766 passed, zero failed, seven
skipped and 5,753 not run, SHA-256
`294ab4a8fee1ea783d5f5b8043a54c52648fe5e9539fdecbd7c42c6000b9c2fa`.

Candidate, oracle, stress, installed-unit and collection report SHA-256 values
are respectively
`cb7809748be7cc915f6125e973c1149b252b11fcf255c7b528489d7ba93a2de2`,
`7652ef246c7f7007159d663a640e1fa9cc14591465de8a34f2961a035d6b3cc0`,
`ffe3600bd95491851f7a623388780fb744a51c5fdae49322ab43f44040cf1004`,
`023b372456d7759abf411a19d06a0352c6a961ef8a313aacde3da503e6e4cf8b`
and `ec96b3f4af79891a2b2049537ac2650702e83d41ee7ca5539bbb06ce4037ccdb`.

Two normalized-payload-reproducible wheel archives have SHA-256 values
`473db3f3170c2124418f5bbd17dae19d8c5cc4f933938f1887233ce862c968b4`
and `68ee94ea983be2ed9794b21fbba030299dcc8a3e00c4eac320099f23d1dc98aa`;
their byte-identical native extension SHA-256 is
`f5b18b328da7c6d6a21e2335151da6289eec167fd9a0652a7818579a53662eca`.
Maturin still identifies Homebrew OpenSSL dylibs as external dependencies, so
these remain verified Mac-local artifacts. Three original default-Context rows
remain: two MagicDNS/`localhost` cookie applicability gaps plus HAR. The
remaining Python and complete Core JS/TS corpora remain open.

NODE-320 advances Python to ABI 75 with independently selectable
`context-offline` and `persistent-context-offline`. The new
`crates/pwright-node-host/src/context_offline.rs` owns one fixed 16-Context
registry, exact owned/default/external scoping, pre-publication
`Network.emulateNetworkConditions` initialization and idempotent release.
Ordinary and persistent Context creation compose offline with locale, timezone,
HTTPS, CSP and JavaScript policies through one policy value. The addon and thin
async/sync Python facades expose only the public `offline` option; ABI features
and resource limits remain queryable.

The exact upstream selection is recorded in
`packages/pwright-python/tests/official/context-offline-manifest.json`, SHA-256
`00268111cd781614c53806e3f90903151be78e1db662680f547189c6996b065f`.
Final installed candidate and independent Playwright 1.58.0 oracle pass 1/1;
candidate stress passes 10/10. Installed-wheel units pass 159/159, all-feature
host tests pass 983 with 26 explicit ignores, addon tests pass 2/2, and strict
Clippy, Ruff, Pyright plus the 108-package dependency proof pass. Fresh full
collection is 6,526/6,526 and the cumulative ledger is 754 passed, zero failed,
seven skipped and 5,765 not run, SHA-256
`c38eea63e7096c771a8faa0912c0b1499158c6a11489dfb4848ac75cc4079592`.

Candidate, oracle, stress, installed-unit and collection report SHA-256 values
are respectively
`27570a9e7750228f4b60c287573b38d015e0daa944ba6bb1624b7bb0de08cf99`,
`f8e99a025c088eba2042729572fb97ae047be0c6862e528cfdf4aa4b115c7527`,
`61bce407c655347e3308e6c5384a8f4c2edb48248562181a9a26c2daced268de`,
`df4b303b88ec2458834baf4c54489c4773da2e871db87ac7e30a60a41d522bad`
and `d925042c8bcdb55a7592543f2bd0f7814c592902eafa634e2d719f1bb9869b1c`.

Two normalized-payload-reproducible wheel archives have SHA-256 values
`cdec69d543da109a3071b0b0b52d3e5c35d8344ddcbbdcee2a5931fde18538f9`
and `2be2930d675c736375cebb3506228792c5eac0065c10bde7f5956311e8ad32a4`;
their byte-identical native extension SHA-256 is
`e73bb6233334972f3e58d38370672af658a0c791517862ecb3e50e7c87a123f4`.
Maturin still identifies Homebrew OpenSSL dylibs as external dependencies, so
these remain verified Mac-local artifacts. Five original default-Context rows
remain: two MagicDNS/`localhost` cookie applicability gaps plus geolocation and
permission, HAR and HTTP-credentials product capabilities. The remaining
Python and complete Core JS/TS corpora remain open.

NODE-319 advances Python to ABI 74 with independently selectable
`context-timezone` and `persistent-context-timezone`. The new
`crates/pwright-node-host/src/context_timezone.rs` owns one fixed 16-Context
registry, 256-byte identifier bounds, exact owned/default/external scoping,
pre-publication `Emulation.setTimezoneOverride` initialization and idempotent
release. Ordinary Context creation composes locale, timezone, HTTPS, CSP and
JavaScript policies through `ContextPolicyOptions` instead of extending a
positional host call. The addon and thin async/sync Python facades expose only
the public `timezone_id` option; ABI features and resource limits remain
queryable.

The exact upstream selection is recorded in
`packages/pwright-python/tests/official/context-timezone-manifest.json`,
SHA-256
`68c4d90a61e471f7d904c27b1831bbc65280f80fd91c645c6172fd9c00fa25dc`.
Final installed candidate and independent Playwright 1.58.0 oracle pass 1/1;
candidate stress passes 10/10. Installed-wheel units pass 157/157, all-feature
host tests pass 978 with 26 explicit ignores, addon tests pass 2/2, and strict
Clippy, Ruff, Pyright plus the 108-package dependency proof pass. Fresh full
collection is 6,526/6,526 and the cumulative ledger is 753 passed, zero failed,
seven skipped and 5,766 not run, SHA-256
`a48b4d1c0e8bdce904b95194393ee47709c01313c121843e81e1410f524b27fc`.

Two normalized-payload-reproducible wheel archives have SHA-256 values
`a8b4ae9a06dde9cf9d7f766cb7d326ca3b0e69d773c23b9fdbb14d8c19519632`
and `377d5981d1d76bf121bf0db45222a0b82e9610bb9f6e9b638f075a67f452f49f`;
their byte-identical native extension SHA-256 is
`8b1dd78a036c331ae027e2c1cbb49caa51008ead5b9d4e022c729607f8ba6d02`.
Maturin still identifies Homebrew OpenSSL dylibs as external dependencies, so
these remain verified Mac-local artifacts. Six original default-Context rows
remain: two MagicDNS/`localhost` cookie applicability gaps plus geolocation and
permission, HAR, HTTP credentials and offline product capabilities. The
remaining Python and complete Core JS/TS corpora remain open.

NODE-318 advances Python to ABI 73 with independently selectable
`javascript-disabled` and `persistent-context-javascript-disabled`. Rust owns
the fixed 16-Context policy registry and exact
`Emulation.setScriptExecutionDisabled {"value": true}` initialization. The
same checkpoint adds thin async/sync `Page.expect_navigation` over the existing
frame-navigation owner, first-visible-fragment Locator click targeting, and
nonfatal main-world console replay while paired-world lifetime proof is not yet
available.

The fixed manifest SHA-256 is
`7f2e326e8c7d5ac9623279fac00277468ee9ae25394f11a10b6ac7d8d90641c4`.
Final installed candidate and independent Playwright 1.58.0 oracle pass 4/4;
candidate stress passes 40/40. Installed-wheel units pass 157/157, all-feature
host tests pass 974 with 26 explicit ignores, addon tests pass 2/2, and strict
Clippy, Ruff, Pyright plus the 108-package dependency proof pass. Fresh full
collection is 6,526/6,526 and the cumulative ledger is 752 passed, zero failed,
seven skipped and 5,767 not run, SHA-256
`93f22527ff4e8f89d95a525a17393d3d5d3463432e6e9e75f7135adcea7a81a3`.

Two normalized-payload-reproducible wheel archives have SHA-256 values
`7c2e48a4705d597aba896763e50b67ddfd6d8d03c562312c5dcd3f24ba687859`
and `89b60d08b5e5691aeb4de049a2d10a74f0a42947f42fdf57c3693d0eabd1ab9a`;
their byte-identical native extension SHA-256 is
`26e341524d4f6bf5703089a1db57343cc885e8c243ed2bfc6448b72dfd9a41a0`.
Maturin still identifies Homebrew OpenSSL dylibs as external dependencies, so
these remain verified Mac-local artifacts. Seven original default-Context
diagnostic rows remain, including two MagicDNS/`localhost` applicability gaps;
the remaining Python and complete Core JS/TS corpora remain open.

NODE-317 advances Python to ABI 72 with independently selectable `bypass-csp`
and `persistent-context-bypass-csp`. Rust owns one fixed 16-Context policy
registry and applies exact `Page.setBypassCSP {"enabled": true}` initialization
to every Page/OOPIF session selected by an owned Context or the persistent
default Context before publication. Attached external Contexts inherit no
policy. Close and partial-creation compensation release the same owner
idempotently; Python validates and forwards only public booleans.

Release validation passes 970 all-feature host tests with 26 explicit ignores,
2 addon tests, 152 installed-wheel units, strict Clippy, Ruff, Pyright and the
108-package dependency proof. The fixed manifest SHA-256 is
`6748a04a6e0e1f0340d7e7c6e14ef6ba5ddd4e0e91361dd3fe2feb9f57fc4c64`.
Final candidate and independent Playwright 1.58.0 oracle pass 4/4; candidate
stress passes 40/40. Their report SHA-256 values are
`3760b066bee25591941fc8216c93c3d245a494a08ce8bd9de5ec3a096a241e36`,
`a0776de441de54867fcea680f928e37203ba97779bde912b178cb3c670c83459`
and `7aec8021bd753d172a696cb717cdb6dd1e0b55769cc0fce29b9d0eebdb6f5f7e`.
The installed-unit report SHA-256 is
`865dcf1f721c0758472d18a07d64283ae31c84ae5269893fb18782ba3fbc846b`.
Fresh full collection remains 6,526/6,526, report SHA-256
`de5a014d6818e1f613f08a74ddac3d70a23fc1f720b17d273f626ac0290d32c1`.
The cumulative ledger is 748 passed, zero failed, seven skipped and 5,771 not
run, SHA-256
`713d4047df50050430e4bfec6c843b171806653db19e8c0df6da6e66cd942508`.

Two final wheel archives have SHA-256 values
`44a9ec5ee2be17cab8036084dfa46c63fc717904692175e6fa66bafd8ef21f70`
and `319a9d9c11cf43d55a1b34eb498b9e24dcc6048a5e4ca43931a880a76c5ca7eb`.
After normalizing only CycloneDX's build UUID/timestamp and matching RECORD
hash, every payload entry is identical; both native extensions have SHA-256
`0a2ce2ad114c52ff3cb51328557386db291e922f6684ffa8b2d50133b8fe721d`.
Maturin still reports Homebrew OpenSSL dylibs, so these remain verified
Mac-local artifacts. Eight rows remain from the original default-
BrowserContext diagnostic, including two recorded MagicDNS/`localhost`
applicability gaps. The separate iframe CSP row remains unclaimed because its
baseline first requires `ElementHandle.content_frame`. Full Python and Core
JS/TS execution remains open.

NODE-316 advances Python to ABI 71 with independently selectable
`persistent-context-extra-headers` over persistent launch and the existing
`extra-headers` owner. Rust admits at most 256 launch headers / 64 KiB, moves
them into the existing `HeaderPool`, and atomically registers only the first
persistent default Context before its Page/OOPIF session receives
`Network.setExtraHTTPHeaders`. Owned and attached external Contexts do not
inherit the default. Explicit later Context/Page updates, compensation,
retirement and clear semantics continue through the same `HeaderSettings`
owner. The default consumes one of the existing 16 Context header slots and
the FFI call retains a fixed 512 KiB allowance.

Release validation passes 966 all-feature host tests with 26 explicit ignores,
2 addon tests, 150 installed-wheel units, strict Clippy, Ruff, Pyright and the
108-package dependency proof. The fixed manifest SHA-256 is
`5345d32db44ce2f21a3c8ebc6fec811a498c695f68972b66564a39677d2d1041`.
Final candidate and independent Playwright 1.58.0 oracle pass 1/1; candidate
stress passes 10/10. Their report SHA-256 values are
`f7fe00394cd78f6745be76c28400a87eeb3e55c071a22aa04a151592f708fab0`,
`b0249488f1be47e6685e739ead9c63b958e8544bae3a67708211aafd15b183f2`
and `14302892b311ab2e884592c1364f90e0c34e939719b65ebc860f3a9e22b0ff7a`.
The installed-unit report SHA-256 is
`30a6769c34931b6202d01b8ab39031cb54eef3b5a4fee1127f14ba18e9f4262c`.
Fresh full collection remains 6,526/6,526, report SHA-256
`5a702bfa1b9a911fe704d98b4fcfe0ebbc4741fe428edc3f085afbf10e0d58fc`.
The cumulative ledger is 744 passed, zero failed, seven skipped and 5,775 not
run, SHA-256
`19d1e6e8365cf3c56b86e96edc76d10a6a619b797f967caedb477cacfbd69411`.

Two final wheel archives have SHA-256 values
`f954e83ec2343be3b52fe4ad0f8a3393c343211f4779bea8f6a3d5a7857ec7e2`
and `1f0a64b3d822ca9b018b7ba64c94d8852f53aaf0de6069f33ab4ef1bb2960ebc`.
After normalizing only CycloneDX's build UUID/timestamp and matching RECORD
hash, every payload entry is identical; both native extensions have SHA-256
`a5b9fcb1d52d72986a017de082b0ee342fa57b5e64b413e52827c6d0c22392d2`.
Maturin still reports external Homebrew OpenSSL dylibs, so these are verified
Mac-local artifacts rather than self-contained distributable wheels. Nine rows
remain from the original default-BrowserContext diagnostic, including two
recorded MagicDNS/`localhost` applicability gaps. Full Python and Core JS/TS
execution remains open.

NODE-315 advances Python to ABI 70 with independently selectable
`persistent-context-emulate-media` over `persistent-context` and the existing
`emulate-media` leaf. Rust admits one immutable optional default before initial
Page topology and applies it only to the persistent default Context. Every new
Page/OOPIF session reuses the existing media initializer and capacity-one gate;
later owned Contexts and attached external Contexts do not inherit the policy.
Python only validates and forwards the public `color_scheme`,
`reduced_motion` and `contrast` enums.

Release validation passes 964 all-feature host tests with 26 explicit ignores,
2 addon tests, 149 installed-wheel units, strict Clippy, Ruff, Pyright and the
108-package dependency proof. The fixed manifest SHA-256 is
`a2da1fb06d4bd6039f7255d1f95e92f853ef5260b491f11fa1236a19f2c956f9`.
Candidate and independent Playwright 1.58.0 oracle pass 3/3; candidate stress
passes 30/30. Their report SHA-256 values are
`166ede1c3ec3e6aff90032bfa6b763ca7fbc6cc3f3798be5d8901af4ac7eea30`,
`341fa62124ef00ab42f2ff1094f6f5a1f4e38e32efafa6ee7846096a3ffe95df`
and `00c6284f51bd7267efd0084f9622cfd56485f135c5a90cc1786db4bc8a698f5e`.
Fresh full collection is 6,526/6,526. The cumulative ledger is 743 passed,
zero failed, seven skipped and 5,776 not run, SHA-256
`8a7fe5975b0bbbf792cc8c13a0b8e81a11a70bb8f17dcb0e8f439abe30e67efb`.

Two final wheel archives have SHA-256 values
`3b7ddb9ea476955dcf442a357846a9856ef1e625f304c435ac970685f1a737a8`
and `e99fc988705d6daafce55e60aa930b49f7964aa3902fb200307b3111cd05f353`.
After normalizing only CycloneDX's build UUID/timestamp and the corresponding
RECORD hash, both payload manifests are identical at
`4a1e6bb998fe7ef421439bd1e7dd398c484713ae82c331e5f6d4c10a1d03177a`;
their native extension SHA-256 is
`6a44342c2790626304fa7483f5537d2f611223a22068bbb667149c48c5654c53`.
Maturin still reports external Homebrew OpenSSL dylibs, so these are verified
Mac-local artifacts rather than self-contained distributable wheels. Ten rows
remain from the original default-BrowserContext diagnostic, including the two
recorded MagicDNS/`localhost` applicability gaps. Full Python and Core JS/TS
execution remains open.

NODE-314 keeps Python ABI 69 and exposes the existing public `args` option only
far enough to reproduce Chromium Playwright's pre-launch ownership errors.
The binding bounds and validates the sequence before native work, rejects
user-data-dir and remote-debugging-pipe ownership violations, and rejects a
bare argument that would open a page. An empty sequence is a no-op. Non-empty
switch-only arguments remain explicitly unsupported until a Rust launch
provider owns them; this checkpoint does not claim general launch-argument
support.

The pinned official row passes candidate/oracle 1/1 and candidate stress
10/10. All 148 installed-wheel units, Ruff and strict Pyright pass. Two wheel
archives are normalized-payload reproducible at SHA-256
`d17207c54c374c072df4d752df38959ecd376c5918dcc215fe3b46a22cb93b07`
and `76bfdf8d21cb28d268139106d225cf1d989ae53e629eb0805bfd9278efc0b052`;
their unchanged native extension SHA-256 is
`d2332cab6551ab9c189acd77678bf7ba43d20c167e5bc4d9c27cebcdd71b430f`.
Fresh collection remains 6,526/6,526 and the ledger is 740 passed, zero failed,
seven skipped and 5,779 not run. The existing unvendored-OpenSSL packaging
warning remains open. The cumulative summarizer also replaces a redundant
Python-3.10-only strict zip with an already length-proven Python 3.9 loop; the
system Python 3.9.6 run reproduces the exact ledger hash.

NODE-313 advances Python to ABI 69 with independently selectable
`context-cookies`. Exact Context ownership is resolved in Rust and all cookie
commands use Chromium's browser-root `Storage.getCookies`,
`Storage.setCookies` and `Storage.clearCookies`; there is no Python or Rust
cookie mirror. Rust owns URL rewriting, URL/domain/path validation, filtering,
filtered expiry and the full partition key, including
`hasCrossSiteAncestor` during filtered clear. Python supplies only bounded
Playwright-shaped async/sync facades and pre-bounds input before FFI copying.

The release gates pass 130 CDP tests, 962 all-feature host tests with 26
explicit ignores, 2 addon tests, 146 installed-wheel units, strict Clippy,
Ruff, Pyright and the unchanged 108-package dependency proof. Two wheel builds
are normalized-payload reproducible; archive SHA-256 values are
`174a30499a3c21c746d4ed5ffb5461247afe012bc9207c32834c68c0801382d4`
and `faa8f553d1643ed4594997acf1fd06dd75f730c3981cbf99fe12d3dda0d69c58`,
and their native extension is byte-identical at
`d2332cab6551ab9c189acd77678bf7ba43d20c167e5bc4d9c27cebcdd71b430f`.
Maturin still reports unvendored Homebrew OpenSSL dylibs, so these are verified
Mac artifacts rather than a cross-machine self-contained release.

The fixed four-row manifest has exact candidate/oracle parity. Clear-cookies
and third-party-cookie rows pass 2/2. Read/add rows differ only because the
required Tailscale fixture yields its MagicDNS cookie domain while unchanged
upstream assertions hardcode `localhost`; both remain not-run applicability
gaps. Ten repeats complete 40 launches with 20 passes and the same 20 fixture
outcomes. Fresh collection is 6,526/6,526 and the ledger is 739 passed, zero
failed, seven skipped and 5,780 not run.

NODE-312 keeps Python ABI 68 and adds async/sync `BrowserContext.on`, `once`
and `remove_listener` over the existing Rust-owned lifecycle publication. The
facades expose only installed `page`, `close` and optional `console` events;
they create no second watcher, polling loop, native export or lifecycle state.
Sync wraps the exact existing facade identities and routes callback failures
through the existing bridge error owner. Context-console listeners hold one
existing bounded native subscription from first listener through last removal.

All 141 installed-wheel units, Ruff, strict Pyright, formatting and the
108-package dependency proof pass. Two normalized-reproducible wheel SHA-256
values are
`2487ed3dbbac1e4fa7047e2cfbf7b3c6e5f0978b94eb972db27b9f6f60a4acf3`
and `21d9631c7e361584cad656a7eb5332d9332cdd86db7fb1314c08fae8f26e778b`;
their byte-identical native extension SHA-256 is
`890e6b4d7023d0ee13ff1a4bfa1b823416297a5ad273dbc5c1d84c2ceabc3f17`.
The unchanged official persistent-close row passes candidate/oracle 1/1 and
candidate stress 10/10. Fresh collection remains 6,526; the ledger is 737
passed, zero failed, seven skipped and 5,782 not run, with exactly that row
changing and no losses.

NODE-311 advances Python to ABI 68 with independently selectable
`persistent-context-device-emulation` over `persistent-context` plus
`device-emulation`. The default policy is admitted before initial Page
topology, selected only for the persistent default Context and counted against
the existing 16-Context device registry and byte budget. Later owned Contexts
remain explicitly registered and attached external Contexts remain unaffected.

The ABI tuple is now `(base_url, locale, ignore_https_errors, device,
executable_path, channel, timeout_ms)`. Rust remains authoritative for
viewport, screen, scale, mobile, touch and user-agent emulation; the Python
facades add only validation, forwarding and Playwright-compatible
`Page.viewport_size` snapshots. The existing device initializer, Page/OOPIF
replay, user-agent composition and resource bounds are reused.

954 all-feature host tests with 26 explicit ignores, 2/2 addon tests, 137
installed-wheel units, Clippy, formatting, Ruff, strict Pyright and the
108-package dependency proof pass. Two normalized-reproducible wheel SHA-256
values are
`a1ca0c2bb6d80a0e9ac6fadfa3e778077acc3f42cd6ef9d3e0a55101ba798059`
and `654386396048318f4fb8fdfbf6839da2b8c7ee69be19b74a921569370a30987f`;
their byte-identical native extension SHA-256 is
`c4d78c799f17dc1955514593fe8132173bf48c273d14a1a2ac4a74cd36703fe0`.
The five unchanged official rows pass candidate/oracle 5/5 and candidate
stress 50/50. Fresh collection remains 6,526; the ledger is 736 passed, zero
failed, seven skipped and 5,783 not run, with exactly those five rows changing
and no losses.

NODE-310 advances Python to ABI 67 with independently selectable
`persistent-context-ignore-https-errors` over `persistent-context` plus
`ignore-https-errors`. The default policy is admitted before initial Page
topology, selected only for the persistent default Context and counted against
the existing fixed 16-Context HTTPS-policy registry. Attached external and
unselected later owned Contexts remain unaffected.

The ABI tuple is now `(base_url, locale, ignore_https_errors, executable_path,
channel, timeout_ms)`. Four focused Rust regressions, 952 all-feature host
tests with 26 explicit ignores, 2/2 addon tests, 136 installed-wheel units,
Clippy, formatting, Ruff, strict Pyright and the 108-package dependency proof
pass. Two normalized-reproducible wheel SHA-256 values are
`c153464a16a2083899e44a3637d48d2cf512a9d669675feed27a244df2f65880`
and `f78552d51c1ecc9c2cac8a8b8193896663c71213d487a57903c8d45c184f1387`;
their byte-identical native extension SHA-256 is
`7f32e973622c1b7e1edbd1465637bebbc94af840972bf5457ee13df500c265eb`.
The unchanged official row passes candidate/oracle 1/1 and candidate stress
10/10. Fresh collection remains 6,526; the ledger is 731 passed, zero failed,
seven skipped and 5,788 not run, with exactly that row changing and no losses.

NODE-309 keeps Python ABI 66 and repairs persistent Browser ownership. A
Browser returned behind `launch_persistent_context` is now marked as the unique
owner of its default Context. Once `BrowserContext.close()` completes normal
Context retirement, it closes that Browser's existing native session; Rust
then removes the session from the fixed 16-slot Python environment. A
successfully closed Browser is also removed from `Playwright._browsers` so the
language object graph is not retained. Ordinary owned Context close continues
to leave a shared connected Browser open, and sync uses the same async bridge.

The first same-process ten-repeat ended with 10/30 passing because five
persistent launches per repeat exhausted the unreclaimed 16-session limit. The
final installed wheel passes 30/30 invocations and all 50 launches. Three new
AAA regressions and all 135 installed-wheel units pass, as do all-feature addon
release Clippy, formatting, Ruff, strict Pyright and the 108-package dependency
proof. Two final ABI3 wheels are normalized-reproducible; their SHA-256 values
are `1bdd9b424f21d52def2b82901df4b4069e8c605e946b4476aa8312f8519120ba`
and `58cc07b09356c442130238faf876336164964e9d6ff17429dcdf8024b66ae8d0`.
The native extension is unchanged, SHA-256
`45fc56e1138798ad239e06c066b16e6e6ed8c5eab2064e9a7a9e1e92cf975024`.

The three unchanged persistent-profile baseline rows pass candidate and oracle
3/3. Fresh collection is 6,526/6,526 and the cumulative ledger advances to 730
passed, zero failed, seven skipped and 5,789 not run, with exactly those three
rows changing and no losses. The other 23 Chromium rows from the 27-row
diagnostic remain explicit gaps.

NODE-308 keeps Python ABI 66 and repairs `wait_for_function` timeout ordering.
At the public deadline Rust cancels the polling owner, prevents another
predicate attempt and waits for only the current admitted browser command plus
its ordinary publication fence to acknowledge cancellation. A late Handle is
disposed before the public `Timeout` returns; stop/disconnect remains
authoritative `TargetClosed`. This adds no export, feature, task, queue,
transport, registry, retained session or dependency.

Eight focused Rust regressions, 950 all-feature host tests with 26 explicit
live/diagnostic ignores, 2/2 addon tests, formatting, strict selected/all-
feature release Clippy, Ruff, strict Pyright, the 108-package dependency proof
and 132/132 installed-wheel units pass. Two final ABI3 wheels are normalized-
reproducible; their SHA-256 values are
`752e68fd89b54c8dd4cb5a5552ff6c847cf46736fb4d9c70465e9c5b58e57320`
and `a1827bb7227124bb298d94d3b885dc317d7dc16c61fce7aa9560246c5f0a0f86`.
The byte-identical native extension SHA-256 is
`45fc56e1138798ad239e06c066b16e6e6ed8c5eab2064e9a7a9e1e92cf975024`.
All six unchanged official async Chromium rows pass candidate and oracle; the
final candidate stress passes 60/60. Fresh collection is 6,526/6,526 and the
cumulative ledger advances to 727 passed, zero failed, seven skipped and 5,792
not run, with exactly those six rows changing and no losses. The first 58/60
stress result is retained only as diagnostic evidence.

NODE-307 advances Python to ABI 66 and adds independently selectable
`persistent-context-locale` over `persistent-context` plus `context-locale`.
The persistent default locale is admitted before initial Page topology
initialization and consumes one of the existing 16 locale entries. Every
selected Page/OOPIF receives Playwright's Chromium pair. A shared stateless
boundary emits one `Emulation.setUserAgentOverride`, preserving any device
`userAgent` while composing `acceptLanguage` for `navigator.language`; the
locale initializer follows with `Emulation.setLocaleOverride` for ICU/`Intl`.
The leaf adds no Pwright package beyond the persistent-Context closure.

Seven locale regressions, four user-agent composition regressions, 948 all-
feature host tests with 26 explicit live/diagnostic ignores, 2/2 addon tests,
formatting, strict selected/all-feature release Clippy, Ruff, strict Pyright
and the 108-package dependency proof pass. Two final repaired wheels are
normalized-reproducible; their
SHA-256 values are
`d05505434a588149a878a1d06657cbfbecd4ff9046ad6cf79787e26fd1c5f870`
and `e3cbc6a377ce20492652baeb8f46ac49f79088dced638bcd8a8bddf40ebabb79`.
Installed-wheel units pass 132/132. A real-Chrome device-UA-plus-locale probe
passes 1/1. The final installed candidate and independent oracle each pass the
unchanged persistent-locale row; the candidate repeat passes 10/10 and the
prior Context-locale Worker row stays green.

Fresh final-wheel collection is 6,526/6,526. The cumulative ledger is 721
passed, zero failed, seven skipped and 5,798 not run, SHA-256
`352660d65526e5c2cbabfdf3d7115eb32e3e75ed5dc484b9f18efee66914f053`,
with exactly the selected persistent-locale Chromium row changing from not run
to passed and no losses. The first real-browser candidate failure is retained
only as diagnostic evidence: it returned `en-US` and exposed the missing
`acceptLanguage` override before the upstream command pair was completed.

NODE-306 advances Python to ABI 65 and adds independently selectable
`context-locale`. Rust retains at most 16 owned-Context locale strings of 256
UTF-8 bytes each and applies `Emulation.setLocaleOverride` during owned Page
and OOPIF initialization. Chromium shares locale across Pages in one renderer,
so the initializer suppresses only the exact upstream duplicate-override
protocol error and propagates every other failure. Dedicated and nested
Workers inherit the renderer locale exactly as in upstream Playwright; locale
adds no Worker command, registry, task, queue or feature.

Five locale regressions, 942 all-feature host tests with 26 explicit
live/diagnostic ignores, 2/2 addon tests, formatting, strict selected/all-
feature release Clippy, Ruff, strict Pyright and the 108-package dependency
proof pass. Two repaired wheels are normalized-reproducible; their SHA-256
values are `5748b3dd7310752b8bac8a7a39588b06e706f104f647cc1ac6be30aec3da24b1`
and `7865ee31d2dbdebcabbf18b9e51dc9fd47dcf45e8968a9c5e1f3743785200abb`.
Installed-wheel units pass 131/131, report SHA-256
`293ddb1ba023c74ef2e3682c0632de12e2138488626ab8506a491a5e7069987f`.
Every final-wheel runtime member is byte-identical to the browser-tested
corrected wheel, including native extension SHA-256
`3cfc824663b084756e8c4db9b3a34b07222947768eb26d26e813796b084108ee`;
only `METADATA`, `RECORD` and CycloneDX identity changed with the final README.

The unchanged Context-locale Worker row passes 1/1 in installed candidate and
independent oracle, and the candidate repeat passes 10/10. The complete
unchanged Chromium Worker file passes 13/13. Candidate, oracle, repeat and
complete-file report SHA-256 values are respectively
`adc3bef79f6fcf3a7d74525f9b5e22fccf627218c13d7ae7c255aa9758aa78b5`,
`2e7cc0bf2da134da5fc3a6666f77e52dfe425626407e19f627e84647ffc897a1`,
`42662339aad777d1c042dfd7880325c5a59a20778443bdab2f5008b84bf9ad4f`
and `523fa108aaa13c1f7ff529fe50b26c2f6bae46d924ef83fd7ff4c963566ca278`.
Fresh final-wheel candidate collection is 6,526/6,526, report SHA-256
`87de9dcf0da92a1da6fa886c70947f92747931f04759aff0bd59f67faf73084f`.
The cumulative ledger is 720
passed, zero failed, seven skipped and 5,799 not run, SHA-256
`20e36211f8f1bc6027c7c6998aa7060cb9d43262eadcaf7eb60561bfde4a66f3`,
with exactly the selected Chromium row changing from not run to passed and no
losses. Earlier provisional wheels and red duplicate-override runs are retained
only as diagnostic evidence.

NODE-305: Advance Python to ABI 64 and add independently selectable
`worker-console-argument-handles` over `worker-console-events` plus
`worker-handle-evaluation`. Rust validates and transfers every ordered console
RemoteObject as one atomic batch into the existing 64-entry Worker Handle
registry; a validation, queue or capacity failure compensates every
untransferred object exactly once. The event FIFO remains 128 notices/16 MiB,
and Handle calls retain the existing eight-concurrent-call and 64-cleanup-slot
bounds. Worker-created `JSHandle.json_value` now reuses the exact Worker
execution context, with retained primitives requiring no browser round trip.
Python only converts the native Handle descriptors and supports both the old
11-field and new 12-field console notice shapes. Five new Rust regressions pass;
the host passes 937 all-feature tests with 26 explicit live/diagnostic ignores,
the addon passes 2/2, and formatting, Ruff, strict Pyright, strict selected/
all-feature release Clippy and the 108-package dependency proof pass. The
unchanged official async Chromium console-Handle row passes in installed
candidate and independent oracle and passes 10/10 repeated candidate runs.
The complete unchanged Chromium Worker file is 12/13; only BrowserContext
locale remains. Candidate focused report SHA-256 is
`a35e11498bf94b74445e4b490ff848b56ba5dfbb32ec7248901166c5af909e51`;
oracle report SHA-256 is
`1e8aa023fca1ebf396ead20ca624f03b884f2b608470e621b6e9fdeff1a17747`;
stress report SHA-256 is
`857416b027e0c898741d07daa5b81343611d1c55b39289789b391c73ec1d1938`;
complete Worker report SHA-256 is
`c15afdc0e2f9bb7cefba76d1c89610455f8a70e5c44a5a59132cf7aed689ceb1`.
The cumulative ledger is 719 passed, zero failed, seven skipped and 5,800 not
run, SHA-256
`51b9b9c6e380774f82c279b7032131ff0fa0aefb59b059ec3deba014e67f3e69`,
with exactly one not-run-to-passed change and no losses. BrowserContext locale,
the remaining Python corpus and the complete Core JS/TS corpus remain open.
Two repaired ABI3 wheel SHA-256 values are
`6b0ce7e210c44031f228625dcdb5a5850a786298f36fbd1b3d54698a44c3c3c6`
and `870707e752cd95af55b70651aa4cbdb54861158fcaf6cc03fdf3a930e0788a60`;
their normalized payloads are reproducible after normalizing only CycloneDX
identity. The final installed-wheel unit report passes 131/131, SHA-256
`eadd928bedb83495537abda01706fe910140bc67d577d09ca04b28ded94aa24c`.
Three final-wheel browser attempts are excluded from behavior evidence: one
stopped before collection because the clean venv lacked the official oracle's
`pyee` dependency, one found a pre-existing target without a ready main world,
and one found another task's noisy page had filled pending console Handle
capacity. None reached the selected test body; the provisional installed-wheel
candidate/oracle evidence above remains the NODE-305 browser result.

NODE-304: Advance Python to ABI 63 and add independently selectable
`worker-handle-evaluation` over `worker-events` plus `handles`. Rust evaluates
and publishes opaque Handles in the exact retained Worker session and unique
execution context under the shared 64-Handle, eight-concurrent-call and
64-cleanup-reservation limits. Handle cores retain only a weak Worker owner,
so disposal cannot keep the Worker session or CDP connection alive through
Browser shutdown. Python adds only `Worker.evaluate_handle` and
`JSHandle.get_property` facades. Dependency proof shows the selected leaf has
the same Pwright package closure as Page events plus Handles. The final
repaired wheel passes 131/131 Python units, Ruff, strict Pyright, 932
all-feature host tests with 26 explicit live/diagnostic ignores, 2/2 addon
tests, strict selected/all-feature release Clippy and dependency proof. The
unchanged official async Chromium Worker Handle lifecycle row passes in the
final installed candidate and independent oracle, and the final-wheel
ten-repeat candidate run passes 10/10. The complete unchanged Chromium Worker
file is 11/13; only console argument Handles and Context locale remain.
Candidate focused report SHA-256 is
`aead603f57778f8cd6939a1a340006ba768a0c33aa475dff4dce5ae379cfb415`;
oracle report SHA-256 is
`512eb5d9416563d9166ff74ef0b8e394c863fe36f64ee4200c492ed2d812fc6a`;
stress report SHA-256 is
`fdd3c72e42ec4112b13a504308893e5bf923236701610bec27d1db6803adfa98`;
complete Worker report SHA-256 is
`1ded7bec96dbd0ae616d70f7a5b7ddca404f3e355c85fcf5bb6f0652aa430d17`.
The cumulative ledger is 718 passed, zero failed, seven skipped and 5,801 not
run, SHA-256
`e0dfeb150e8e410f6c4b4f62968330a9313023300d745d20f762def57bcc9501`,
with exactly one not-run-to-passed change and no losses. Fresh 6,526/6,526
collection SHA-256 is
`642a9f5f2fcfd338486db1fce733a596618a1586831f6af58c308f3aabb3e81b`.
Two normalized-reproducible repaired wheel SHA-256 values are
`88481e8e363eddd22de51e075579918158b3900aef1aeed3de6e9ec7f1a47215`
and `7163f0e7dd52412ab3d78e8a65c94d8702ba58de0c14ebe806a00fd0eb609692`.
Setup-only final-v1/final-v2 attempts, the zero-test stress-final-v1 selector
attempt and earlier diagnostic/hung reports remain excluded from final
behavior evidence. This checkpoint does not claim full Worker Handle parity or
full Python/Core corpus completion.

NODE-303: Keep Python at ABI 62 and add independently selectable
`worker-network-events` over `worker-events`, `request-events` and
`response-events`. Rust enables Network on retained Worker sessions before
debugger resume, routes exact related sessions to their owning Page, scopes
native IDs by `(session, request-id)` and shares the Page's existing
64-active-request limit and publication lane. Worker detach drains only that
scope. Python reuses one existing network watcher and bounded weak Page
identity maps. The final wheel passes 130/130 Python units, Ruff, strict
Pyright, dependency proof, strict selected/all-feature release Clippy, 927
all-feature host tests with 26 explicit live/diagnostic ignores and 2/2 addon
tests. Both unchanged official async Chromium Worker-network rows pass in the
final installed candidate and independent oracle; the ten-repeat candidate
run passes 20/20. The complete unchanged Chromium Worker file is 10/13; only
Worker Handle evaluation, console argument Handles and Context locale remain.
Candidate focused report SHA-256 is
`42e5087226d06cb439d66132b3aada80a1d68586821bbc243949355cf4b29dab`;
oracle report SHA-256 is
`7b88bbb11f719ed0330c05eddb610c8950782b832de73483b56f4060a6f7e65d`;
stress report SHA-256 is
`cc5c71028a5c0ff50c45458011050e8ebdbc0ebe2521724a6e6df252e8505c8a`;
complete Worker report SHA-256 is
`b13c2b50c32e45a4ed5bdd4518b211583ebe1458625b09bf4cf717718f5b8d3c`.
The cumulative ledger is 717 passed, zero failed, seven skipped and 5,802 not
run, SHA-256
`361e34e330c5fd5908802e561f126743e2a42f911f34d762283633a0333efbff`,
with exactly two not-run-to-passed changes and no losses. Fresh 6,526/6,526
collection SHA-256 is
`d43ff5a1d5478273618fec81cb0ee5d23c81d5f60ae67590eb5f3815c8ed9efe`.
Two normalized-reproducible repaired wheel SHA-256 values are
`4b97facee24ba3484b484b5361cf309dfa615a5f53098601feee81bb6950f9b1`
and `06986a96e172727ef31de0bc56ebeb72b04d8cb0be270c32a6f85ca52f016997`.
The initial setup-only report and the pre-identity-fix diagnostic report remain
excluded from final behavior evidence.

NODE-302: Keep Python at ABI 62 and add independently selectable
`worker-error-events` over `worker-events` plus `page-errors`. The Rust
Page-error observer preserves Frame/Page ownership precedence and falls back
to the exact retained Worker session's Page. It reuses the existing bounded
PageError queue/history and Python watcher, so there is no new native export,
task, FIFO, resource allowance or ABI bump. The final wheel passes 129/129
Python units, Ruff, strict Pyright, dependency proof, strict selected/all-
feature release Clippy, 927 all-feature host tests with 26 explicit
live/diagnostic ignores and 2/2 addon tests. The unchanged official async
Chromium Worker-error row passes in final candidate and independent oracle;
the ten-repeat candidate run passes 10/10. Candidate focused report SHA-256 is
`4b426debbd1724ad71de797c2f539bed0b2e6915a990e9a8bbf5e9d721bd83fb`;
oracle report SHA-256 is
`2b71de897545134ca5ff8727743bad9eaf38b875866188c9322a3e88220b9b3c`;
stress report SHA-256 is
`90571679bf40d10f6a7592f4301f7b867e73f0cfdebcd2fe6bb27066a8fc7c81`.
The cumulative ledger is 715 passed, zero failed, seven skipped and 5,804 not
run, SHA-256
`caf76e34c05bd6ce6da5b3467fc142f7abdaf05c545f8cf94f69578e0fe2a539`,
with exactly one not-run-to-passed change and no losses. Two normalized-
reproducible repaired wheel SHA-256 values are
`2db7c264dcc10c34f52103ca575df0a28812084c3423630ef2b098c98dc08054`
and `5e8f6cd0d851b9ab841b104230ee958ee4c631e77c2b46cfc5e445bc555ab9c5`.
Worker console argument Handles, Handle evaluation, Context locale, Worker
network attribution and the remaining Python/Core corpora stay open.

NODE-301: Advance Python to ABI 62 with independently selectable
`worker-console-events`. Rust correlates `Runtime.consoleAPICalled` with the
exact retained Worker session, owns a 128-notice/16 MiB FIFO and releases
diagnostic RemoteObjects through the bounded cleanup executor. Python runs one
Browser watcher and emits the same `ConsoleMessage` object to Worker, Page and
Context; `ConsoleMessage.worker` has exact public identity, while Worker
argument Handles remain explicitly unsupported. The feature adds no Pwright
package beyond the existing `page-events` closure. The final installed wheel
passes 129/129 Python units, Ruff, strict Pyright, dependency proof, strict
selected/all-feature release Clippy, 926 all-feature host tests with 26
explicit live/diagnostic ignores and 2/2 addon tests. Three unchanged official
async Chromium rows pass 3/3 in final candidate and 3/3 in the independent
oracle; a ten-repeat stress run passes 30/30. Candidate focused report SHA-256
is `d5a556b3377b7c34f73f2fb9be9f088988c317569cfa9097a019f75973415987`;
oracle report SHA-256 is
`e7128df0d47ad0936b15dbfdf903ff15a6db45c077f697819cc2f7ac82f90829`;
stress report SHA-256 is
`5c1320a89c96fa8c4355940f5ff7bc1b11d65b7e2b1c1111cf4c2f629836dbef`.
The cumulative ledger is 714 passed, zero failed, seven skipped and 5,805 not
run, SHA-256
`ef648ecece0f94e5f7a3968e6d652a08d1bb5eb56b8bf8c1afd7045f6d6b902c`,
with exactly three not-run-to-passed changes and no losses. Two
normalized-reproducible repaired wheel SHA-256 values are
`739d49ccdffd7c2e1685e0dd7d2ae6948718fe6b59758924826b7be2027217a5`
and `efd120f8b456e3734f0eae414fa35e4b4de71c9dd9a6b42904214917f8f0af46`.
Worker console argument Handles, Worker errors, Handle evaluation, Context
locale, Worker network attribution and the remaining Python/Core corpora stay
open.

NODE-300: Keep Python at ABI 61 and retire dedicated Workers from every Page,
Context and Browser terminal path. Python releases the Rust native owner,
emits public close once and rejects evaluation after retirement; exact late
detach notices for an already removed Page no longer retain an error traceback
and CDP connection. Four unchanged official async Chromium Worker rows pass
from the installed wheel; focused report SHA-256 is
`601621d526157d550fbdfd92dabf4546fb1574cef2da5f241382900a827470b2`.
The cumulative ledger is 711 passed, zero failed, seven skipped and 5,808 not
run, SHA-256
`cfcddd468eef3ef754910e1c7055ffbbbbc4c6454546589ff74323999acbf4fb`,
with four not-run-to-passed changes and no losses. Two
normalized-reproducible wheel SHA-256 values are
`aecd520722b35db70ee096eb030fbb55662e37d4c479a3efbffc050b6c26329c`
and `be4ed3912c802d2b30ce11d99361428c67740386266c5acd8e0301fa13913032`.
Worker console/error publication, Handle evaluation, Context locale, Worker
network attribution and the remaining Python/Core corpora remain open.

NODE-299: Keep Python at ABI 61 and add a caller-thread dispatcher compatible
with the official sync binding. Greenlets only suspend facade calls while all
browser coroutines remain on the bounded background loop. The installed wheel
passes the unchanged close/rejection row and the complete Chromium sync file
27/27. The cumulative ledger is 707 passed, zero failed, seven skipped and
5,812 not run, SHA-256
`a43bef1f0b558f324191e12cb2dc28d713d9108466c3bf68bf0edf09bc9bddc3`.
Exact comparison with NODE-298 shows one failed-to-passed change and no losses.
Two normalized-reproducible wheel SHA-256 values are
`eb078b9c0da926a28dd263fe831336839176e700ee6b90fa00fec207e8241d68`
and `ea38b110a1af14b0d88ae310b8c6c783a41dbe3ba37758475b5cc864d0115705`.
The remaining Python corpus and the complete Core JS/TS corpus remain open.

NODE-298: Advance Python to ABI 61 with independently selectable
`worker-events`. Rust owns dedicated-Worker child-session retention, exact Page
association, creation order, readiness, bounded evaluation and detach cleanup;
Python only adapts async/sync Worker facades through one Browser watcher. The
unchanged official Worker row passes and the complete Chromium sync file
improves to 26/27. The cumulative candidate ledger is 706 passed, one failed,
seven skipped and 5,812 not run, SHA-256
`67413210a814c04723bc62aa595210856f43891ed23e615622817b5465736a36`.
Exact comparison with ABI 60 shows one failed-to-passed change and no losses.
Two normalized-reproducible wheel SHA-256 values are
`dd3fc5ef46cac61ee3a54321f04fffe72d86fba962fcc69194e0bb65d0846158`
and `07bd3a2845afc60027a69a488714d7cf2e163ec2369b9fc23c2257bd011850d9`.
The official caller-loop `sync_gather` discrepancy, the remaining Python
corpus and the complete Core JS/TS corpus remain open.

NODE-297: Advance Python to ABI 60 with independently selectable
`download-events`. Rust owns exact-Context Chromium policy, native
Frame/Page/Context correlation, bounded GUID-named artifact storage, completion
state and cleanup; Python only adapts async/sync `Download` objects through one
Browser watcher. The Rust click navigation barrier also treats a same-Frame
download as the terminal success of a navigation that became a download. The
unchanged official download row passes and the complete Chromium sync file
improves to 25/27. The cumulative candidate ledger is 705 passed, two failed,
seven skipped and 5,812 not run, SHA-256
`c7c6e0f6a827db762291614bbfa127205cec572b1c1328250ad1dc1e71da6f9b`.
Exact comparison with ABI 59 shows one failed-to-passed change and no losses.
Two normalized-reproducible wheel SHA-256 values are
`e84ee8646f093d676b28fcb8a507040cbefb2c1802dc519f90f496120c6744f4`
and `0b152147e6eb49901b113ec7541d88a0343248466373b806c68750be29d294e0`.
Full Python and Core JS/TS parity remains open.

NODE-296: Advance Python to ABI 59 with an independently selectable
`request-events` projection over the existing Rust network observer. Rust owns
bounded Context subscriptions, exact Page association, retained Request
owners, a 256-notice FIFO, shared 16 MiB queue accounting and topology/Browser
cleanup. Python runs one Browser-scoped native watcher and only adapts public
async/sync Request wrappers and callback identity. The unchanged official
network-event row passes and the complete Chromium sync file improves to
24/27. The cumulative candidate ledger is 704 passed, three failed, seven
skipped and 5,812 not run, SHA-256
`5a4815de54d0993e383f53e960378cec14199a86753379702ddf167b23aba983`.
Exact comparison with ABI 58 shows one failed-to-passed change and no losses.
Two normalized-reproducible wheel SHA-256 values are
`c4d90b75345d401d91820a8df0e7125ed6c8f042c0a7e969904d74577e636f50`
and `a335c4f2804328ec1923b9b170a58305a7052ba3780ffaa5cf7eeea726951a6d`.
Full Python and Core JS/TS parity remains open.

NODE-294: Advance Python to ABI 57 with an independently selectable
`console-events` projection over the existing Rust console observer and Page
lifecycle fence. Rust owns event observation, Page association, ordering,
bounded message snapshots and publication; Python only constructs public
async/sync `ConsoleMessage` wrappers and dispatches callbacks. The 256 KiB
lifecycle-conversion reservation now has compile-time proofs for the maximum
console and Page-error payloads. Full-profile release build, all-feature strict
Clippy, the 108-package dependency proof, Ruff, strict Pyright and 112/112
Python units pass. `ConsoleMessage.args` deliberately remains unsupported
until Rust publishes retained argument Handles; the JSON diagnostic snapshot
is not exposed as fake Handles. No new official browser result or ledger
movement is claimed: the shared dedicated Chrome still contains an unrelated
Gemini page, so the unchanged sync console rows remain pending and the ABI 56
candidate counts below remain current.

NODE-293: Complete the source-side Node `element-handle-dom` extraction.
Rust now owns exact-target `textContent`, ordered descendant batch publication,
the temporary query-array lifetime for `$$eval`, XPath scoping and
non-Element filtering. Node only validates public arguments and publishes
wrappers; `Page.textContent` and `Frame.textContent` are thin Locator
delegations. Package 273/273, strict TypeScript, the selected native build/load,
standalone strict Clippy and dependency boundaries pass. No new official
browser result or ledger movement is claimed yet: the zero-tab guard found an
unrelated Gemini page in the shared dedicated Chrome, so the unchanged 17-row
candidate/oracle rerun remains pending. The ledgers therefore remain at the
NODE-292 counts below and the full corpora remain incomplete.

NODE-292: Select the existing Rust `element-handle-dom` leaf independently in
the Node artifact. `ElementHandle.innerText`, `$` and the first-match part of
`$eval` now cross N-API as bounded typed calls; Rust owns descendant selector
compilation, exact retained-target evaluation, null release and
`HTMLElement.innerText`. `Page.innerText` and `Frame.innerText` are thin
delegations to the existing Rust Locator. The unchanged official query,
selector-evaluation and inner-text selections pass 15/15 in both candidate and
oracle. The merged Core ledgers are candidate 1,216 expected / one skipped / 31
unexpected / 47,229 not run, SHA-256
`6d98659a9a5de4d92c3d74ae4ee6fad7e836c3005384f4423ce20e85bf8a2a08`,
and oracle 899 expected / 47,578 not run, SHA-256
`a63c4bcd6dcd69395a758e7700e8e0e83923e1edd0c669c53641e57c97bcdb5b`.
Node `$$`, `$$eval` and `ElementHandle.textContent` are still explicit
Rust-extraction work; the complete Core and Python corpora remain incomplete.

NODE-291: Advance Python to ABI 56 with the independently selectable
`element-handle-dom` composition. Rust owns descendant selector compilation,
existing-handle evaluation, first-match publication, no-match release and
`HTMLElement.innerText`; Python only exposes async/sync wrappers. The two
previously failing official Chromium rows now pass 2/2 from the final wheel.
The repaired sync tranche is 16 passed / nine failed, so the whole file is
18/27 candidate versus 26/27 oracle. The cumulative candidate ledger is 682
passed, nine failed, seven skipped and 5,828 not run, SHA-256
`a5a1ce1effe030bc97ad9cb9c2405c0f7ee20ed9ba0961f34770f314945501a1`.
The corpus remains explicitly incomplete and red.

NODE-290: Keep Python ABI 55 and execute all 25 previously unrecorded Chromium
rows in unchanged sync `test_sync.py`. The independent oracle passes 24 and
fails the same `test_sync_playwright_multiple_times` asyncio-loop assertion as
the candidate. The candidate baseline is 9 passed / 16 failed; thin public
repr facades and publication of the existing Rust request-history association
through `Response.request` repair five rows without moving browser semantics
into Python. The repaired candidate is 14 passed / 11 failed for this tranche,
and every one of the file's 27 Chromium rows now has an outcome. The cumulative
candidate ledger is 680 passed, 11 failed, seven skipped and 5,828 not run,
SHA-256
`099db2a6f4937871240a3fb7c329606cb2f9c81fccb58932534bc3c0ef45116e`.
The file and full corpus remain explicitly red; no full-suite claim is made.

NODE-289: Keep Python ABI 55 and complete the five applicable Chromium rows in
unchanged async `test_asyncio.py`. The two newly selected TaskGroup and API-name
rows pass in final installed candidate and independent oracle. The candidate
baseline exposed one public-boundary discrepancy: the correctly classified
`JavascriptError` lacked Playwright's `Page.evaluate:` prefix. Python now
rewrites only that public error shape while preserving native type, code, name
and stack; Rust remains the classifier. The cumulative candidate ledger is 666
passed, seven skipped and 5,853 not run, SHA-256
`2e8f750a7e2a16af688489ef6aff90fc9467946109b18a007eccfb5e4626fd0d`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-288: Keep Python ABI 55 and add Playwright-shaped `.name` access to the
existing native error taxonomy. Three Chromium selector-wait rows pass in
installed candidate and independent oracle: cancellation without an
unretrieved future, timeout error naming and sync Page waiting. The cumulative
candidate ledger is 664 passed, seven skipped and 5,855 not run, SHA-256
`7f8c62ece39f4634eba752e80441a53aabf4f4272cffd5ce84689a75becddfef`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-287: Advance Python to ABI 55 with thin async/sync
`Page.wait_for_selector` facades over the existing Rust Locator wait and Handle
owners. Two applicable Chromium rows from unchanged async `test_issues.py`
pass in installed candidate and independent oracle, including cancellation of
an unresolved wait. Generic `launch(ignore_default_args=...)` and the red
MagicDNS HTTP service-worker diagnostic remain explicit applicability gaps.
The cumulative candidate ledger is 661 passed, seven skipped and 5,858 not
run, SHA-256
`169dfbabd9bf984d59082cdf4c0b68a84941a67c7e04c8997cc63645243561e9`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-286: Advance Python to ABI 54 with the separately selectable
`file-chooser-events` composition. `crates/pwright-node-host/src/page_file_chooser.rs`
intercepts the exact Page/OOPIF session, publishes `Page.fileChooserOpened`
through the existing Page lifecycle fence, and reuses the bounded file-input
payload/path owner for exact backend-node mutation. The complete unchanged sync
input file passes both Chromium rows in final repaired-wheel candidate and
independent oracle; four Firefox/WebKit rows remain explicit provider gaps. A
strong native connection retained by the first owner caused a verified teardown
hang; the final weak route fixes it without widening lifecycle ownership. The
cumulative candidate ledger is 659 passed, seven skipped and 5,860 not run,
SHA-256
`557e937c00ccc8e9065c37df2c14fde9b7d71ae0c1b9e011f02171b52d71730a`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-285: Advance Python to ABI 53 with separately selectable Rust-owned
Context request waits, exact request-history ownership, same-cursor Frame
availability and post-data JSON parsing. The complete unchanged async Page
network-request file passes both Chromium rows in final installed candidate
and independent oracle; four Firefox/WebKit rows remain explicit provider
gaps. The cumulative candidate ledger is 657 passed, seven skipped and 5,862
not run, SHA-256
`83ed08bf4d1f711e6d506db5a2315bc6e7aed49dff4717d0db95750074c0318b`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-284: Advance Python to ABI 52 with independently selectable bounded
Rust-owned `Response.finished()` completion. Both complete unchanged async and
sync Page network-response files pass all four Chromium rows in final installed
candidate and independent oracle; eight Firefox/WebKit rows remain explicit
provider gaps. The cumulative candidate ledger is 655 passed, seven skipped
and 5,864 not run, SHA-256
`82dad9ef03972b03a4174dad73b07409b59776839d2e9885a01e81fb35f6f0c7`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-283: Advance Python to ABI 51 with a separately selectable bounded
Rust-owned `wait-for-timeout` leaf and thin async/sync Page facades. The
complete unchanged shared signal file passes both Chromium rows in final
installed candidate and independent oracle through the same dedicated CDP
Chrome; Firefox/WebKit remain four explicit provider gaps. The cumulative
candidate ledger is 651 passed, seven skipped and 5,868 not run, SHA-256
`b64fcdc19f027fd37718a339b122ecb6ea240890e12b4c56c147baac4f3ae0e4`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-282: Keep Python ABI 50 and add a one-error-bounded sync bridge so Page
listener exceptions return through the public call boundary after the call
finishes. The complete unchanged shared event file passes its Chromium row in
final installed candidate and independent oracle; Firefox/WebKit remain
explicit provider gaps. The cumulative candidate ledger is 649 passed, seven
skipped and 5,870 not run, SHA-256
`3e75c9588a73cd741ac571800a430e290affb178aee3bed5ab0951b46987602d`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-281: Keep Python ABI 50 and execute the complete unchanged shared
`test_threads.py` file. Candidate and independent oracle pass the Chromium
worker-thread lifecycle row without product changes; Firefox/WebKit remain
explicit provider-gap `not-run` rows. The cumulative candidate ledger is 648
passed, seven skipped and 5,871 not run, SHA-256
`4febdfe050081de20ef509f153f4e77669eb2672d22be1a07853d9e2e1916345`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-280: Keep Python ABI 50 and add bounded Playwright-scoped Rust ownership
for mutable test-id configuration plus bounded strict-error DOM previews. The
single Chromium row in the complete unchanged async selector-generator file
passes in final installed candidate and independent oracle; Firefox/WebKit
remain explicit provider-gap `not-run` rows. The cumulative candidate ledger
is 647 passed, seven skipped and 5,872 not run, SHA-256
`38ab5abac559594a16e9faedab66525f7a168e4bddcf765da1b294a30139a10d`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-279: Keep Python ABI 50 and add typed Rust-owned legacy text/regex
selectors, quote-aware selector-chain tokenization, bounded Page selector
handle collection, non-strict Page selector evaluation and thin `to_have_id`
assertions. Both Chromium rows in the complete unchanged async legacy-text
selector file pass in final installed candidate and independent oracle; its
four Firefox/WebKit expansions remain explicit provider-gap `not-run` rows.
The cumulative candidate ledger is 646 passed, seven skipped and 5,873 not
run, SHA-256
`603945f33ceadde881b175508db8402092fe71f246b3a025ea2b9277227fde25`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-278: Advance Python to ABI 50 with bounded Rust-owned Page, Locator and
exact retained ElementHandle screenshot paths, including bounded mask setup
and document-space clipping. All three Chromium rows in the complete unchanged
async screenshot file pass in final installed candidate and independent
oracle; the six Firefox/WebKit expansions remain explicit provider-gap
`not-run` rows because the source has no Chromium-only marker. The cumulative
candidate ledger is 644 passed, seven skipped and 5,875 not run, SHA-256
`f898f89d210cfb1c73d37b5817b68688050fae9f273c54c92480f4e0375aaf5c`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-277: Advance Python to ABI 49 by extending the existing Rust-owned
`page-pdf` leaf with typed tagged/outline booleans. All three applicable
Chromium rows in the complete unchanged async PDF file pass in final installed
candidate and independent oracle; six Firefox/WebKit expansions remain
explicit upstream Chromium-only non-applicable rows. The cumulative candidate
ledger is 641 passed, seven skipped and 5,878 not run, SHA-256
`311867644fb8f62e11af82b1cdb08ce58392f1a4dfe2a03bc0bd7cc1ff387cd8`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-276: Advance Python to ABI 48 with an independently selectable bounded
Rust-owned `page-pdf` leaf. Both applicable Chromium rows in the complete
unchanged sync PDF file pass in the final installed candidate and independent
oracle; the four Firefox/WebKit expansions remain explicit upstream
Chromium-only non-applicable rows. The cumulative candidate ledger is 638
passed, seven skipped and 5,881 not run, SHA-256
`801181534ae92df917d69d02f0d03708981c14ab8a171fefa7086e12acae3b02`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-275: Advance Python to ABI 47 with a bounded, independently selectable
Rust-owned `ignore-https-errors` Context/session policy. Both unchanged
Chromium rows in the complete async HTTPS-error file pass in candidate and
independent oracle; the four Firefox/WebKit expansions remain explicit
provider-gap `not-run` rows. The cumulative candidate ledger is 636 passed,
seven skipped and 5,883 not run, SHA-256
`690f4b86c509932323cd613f2f40fede63a06e86133209bf98944421ae3d70d8`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-274: Keep Python ABI 46 and execute the complete unchanged async/sync fill
and context-manager files. Candidate and independent oracle each pass all nine
Chromium rows. Page, BrowserContext and Browser now expose thin language-level
context-manager protocols over existing close owners; the oracle launch
adapter hides only Contexts that predate adapted launch. The cumulative
candidate ledger is 634 passed, seven skipped and 5,885 not run, SHA-256
`8bb2f5b5437477779c6eb6690da2f247d3f377422f62b1d8bd3ee0cafb5f3908`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-273: Advance Python to ABI 46 with native Locator collection evaluation
over the existing typed Rust recipe. The complete unchanged async
`test_selectors_get_by.py` Chromium file passes 3/3 in candidate and oracle,
covering regex/exact escaping, role accessible names and `include_hidden`.
The cumulative candidate ledger is 625 passed, seven skipped and 5,894 not
run, SHA-256
`39fc2c66a96066e5d0275b146688a48a7dbb1e8a3aeaf37f3971d598cbee4e4b`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-272: Advance Python to ABI 45 with typed Rust-owned semantic Locator
leaves and Rust-owned attribute-expectation polling. The complete unchanged
sync `test_locator_get_by.py` Chromium file passes 10/10 in both installed-
candidate and independent-oracle lanes; the pinned upstream tree has no async
twin. The cumulative 6,526-row candidate ledger is 622 passed, seven skipped
and 5,897 not run, SHA-256
`e37434aa74c4dea2f6f8001ae27d657e833cf0c197207b0186b5d133b9a40813`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-271: Advance Python to ABI 44 with a lazy Rust-owned FrameLocator recipe,
typed dynamic-frame re-rooting, visibility expectations, strict Locator call
logs and the official Python public-handler arity adapter. The final installed
wheel passes every unchanged async/sync Chromium Locator row, 86/86 plus
77/77, matching the independent 163/163 oracle. The cumulative 6,526-row
ledger is 612 passed, seven skipped and 5,907 not run, SHA-256
`89eabed04a3a7804637229ff077cf0898cc1aa903c190611bfb2d1553650aefe`.
All unexecuted Python and Core JS/TS rows remain explicit and open.

NODE-270: Advance Python to ABI 43 and compose the existing Rust-owned Locator
bounding-box and independently selectable Locator-screenshot capabilities.
All four viewport-dependent official rows pass; complete Locator execution is
145/163, exactly four gains and no losses. The cumulative 6,526-row ledger is
594 passed, 18 failed, seven skipped and 5,907 not run, SHA-256
`811c14419d0fee688717168bfb0118d07cfaa82c27d9d8c2e0f14650d01348fe`.
The remaining Python and Core JS/TS rows stay explicit and open.

NODE-269: Advance Python to ABI 42 for a bounded, independently selectable
Rust-owned Page viewport registry and session-initialization replay. Preserve
the four-row red diagnostic because viewport mutation succeeded before the
tests exposed missing Locator screenshot and bounding-box facades; do not count
those rows as passing until NODE-270.

NODE-268: Keep Python ABI 41 and make capture-plus-`nth` errors, bounded
Locator click call logs, role descriptions, Unicode timeout diagnostics and
the admitted simple relative-XPath `has=` subset Playwright-shaped in Rust.
All eight targeted official rows pass; complete Locator execution reaches
141/163 with exactly eight gains and no losses. The cumulative 6,526-row
ledger is 590 passed, 22 failed, seven skipped and 5,907 not run, SHA-256
`6180ffa49de77b48b2a61d13a8fe3f159654c4e5c4529d841082377112d0ca46`.
The remaining Python and Core JS/TS rows stay explicit and open.

NODE-267: Advance Python to ABI 41. Add independently selectable Rust-owned
Locator `dispatch_event` and `select_text`, and route Locator
`evaluate_handle` through the existing bounded Handle owner. All six targeted
official rows pass; complete Locator execution reaches 133/163 with exactly
six gains and no losses. The cumulative 6,526-row ledger is 582 passed, 30
failed, seven skipped and 5,907 not run, SHA-256
`1feb3bb3ec44812ee930defa5471cd8e8454e2d8c49e9502059273f1c360fe41`.
All remaining Python and Core JS/TS rows stay open.

NODE-266: Advance Python to ABI 40 and expose the existing bounded Rust
Locator-highlight capability as a separate feature. Targeted async/sync rows
and the complete 163-row Locator tranche pass 127/163, exactly two gains and
no losses. Preserve the Chrome-restart-interrupted async report and repair its
seven setup-affected rows with exact JUnit replacements. The cumulative
6,526-row ledger is 576 passed, 36 failed, seven skipped and 5,907 not run,
SHA-256
`6976549c14fa4000f16617a0226b53753f54580907a5294aa413eea1d40f2cce`.
All remaining Python and Core JS/TS rows stay open.

NODE-265: Advance Python to ABI 39 for Rust-owned double-click, bounded file
input and element-handle publication. Complete Locator execution reaches
125/163 with 11 gains and no losses.

NODE-264: Keep Python ABI 38 and implement visible selector-chain composition
inside the bounded Rust query/runtime. Complete Locator execution is now
114/163 with exactly four gains and no losses; the cumulative 6,526-row ledger
is 563 passed, 49 failed, seven skipped and 5,907 not run. Strict
Rust/Python/addon and normalized two-wheel gates pass. The remaining Python
and Core JS/TS corpora stay open.

NODE-263: Advance Python to ABI 38 for typed role/test-id leaves and native
single-Locator evaluation. Complete Locator execution reaches 110/163 with
three gains and no losses.

NODE-262: Advance Python to ABI 37 for native Locator wait/expectation and
remaining filter/or semantics. Complete Locator execution reaches 107/163
with 24 gains and no losses.

NODE-261: Advance Python to ABI 36 for native typed filters and combinations.
Complete Locator execution reaches 83/163 with four gains and no losses.

NODE-260: Complete isolated async/sync Locator repair evidence, prevent eager
borrowed-Context network capture during CDP attach, update ledgers and all
Rust/Python/Node/package verification gates. Preserve borrowed pre-existing
Context demand-driven activation and the remaining 84 discrepancies as open.

NODE-259: Reuse the ABI 35 native Locator owner for bounded all/frame-rooted
composition and additional actions. Record 19 complete-file pass transitions
without deleting failures.

NODE-258: Run every official async/sync Chromium Locator row in candidate and
oracle lanes. Record candidate 60/163 versus oracle 163/163 before repairs.

NODE-257: Advance Python to ABI 35 and export an immutable native `Locator`
recipe owner so chaining plus `first` / `last` / `nth` stay typed and Rust-
owned rather than becoming Python-authored selector strings. The fixed
manifest SHA-256 is
`78e5b01d4582222168c51fea88add9a89987374914de5bac4711e2a7e4d4755f`.
Its four async and two sync Chromium invocations fail 4/4 plus 2/2 against ABI
34 with missing members, then pass 4/4 plus 2/2 in both the final installed-
wheel candidate and independent oracle. No Cargo feature, Pwright package or
dependency edge is added. Final wheel payloads are reproducible, installed
unit tests pass 39/39, Node adapter tests pass 270/270 and collection remains
6,526/6,526. Current ledgers are candidate 505/7/6014, SHA-256
`791513dec404b04ed5521aa031524c418703dc78f2f59adced1016c0efed07a7`,
and oracle 486/7/6033, SHA-256
`efdd8cd287445c9dadd84942ebd64f4e2bb9f565f06928bda3c4eeaa3423d775`.
[Python ABI 35 typed Locator evidence](python-ffi-compatibility-design.md#abi-35-typed-locator-composition-checkpoint-2026-09-28).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-256: Add Python `Locator.description` / `describe` as immutable language-
facade metadata with no browser behavior, protocol state or resource owner.
The fixed async-only manifest SHA-256 is
`d578cddad1cbb56d29d098b4fb1c19a20bd1d24a739809a25bdacb03d6591374`.
Its three unchanged Chromium invocations pass 3/3 in the final installed-wheel
candidate and 3/3 in the independent oracle; the initial three
`AttributeError` failures remain diagnostic evidence. ABI stays 34 with no
Cargo capability, package or dependency edge. Final wheel payloads are
reproducible, installed unit tests pass 38/38 and collection remains
6,526/6,526. Current ledgers are candidate 499/7/6020, SHA-256
`c5e89f60ce3e252855f80dd5ca8bcd9444cb089f62b61cb60eee04586d29cc67`,
and oracle 480/7/6039, SHA-256
`100d1c8ef02411cbfd506042600f9b7c90ea54f53b74478c571fc276a7a151f6`.
[Python ABI 34 Locator description evidence](python-ffi-compatibility-design.md#abi-34-locator-description-checkpoint-2026-09-28).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-255: Add Rust-owned Locator count expectation polling and expose it through
thin async/sync Python and Node assertion facades. The fixed manifest SHA-256
is `ed03f8d21b9d27cd400b3a3cb98203a20a35fdc4c7f57aa6e03ec00c92411e09`
and selects ten unchanged Chromium invocations from the pinned async/sync
`test_locators.py` sources. Candidate and independent oracle each pass 5/5
async plus 5/5 sync. Python ABI advances to 34 for
`locator_expect_count`; no Cargo capability, package or dependency edge is
added. Repaired wheel payloads are reproducible, installed unit tests pass
36/36, Node adapter tests pass 270/270, and full Python collection remains
6,526/6,526. Current ledgers are candidate 496/7/6023, SHA-256
`0562df16dd47427a0aa6bd6e6071b833c4a29dc88c5998d8e88aff5a23276649`,
and oracle 477/7/6042, SHA-256
`14912795ba9cbb4efc8a6c3713506dc039094c497ab0f14806f5cd8ee160ba8b`.
[Python ABI 34 Locator expectation evidence](python-ffi-compatibility-design.md#abi-34-locator-execution-and-count-expectation-checkpoint-2026-09-28).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-254: Expose basic async/sync Locator reads, collection reads, actions and
evaluation as thin Python facades over the existing Rust owners, and extend the
Rust selector recipe with typed Playwright `:text("...")` parsing. A fixed
34-row Chromium selection from the unchanged async/sync locator files passes
34/34 candidate and 34/34 independent oracle. The initial 34 `AttributeError`
diagnostics and the intermediate 16/17 candidate report remain outside the
green ledger. ABI stays 33 and no capability or package edge is added. Final
wheel payloads are reproducible, installed unit tests pass 34/34 and full
collection is 6,526/6,526. Current ledgers are candidate 486/7/6033, SHA-256
`2a956ea3e610f1a8ddad4b92cce0dde918a86aaa3e46a0eb915a5525cc925437`,
and oracle 467/7/6052, SHA-256
`7a5412b7212fdf7c4e3a6828d411febc65e922dc4be5bfa046210e67e2b3563b`.
[Python ABI 33 basic Locator evidence](python-ffi-compatibility-design.md#abi-33-basic-locator-facade-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-253: Add `layout-selectors` as an independently selectable Rust host
capability without changing Python ABI 33. Rust parses and compiles the
official `left-of`, `right-of`, `above`, `below` and `near` layout clauses,
including the tested top-level adjacent-sibling composition, then performs the
geometry scoring in the browser world; Python only forwards selector input.
The unchanged async/sync Chromium layout rows pass 2/2 in the final installed-
wheel candidate and 2/2 in the independent oracle. Two wheel payloads are
reproducible, installed unit tests pass 33/33, and collection remains
6,526/6,526. Current ledgers are candidate 452/7/6067, SHA-256
`295722cd54450259098ff79cd59a5f47cb59926f2d4403f8292d47f481f373df`,
and oracle 433/7/6086, SHA-256
`263f3100d25d77d1de6b705e6b4cf152ac439857da9f5caa769f1bf055ed6a98`.
Arbitrary nested layout/CSS shapes, retry polling, positive browser-program
`Page.query_selector` handle publication, Firefox/WebKit and the remaining
Python/Core corpora stay explicit gaps.
[Python ABI 33 layout evidence](python-ffi-compatibility-design.md#abi-33-layout-selector-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-252: Add Python ABI 33 registered selector-engine execution over the
bounded Rust registry and locator-evaluation owner. Rust compiles a bounded,
immutable selector program, installs each referenced engine once, and chooses
the main or utility world from `content_script`; Python only forwards calls.
The unchanged async/sync Chromium base, path, and main/isolated-world rows pass
6/6 in the installed-wheel candidate and 6/6 in the independent oracle. Two
ABI 33 wheel payloads are reproducible, installed unit tests pass 33/33, and
collection is 6,526/6,526. Current ledgers are candidate 450/7/6069, SHA-256
`52fe9d0c64ecf31fb34098e468053265f79eddd68a8cb29dcddab2f75e45c675`,
and oracle 431/7/6088, SHA-256
`e38018a3c4207b9fdb0995bf8b121d2c455510c49a9440275b2c329cefde8bd3`.
Positive registered `Page.query_selector` handle publication, typed Locator
composition on registered plans, retry polling, Firefox/WebKit and the
remaining Python/Core corpora stay explicit gaps.
[Python ABI 33 custom-engine evidence](python-ffi-compatibility-design.md#abi-33-registered-selector-engine-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-251: Add Python ABI 32 native selector validation before
`Page.query_selector` reaches the browser. This preserves the exact Playwright
unknown-engine error instead of leaking a DOM CSS `SyntaxError`, without
parsing selector syntax in Python. The unchanged async and sync Chromium
registration-error rows pass 2/2 in the installed-wheel candidate and 2/2 in
the independent oracle; their initial candidate failure remains outside the
green ledger. Two ABI 32 wheel payloads are reproducible, installed unit tests
pass 32/32, and collection is 6,526/6,526. Current ledgers are candidate
444/7/6075, SHA-256
`7b3fb579c03cf79a5869613890e06506903142ae8e4627de877e2f8bc4f0d6f3`,
and oracle 425/7/6094, SHA-256
`89d888ddfe687e446614a35d5a24467e7b3123678242d1191129d93fb0d9b727`.
Custom selector-engine execution and non-Chromium providers remain open.
[Python ABI 32 selector-validation evidence](python-ffi-compatibility-design.md#abi-32-selector-validation-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-250: Add Python ABI 31 global selector registration as an independently
selectable Rust-owned capability. The registry admits at most 64 engines,
64-byte names, 1 MiB per source and 8 MiB total; Python only normalizes string
or bounded-path input and forwards it. The exact unchanged Chromium duplicate-
registration row from `tests/async/test_selectors_misc.py` passes 1/1 in the
final installed-wheel candidate and 1/1 in the independent oracle. Custom
selector-engine execution is still open. Final wheel payloads are reproducible,
installed unit tests pass 32/32, and collection is 6,526/6,526. Current ledgers
are candidate 442/7/6077, SHA-256
`80a4c3855a5a064ef26c467eae02b50b960e93e0faff250fda1693aa8f0a1445`,
and oracle 423/7/6096, SHA-256
`cddd8fff7571cd00a132fc91ac1c3a722d9bbf4aba1807efc1c55390a788ab7b`.
Firefox/WebKit remain explicit `not-run` provider gaps.
[Python ABI 31 selector-registration evidence](python-ffi-compatibility-design.md#abi-31-selector-registration-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-249: Complete the ABI 30 async regression for the same collection
selector capability without changing the native ABI, feature graph or public
surface. The exact unchanged Chromium invocation from
`tests/async/test_selectors_misc.py` passes 1/1 in the installed-wheel
candidate and 1/1 in the independent oracle. Candidate and oracle report
SHA-256 values are
`e6fa55d7b30db92b8d1e2fe3618f75157af502f461711f7df0ed7d7d452f9c4f`
and `c989503c090d5c1e36bcf45f6b4302c6eb051c509f6592c5cf71060ac449bb71`.
Current ledgers are candidate 441/7/6078, SHA-256
`39d6bda545c2f7ce3a21252ee63903b8624732de1fbb29aaaaab6133259d0c86`,
and oracle 422/7/6097, SHA-256
`318ff5c4cfdb01fdeb833c027fed4ead7765dc32fad7d881f8512a01f78e7be8`.
Firefox/WebKit remain explicit `not-run` provider gaps.
[Python ABI 30 async selector evidence](python-ffi-compatibility-design.md#abi-30-async-selector-regression-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-248: Add Python ABI 30 `Page.eval_on_selector_all` over the existing
bounded Rust collection-evaluation path and parse Playwright's
`internal:and=<JSON selector>` as selector intersection rather than descendant
CSS. The exact unchanged Chromium invocation from
`tests/sync/test_selectors_misc.py` passes 1/1 in the installed-wheel
candidate and 1/1 in the independent oracle. The connection replay budget is
now independently bounded at 8,192 event items and 512 MiB of retained event
payloads, so `Runtime.enable` console-history bursts do not imply a 2 GiB
retention declaration. Two ABI 30 wheel payloads are reproducible, the
installed wheel passes 29/29 local unit tests, and collection is 6,526/6,526.
Current ledgers are candidate 440/7/6079, SHA-256
`65bd9d4261400421fa079ceea93780052d25c11b19e92a3ad8d2ea25fa75862b`,
and oracle 421/7/6098, SHA-256
`c4f10b1c972140a3414e48902f610e4a99f8b18384daac925bad89bd9e6db116`.
[Python ABI 30 selector evidence](python-ffi-compatibility-design.md#abi-30-collection-selector-evaluation-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-247: Complete the Python ABI 29 listener checkpoint with the exact pinned
Chromium invocation from the unchanged `tests/sync/test_listeners.py` source.
Candidate and independent oracle pass 1/1; the full-file candidate diagnostic
keeps Firefox/WebKit as explicit provider errors and is not merged into the
green ledger. Reconnect now preserves live network lanes while the
authoritative snapshot is incomplete, then releases only Pages absent from the
finished snapshot. The bounded CDP event replay window is 2,048 entries for
the declared 16-Page/128-session/256-world host profile; recovery remains one
snapshot and one epoch advance, not a retry loop. Two final wheel payloads are
reproducible, the installed wheel passes 28/28 local tests, and collection is
6,526/6,526. Current ledgers are candidate 439/7/6080, SHA-256
`4da01fe20cf788ee1a12f28509708166ed31b5f3eb3cefd78672ba4cddc3ffee`,
and oracle 420/7/6099, SHA-256
`f654d55f1bb21a0382a2bca2636438070a0c14e3b92cdcd1d7c2e643b6f9ea90`.
[Python ABI 29 sync evidence](python-ffi-compatibility-design.md#abi-29-sync-listener-completion-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-246: Add Python ABI 29 bounded Page `response` listener publication as
an independently selectable Rust leaf over `network-events`. Rust activates
response interest before Context publication, shares the existing 16 MiB
network queue budget, admits 16 live Contexts and 256 queued notices, and
coordinates the response subscription with `request-history`. The host now
recovers sticky CDP topology event loss with one authoritative same-connection
reconstruction and one monotonic epoch advance; materialized Page membership
is kept separate from transient command-session routing during reconnect. The pinned
Chromium invocation from the complete unchanged `tests/async/test_listeners.py`
source passes 1/1 in both final-wheel candidate and independent oracle lanes;
Firefox/WebKit remain explicit provider gaps. Final wheel payloads are
reproducible, the installed wheel passes 28/28 local tests, and collection is
6,526/6,526. Current ledgers are candidate 438/7/6081, SHA-256
`382e3c5a4fa98f8bd22e9503066d21a0236ded7f7f823a10b8e7ee3d17e439c2`,
and oracle 419/7/6100, SHA-256
`a0d41d0fe578361d80998cc60cdc25b7c07edb44e455061eccf64640785ec13f`.
[Python ABI 29 evidence](python-ffi-compatibility-design.md#abi-29-response-listener-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-245: Add Python ABI 28 device descriptors and Context-scoped Chromium
device emulation as an independently selectable Rust leaf. Rust embeds the
complete 143-entry Playwright v1.58.0 catalog, admits at most 16 configured
Contexts and 64 KiB of retained settings, and applies viewport, screen, device
scale factor, mobile, touch and user agent inside the existing pre-publication
session-initialization fence. The complete unchanged official device file is
1 passed plus two upstream browser skips in both final-wheel candidate and
independent oracle lanes. Final wheel payloads are reproducible, the installed
wheel passes 26/26 local tests, and collection is 6,526/6,526. Current ledgers
are candidate 437/7/6082, SHA-256
`c02c13e38b6b3dae4d83ec8ad280088dabd7432c30b70ecfeb83e83bf9e5cfc3`,
and oracle 418/7/6101, SHA-256
`a60219448e4b669fd5c3520186e93579b2e3fea6692492d483c0845a7fd3c644`.
[Python ABI 28 evidence](python-ffi-compatibility-design.md#abi-28-device-descriptor-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-244: Add Python ABI 27 bounded `Page.requests()` history as an
independently selectable Rust leaf over the existing network-events
composition. Rust retains the newest 100 Requests per Page while separately
preserving up to 256 user-held owners per Page, 4,096 owners globally and 64
MiB of event bytes; retained evictions keep `Request.response()` and
`Response.text()` functional. The unchanged async/sync Chromium rows pass 2/2
candidate and 2/2 independent oracle, including 199 concurrent retained-owner
reads. Final wheel payloads are reproducible, the installed wheel passes 25/25
local tests, and collection is 6,526/6,526. Current ledgers are candidate
436/5/6085, SHA-256
`2fb546a564e528dfc3788466aae16bb6fc556163e950eff2f26df2bd90e4042e`,
and oracle 417/5/6104, SHA-256
`9ba7a572ab3c66a18b0c409ba09ac4b32c23db36f84d9c73c3be6db7b6040a4c`.
[Python ABI 27 evidence](python-ffi-compatibility-design.md#abi-27-request-history-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-243: Add Python ABI 26 bounded `Page.console_messages()` history as an
independently selectable Rust leaf. Rust retains the newest 200 messages per
Page across 16 Page slots, 3,200 items and 16 MiB, without selecting console
callback publication. Unchanged async/sync Chromium rows pass 2/2 candidate
and 2/2 independent oracle. Final wheel payloads are reproducible, the
installed wheel passes 23/23 local tests, and collection is 6,526/6,526.
Current ledgers are candidate 434/5/6087, SHA-256
`9445e7a0c6d424cee6b604a8fc43804ddb4304ab4d97f1eac7371e3398e4d41e`,
and oracle 415/5/6106, SHA-256
`354fadad72ee68e599644973aa648e941c4619499f3b0a12b30049441a080773`.
[Python ABI 26 evidence](python-ffi-compatibility-design.md#abi-26-console-message-history-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-242: Add Python ABI 25 bounded `Page.page_errors()` history to the
independently selectable Rust Page-error leaf. Rust retains at most 200 errors
per Page across 16 Page slots, 3,200 items and 16 MiB, with exact teardown and
a 512-entry selected lifecycle queue. Unchanged async/sync Chromium rows pass
2/2 candidate and 2/2 independent oracle. Final wheels have reproducible
payloads, the installed wheel passes 22/22 local tests, and collection is
6,526/6,526. Current ledgers are candidate 432/5/6089, SHA-256
`6c3d2dad4271da574f924a305cadcb8aeb801e4650079249e524df02be0e1293`,
and oracle 413/5/6108, SHA-256
`59aad3d302b5c05d06a3a3fe8898ee881076658ab8b0bb9edc5de2ef91e5e00d`.
[Python ABI 25 evidence](python-ffi-compatibility-design.md#abi-25-page-error-history-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-241: Add Python ABI 24 over the existing independently selectable Rust
`request-gc` leaf. Async and sync facades forward only Page identity; Rust owns
session lookup, command admission and exact `HeapProfiler.collectGarbage`.
Pinned unchanged async and sync Chromium cases pass 2/2 in the final-wheel
candidate and 2/2 in the independent oracle. Two final wheels have reproducible
payloads, the installed wheel passes 21/21 local tests, and full collection is
6,526/6,526. The ABI 24 checkpoint ledgers were candidate 430/5/6091, SHA-256
`fce03db46415b6bed7e1d97484aec3abbcbfa9b3fa051a52d1ef73eccf659241`,
and oracle 411/5/6110, SHA-256
`d0bf59a7f917f8c19d6be9064ebc41e295825d8825459e4fda29e7b717f3fb71`.
[Python ABI 24 evidence](python-ffi-compatibility-design.md#abi-24-page-request-gc-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-240: Retain bounded terminal receipts for Page-route calls retired with
their Page, then execute the complete 125-case official async Chromium Page
shard in one-process-per-case candidate and independent-oracle lanes. Both
lanes record 124 passes and the same upstream FIXME skip with zero status
differences; report SHA-256 values are
`709000c785522a979b7f56b344216a9ed0a4f8b9d783ffc725dea8e15c9ba9f3`
and `d8058d713c4cc1fcefbbab295b4eb9aa5647dc98eb9f8453a8e877319be7dade`.
The rebuilt 6,526-row ledgers are candidate 428/5/6093, SHA-256
`a8b7ce8ed00ed75676cad32d68c6487685839b8fae29ba8f1b79e5cbd0689980`,
and oracle 409/5/6112, SHA-256
`c0858c03e07687bd7786d5f713b548fcbc763b9028b41af50579a5e3384c7755`.
Two repaired wheels have reproducible payloads; the installed wheel passes
20/20 local tests and recollects 6,526/6,526. This is complete Page-file
evidence, not complete Python or Core JS/TS execution.
[Python full-Page evidence](python-official-conformance.md#full-page-isolated-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-239: Add resumable per-expanded-case pytest process isolation to the
official Python runner. Selection comes from the pinned 6,526-row inventory;
every subprocess has a pytest signal timeout plus an outer deadline, exact
JUnit identity validation, deterministic aggregation and a checkpointed case
report. Missing, malformed, crashed or outer-deadline executions produce an
explicit row. A historical 15-second route-close hang followed by an unchanged
passing title case proves failure isolation; the two-row aggregate SHA-256 is
`4899c51e6a63307f5ff89884869b50ab054437264c6e64fd2b273568c984c885`.
This runner result is diagnostic infrastructure evidence and is not merged into
the product ledger; NODE-240 fixes the product behavior and reruns the complete
Page file.
[Python isolated-runner evidence](python-official-conformance.md#full-no-weakened-assertion-runner).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-238: Add Python ABI 23 as a separately selectable Rust-owned headless
`Page.pause()` timeout guard. Rust snapshots and temporarily zeros Page/Context
operation and navigation timeouts, then restores the exact prior values on
completion, cancellation or drop; headed Inspector pause remains explicitly
unsupported. The two pinned unchanged-upstream functions pass 2/2 in both the
repaired-wheel candidate and independent official oracle on the same
task-owned headless Chrome. Exact replacement produces a 124-pass/one-skip
Page report. That initial merged checkpoint is superseded by NODE-240's fresh
full-file candidate and oracle executions. The remaining 6,093 candidate rows
are still unexecuted.
[Python ABI 23 evidence](python-ffi-compatibility-design.md#abi-23-headless-page-pause-checkpoint-2026-09-27).
[Python official conformance](python-official-conformance.md).

NODE-237: Add Python ABI 22 as a separately selectable Rust-owned
installed-Chromium persistent-Context launch. Rust accepts only empty or
Pwright-marked profiles, selects the installed executable/channel, owns the
ephemeral loopback CDP connection plus process lease, publishes stable default
Context settings before topology, and performs bounded graceful/forced
shutdown. The pinned unchanged-upstream base-URL row passes 1/1 in both the
repaired-wheel candidate and independent official oracle with symmetric
`channel=chrome`. The 6,526-record ledgers are candidate 426 passed / two
failed / five skipped / 6,093 not-run, SHA-256
`4481a1b424e997bbe258493b8075e2d7e7f776e4c4fb9a61739a9884f26bd076`,
and oracle 343 passed / four skipped / 6,179 not-run, SHA-256
`d250b8d62dc4e699741b1a930de51279e38e291de6657c8bade46fa05d360dfc`.
Two repaired ABI 22 wheels have reproducible payloads and archive SHA-256
values
`449735fcd3e26786396a2bb101483ec57f4533408dc2b96aa0b25e819e9ca7c7`
and `8acfaa118c5d633198161a12a23d71c143e1390659ae5fcf400d640cdd001743`.
The installed wheel passes 19/19 local tests and recollects 6,526/6,526. Only
the two debugger-only pause rows remain failed; full-corpus execution is still
incomplete.
[Python ABI 22 evidence](python-ffi-compatibility-design.md#abi-22-persistent-chromium-context-checkpoint-2026-09-27).
[Python official conformance](python-official-conformance.md).

NODE-236: Close pending Page-event waiters when the Rust-owned Page lifecycle
publishes its terminal close. The thin Python event layer now rejects every
non-close waiter with Playwright's target-closed error, caps waiter identities
at 64 per Page and exposes the `expect_download` terminal path without claiming
successful download events or artifacts. The fixed unchanged-upstream Chromium
row passes 1/1 in both final-wheel candidate and independent oracle lanes.
Exact replacement produces a 125-case Page report with 122 passed / two failed
/ one upstream skip. The 6,526-record ledgers are candidate 425 passed / three
failed / five skipped / 6,093 not-run, SHA-256
`b026f46fa2a4934023b788545ddd769c1cead87e767aea8339a2dd810a98a7e7`,
and oracle 342 passed / four skipped / 6,180 not-run, SHA-256
`4d6186441568d6d6ca19a51e58c36f1439177a2e78a44619d9983d3479581953`.
Two repaired ABI 21 wheels have reproducible payloads and archive SHA-256
values
`015f986890fe67d01c70401206961d623fafde3dcf587d70a4938833d16ed216`
and `f2a99efc425981fa515fa2e6e51e92f9fcd1c935a5b9bbb17567ed260346a6f9`.
The installed wheel passes 19/19 local tests, recollects 6,526/6,526 and reruns
the route-fulfill row 1/1. Three candidate failures remain; full-corpus
execution is incomplete.
[Python ABI 21 evidence](python-ffi-compatibility-design.md#abi-21-page-close-waiter-extension-2026-09-27).
[Python official conformance](python-official-conformance.md).

NODE-235: Add Python ABI 21 route fulfillment and exact fulfilled-response
body ownership to the separately selectable Rust `page-routes` plus
`network-events` composition. Rust resolves relative string patterns against
the Context base URL, validates and bounds status/headers/body, dispatches
`Fetch.fulfillRequest`, and correlates at most eight response bodies by exact
CDP session/request identity. Python retains only public wrappers and rejects
uninstalled JSON/path/response fulfillment. The fixed unchanged-upstream
Chromium row passes 1/1 in both final-wheel candidate and independent oracle
lanes, converting the base-URL file to 6/7. The 6,526-record ledgers are now
candidate 424 passed / four failed / five skipped / 6,093 not-run, SHA-256
`e997512cfb5be3986ab2fad04deaf7c114a2da4b9db675f97243f442fb11357b`,
and oracle 341 passed / four skipped / 6,181 not-run, SHA-256
`eb9ffdee9f86cce60cc115c2c0ca60ee4f299d588665d24893856e3036373deb`.
Two repaired wheels have reproducible payloads and archive SHA-256 values
`88015c44fe0967b531ee3d8ca01d9b8537c07088c7bfb3ec3edd6aecc74e71ea`
and `751c905bcf7c4d1fedfee569f9ce2e795da2d342b62887625aae78f98f360af0`.
The installed wheel passes 17/17 local tests and recollects 6,526/6,526. A
teardown diagnostic exposed and fixed a retained task-local route-controller
Arc before canonical evidence was recorded. Four candidate failures remain;
full-corpus execution is incomplete.
[Python ABI 21 evidence](python-ffi-compatibility-design.md#abi-21-route-fulfill-and-response-body-checkpoint-2026-09-27).
[Python official conformance](python-official-conformance.md).

NODE-234: Add Python ABI 20 as a separately selectable Rust-owned
`page-dialogs` leaf. Rust observes ordered dialog openings, maps observer
sessions back to portable Page ownership, retains the exact emitting CDP
session for single-use accept/dismiss, suppresses duplicate observer events,
and bounds owner count and all dialog strings. Python retains public wrapper
and callback identity only. The fixed 11-case unchanged-upstream manifest
passes 10/10 async plus 1/1 sync in the final-wheel candidate and independent
oracle lanes. Exact replacement converts two beforeunload failures to passes
and produces a 125-case Page report with 121 passed / three failed / one
upstream skip. The complete ledgers are candidate 423 passed / five failed /
five skipped / 6,093 not-run, SHA-256
`d139b5697935e328a9a6d3b1a2a3a9b0181c3a07139872fe875630d1b0a0bb3c`,
and oracle 340 passed / four skipped / 6,182 not-run, SHA-256
`2f2543db5a58b082d70a305928138d03136860b0c6b2f35582f31278d59fb817`.
The dependency proof adds no Pwright package beyond `page-events`; final
repaired-wheel payloads are reproducible and archive SHA-256 values are
`4c057a55597213a7842caa670c27bd38643811df30e6282a7a8229d3509b9d27`
and `68bf970bb1ed3480ccc66c1b6a116c7fe2367fe48318c28edd85da22ad847c04`.
The installed wheel passes 15/15 local tests and recollects 6,526/6,526.
Firefox/WebKit dialog support and full-corpus completion remain open.
[Python ABI 20 evidence](python-ffi-compatibility-design.md#abi-20-page-dialog-checkpoint-2026-09-27).
[Python official conformance](python-official-conformance.md).

NODE-233: Add Python ABI 19 as a separately selectable Rust-owned
`drag-and-drop` leaf. Page and Locator facades normalize public options only;
Rust validates same-Page/same-Frame ownership, resolves and scrolls both
locators, computes bounded viewport points and performs the retained
mouse/HTML5-drag sequence. The fixed eight-case unchanged-upstream manifest
passes 6/6 async plus 2/2 sync in the final-wheel candidate and independent
oracle lanes. Exact replacement produces a 125-case Page report with 119
passed / five failed / one upstream skip. The complete ledgers are candidate
412 passed / seven failed / five skipped / 6,102 not-run, SHA-256
`0f3ac26e7edb0d83c11a6e56e28a3adfcfbcb022e9976ea44796111dce9713ee`,
and oracle 329 passed / four skipped / 6,193 not-run, SHA-256
`66a85bab5620c6611f3cc4503443f0a0a6961738951e3c1500003be32e018cc2`.
The dependency proof shows one added Pwright package,
`pwright-backend-cdp-drag`; final repaired-wheel payloads are reproducible and
archive SHA-256 values are
`0766475c32dbc5f86128c57347c4311caf505a069d012699ada916b3d035255f`
and `99a5ca07d5e9e0e628a9fceac1c2db2743b289926f007f915bc4c80723ddca3c`.
The installed wheel passes 15/15 local tests and recollects 6,526/6,526. Full
actionability/hit-target parity, force mode, other browsers and full-corpus
completion remain open.
[Python ABI 19 evidence](python-ffi-compatibility-design.md#abi-19-drag-and-drop-checkpoint-2026-09-27).
[Python official conformance](python-official-conformance.md).

NODE-232: Add Python ABI 18 `Request.sizes()` over the existing optional Rust
network-events leaf. Rust owns at most 64 pending size owners, decodes bounded
CDP `postDataEntries`, correlates bounded `requestWillBeSentExtraInfo` raw
headers, suppresses Chromium favicon traffic from the public Page stream and
ignores attach-boundary response/terminal tails with no observed start. Python
only awaits an opaque owner and maps the five integer fields. The fixed two-case
unchanged-upstream manifest passes 2/2 in the final-wheel candidate lane and
2/2 in the independent oracle. Exact replacement produces a 125-case Page
report with 115 passed / 9 failed / one upstream skip. The complete ledgers are
candidate 404 passed / 11 failed / five skipped / 6,106 not-run, SHA-256
`0fd2ac2ba00ed0526deb10238cfd9be7a4c0e159c8522d230e2a796befc23c63`,
and oracle 321 passed / four skipped / 6,201 not-run, SHA-256
`31773fcf24b25fc5a4ec7dad75db083e514b20ec38dcc9362167b54ac7a8315c`.
Final repaired-wheel payloads are reproducible; archive SHA-256 values are
`ac314f048915df9f7f8a2303a4d88eec8c353d29b0821b961e5b241e9e4db0a9`
and `2bb81c1e7f96e0d32f4f4ce346075ab4c125be481aaa175971ad794e2d5954b1`.
The installed first wheel passes 13/13 local tests and recollects 6,526/6,526;
full-corpus completion remains open.
[Python ABI 18 evidence](python-ffi-compatibility-design.md#abi-18-request-sizes-checkpoint-2026-09-27).
[Python official conformance](python-official-conformance.md).

NODE-231: Add Python ABI 17 and the separately selectable Rust `page-routes`
leaf. Rust owns bounded `Fetch` interception, registration and paused-call
capacity, glob/path selection, continue/abort commands and terminal Page-close
receipts; Python retains callable and public wrapper identity only. The fixed
five-case unchanged-upstream async Chromium manifest passes 5/5 in the final
candidate wheel and 5/5 in the independent oracle. Exact replacement produces
a 125-case Page report with 113 passed / 11 failed / one upstream skip. The
complete ledgers are candidate 402 passed / 13 failed / five skipped / 6,106
not-run, SHA-256
`918c3da79ac696ef05a504693d3b5a09cab43e97b6ceab82fa29438dab3e3fd8`,
and oracle 319 passed / four skipped / 6,203 not-run, SHA-256
`882eb96a2267c98589fb6811d972f67f1d0d814f51e0945e3ef6c778d8436036`.
Final repaired-wheel payloads are reproducible; archive SHA-256 values are
`440691bb254993669babecc422be9eac6d1cdbec3d91d7b314524c967fa3289c`
and `4527aee3a74b4c3f8b865500dc1f90e4365df36325f28501db0b5c536943a6fa`.
The installed first wheel passes 13/13 local tests and all strict gates pass.
Only async Page string patterns with continue/abort are claimed; regex and
predicate patterns, `times`, overrides, fulfill/fallback/unroute, Context and
sync routing remain open. Full-corpus completion remains open.
[Python ABI 17 evidence](python-ffi-compatibility-design.md#abi-17-page-request-routing-checkpoint-2026-09-27).
[Python official conformance](python-official-conformance.md).

NODE-230: Add Python ABI 16 and the independently selectable Rust
`page-errors` leaf. Native `Runtime.exceptionThrown` values are bounded and
published through the existing ordered lifecycle fence; Python only constructs
the public Error wrapper after commit. The fixed unchanged-upstream manifest
passes 5/5 in candidate and independent oracle lanes. The canonical Page report
is 108 passed / 16 failed / one upstream skip. Current ledgers are candidate
397 passed / 18 failed / five skipped / 6,106 not-run and oracle 314 passed /
four skipped / 6,208 not-run. Two final-source repaired wheels have
reproducible payloads and archive SHA-256 values
`aab4fcd451491ff9199c43c064b5c09c94ab60fd1f656f9cb4fdaf2ba569f8e4`
and `65d3eb6ca1cd0553c384c1165d19806181eade685900920a3ba288b2fd2d710d`;
the installed first wheel passes 13/13 local tests and all strict gates pass.
Full-corpus completion remains open.
[Python ABI 16 evidence](python-ffi-compatibility-design.md#abi-16-page-error-checkpoint-2026-09-27).
[Python official conformance](python-official-conformance.md).

NODE-229: Select Python ABI 15 over the existing bounded Rust Page-lifecycle
publication leaf. Python only schedules already-committed native notices and
maintains public wrapper/callback identity; Rust owns observation, ordering,
capacity, close confirmation and the combined close/network retirement path.
The fixed unchanged-upstream lifecycle manifest passes 6/6 in both candidate
and independent oracle lanes. The canonical Page report is now 103 passed / 21
failed / one upstream skip. Current 6,526-record ledgers are candidate 392
passed / 23 failed / five skipped / 6,106 not-run and oracle 309 passed / four
skipped / 6,213 not-run. Full-corpus completion remains open.
Two final-source repaired wheels have reproducible payloads; archive SHA-256
values are `b9fae36315280fbc80da0178d738fdca9485c1baa6263fe6c74bc557481a98f8`
and `e1c1fc914fdad0fb1af78dfc9364056d8b72c2d87012426c1a4c938cbba259c6`.
The installed first wheel passes 13/13 local tests.
[Python ABI 15 evidence](python-ffi-compatibility-design.md#abi-15-page-lifecycle-event-checkpoint-2026-09-27).
[Python official conformance](python-official-conformance.md).

NODE-228: Add Python ABI 14 Page bindings for `expose_function` and
`expose_binding`, selecting the existing Rust-owned bounded registration,
publication and cleanup path. The fixed 12-case unchanged-upstream manifest
passes 12/12 in both candidate and oracle lanes. This historical checkpoint
was candidate 386 passed / 29 failed / five skipped / 6,106 not-run and oracle
303 passed / four skipped / 6,219 not-run; ABI 15 supersedes it.
[Python ABI 14 evidence](python-ffi-compatibility-design.md#abi-14-page-binding-checkpoint-2026-09-27).

NODE-227: Select Python ABI 13 over the bounded Rust Handle and
wait-for-function leaves, then compose Page script/style tags from those
owners. Fixed unchanged-upstream manifests pass 18/18 tag and 7/7 JSHandle
nodes in both candidate and independent oracle lanes. Current Python ledgers
are candidate 374 passed / 41 failed / five skipped / 6,106 not-run and oracle
291 passed / four skipped / 6,231 not-run. Final-source repaired wheel payloads
are reproducible; local Python passes 13/13 and Rust wait-for-function passes
6/6. Full-corpus completion remains open.
[Python ABI 13 evidence](python-ffi-compatibility-design.md#abi-13-handle-wait-for-function-and-tag-checkpoint-2026-09-27).
[Python official conformance](python-official-conformance.md).

NODE-226: Add Python ABI 12 and complete the pinned async/sync init-script
files. The shared Rust leaf now owns explicit Page and BrowserContext scopes,
current-session fanout, future Page/OOPIF initialization and bounded rollback;
Python and Node remain thin source-normalization ports. Candidate and oracle
each pass 18/18 unchanged upstream invocations. At that checkpoint, the Python
ledgers were candidate 349 passed / 59 failed / five skipped / 6,113 not-run
and oracle 266 passed / four skipped / 6,256 not-run. Two repaired wheels have
reproducible
payloads, the installed wheel passes 13/13 local tests, and Node passes
269/269. The Rust host also passes strict all-feature Clippy after its static
file-input diagnostic projection was made exhaustive for `ActionTimeout` and
`FrameDetached`.
[Python FFI design](python-ffi-compatibility-design.md).
[Python official conformance](python-official-conformance.md).

NODE-225: Align three Python Page-fill discrepancies with the official oracle.
The complete unchanged async Page file improves from 64/60/1 to 67 passed / 57
failed / one upstream skip, and the corrected nodes pass 3/3 in the independent
oracle lane. At this checkpoint, Python ledgers were candidate 331 passed / 59
failed / five skipped / 6,131 not-run and oracle 248 passed / four skipped /
6,274 not-run.
The official oracle launch adapter uses official `connect_over_cdp`; a full
diagnostic that blocks at debugger-only `page.pause()` is excluded rather than
weakening the test. Final repaired wheels remain payload-reproducible and the
local suite passes 12/12.
[Python FFI design](python-ffi-compatibility-design.md).
[Python official conformance](python-official-conformance.md).

NODE-224: Select the client-certificate request leaf through Python ABI 11.
The fixed official async/sync manifest expands to 12 invocations per lane:
eight pass and four retain the upstream macOS WebKit skips in both candidate
and oracle. The Python ledgers are now candidate 328 passed / 62 failed / five
skipped / 6,131 not-run and oracle 245 passed / four skipped / 6,277 not-run.
The repaired abi3 wheel vendors OpenSSL, reproduces across two builds after
normalizing only SBOM identity fields, passed 11/11 local tests at this
checkpoint and recollects
the complete 6,526-case corpus. Browser-context certificate rows, proxy and
the remaining full corpus remain open.
[Python FFI design](python-ffi-compatibility-design.md).
[Python official conformance](python-official-conformance.md).

NODE-223: Add an optional Node `api-request-client-certificates` profile backed
by a bounded Rust PEM/PFX identity leaf and a lazy exact-origin Rustls provider.
The exact official validation row passes 3/3 and seven direct-request behaviors
pass 21/21 in both candidate and independent oracle lanes. The shared official
fixture runtime binds TCP/TLS servers to the verified Tailscale IP and uses full
MagicDNS without editing the pinned upstream checkout. The candidate ledger is
1,214 expected / one skipped / 31 unexpected / 47,231 not-run, SHA-256
`b9d7b48117f08def6811425eca735414ba803c506d6fa39cdb28ebf2c2e60c92`;
the oracle ledger is 884 expected / 47,593 not-run, SHA-256
`fcb2883c54d3b6b0df0f6344016947c3d8d4ae00b528e831d118e246b3aa4676`.
The nine request crates pass 96/96 release tests, Node passes 268/268, strict
types/Clippy and the exact dependency proof pass, and a real Tailscale-bound
PEM/PFX mutual-TLS smoke passes. Proxy, browser-context and same-origin
multi-identity behavior remain open; full conformance is incomplete.
[Canonical API request design](api-request-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-222: Select the browser-independent Rust API-request stack through Python
ABI 10. Thin async/sync facades and PyO3 conversions pass the complete unchanged
Python global-fetch matrices: 231/231 candidate and 138/138 plus 93/93 oracle.
The candidate ledger is now 320 passed / 62 failed / one skipped / 6,143 not
run; the oracle ledger is 237 passed / 6,289 not run after six stop/cancel
lifecycle rows also pass in both modes. BrowserContext
sharing, proxy, client certificates and the complete corpora remain open. The
rebuilt wheel collects 6,526/6,526, passes strict Python 3.9 typing, and its
dependency proof keeps the base at 108 Pwright packages with an exact
eight-package request delta. A real CPython 3.9.6 ABI3 build also passes 8/8
unit tests after removing the last runtime-evaluated PEP 604 cast.

NODE-221: Select the browser-independent Rust API-request stack through the
optional Node `api-request` profile. The facade exposes seven verbs and
Playwright-shaped responses while Rust owns HTTP, TLS, retries, redirects,
Basic auth, cookies, storage-state cookies/origins, cancellation and bounded
request-scoped resolver output. Complete unchanged `global-fetch.spec.ts` and
`global-fetch-cookie.spec.ts` runs pass candidate/oracle 198/198 + 60/60 in
Library and 396/396 + 120/120 in Bidi. At this checkpoint the cumulative Core
candidate ledger was 1,193 expected / one skipped / 31 unexpected / 47,252
not-run and the oracle ledger was 863 expected / 47,614 not-run. API-request
Rust tests passed 91/91 and Node package tests passed 266/266, with strict types,
targeted `-D warnings` Clippy and a real native seven-verb smoke pass. Full
Core/Python conformance remains incomplete.
[Canonical API request design](api-request-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-220: Replace serial TCP address attempts with the pinned Playwright 1.58.0
Happy Eyeballs policy. The Tokio leaf keeps the first DNS result's family
priority, alternates IPv6/IPv4, launches one of at most 64 attempts every
300 ms, returns the first success and drops every loser on success or
cancellation. IP literals bypass DNS. The eight API-request crates pass 88
release tests, doctests, rustdoc and all/pedantic strict Clippy. Public profiles
and official ledgers remain unchanged.
[Canonical API request design](api-request-design.md).

NODE-219: Add a header-first event API to the bounded HTTP/1 codec. Transport
stores Set-Cookie at the final head, then drops the connection, decoder and
fixed 32 KiB buffer without consuming a followed redirect or handled Basic 401
body. Terminal returned responses still read, decompress and enforce their
configured body cap. The eight API-request crates pass 83 release tests,
doctests, rustdoc and all/pedantic strict Clippy. No public binding, official
ledger count or hash changed.
[Canonical API request design](api-request-design.md).

NODE-218: Add optional `pwright-api-request-auth-basic` behind the base
authorization-provider port. Rust now owns bounded credential encoding, exact
case-insensitive origin policy, challenge-only/default and preemptive/always
modes, explicit-header precedence and the single Basic retry. The eight
API-request crates pass 78 release tests, rustdoc and all/pedantic strict
Clippy. Proxy/client certificates, header-first challenges and public bindings
remain open; no official ledger count or hash changed.
[Canonical API request design](api-request-design.md).

NODE-217: Add optional `pwright-api-request-cookie` and keep it behind the
base `ApiCookieStore` port. Rust now owns bounded global cookie import/export,
Set-Cookie parsing, domain/path/secure/expiry matching, redirect recomposition
and explicit caller Cookie precedence. The seven API-request crates pass 66
release tests, rustdoc and all/pedantic strict Clippy. BrowserContext sharing,
origin storage state, header-first redirects and public bindings remain open;
no official ledger count or hash changed.
[Canonical API request design](api-request-design.md).

NODE-216: Add separate Tokio TCP, Rustls HTTP/HTTPS and standard response-body
decompression leaves. DNS answers cap at 64; every socket/TLS operation is
cancellable; Rustls selects verified WebPKI roots or request-scoped
`ignoreHTTPSErrors` and only advertises HTTP/1.1; gzip/x-gzip, deflate and
Brotli decode under the public output cap. A self-signed in-memory TLS handshake
and all codec round trips pass. The six API-request crates now pass 53 release
tests, rustdoc and all/pedantic strict Clippy. Happy Eyeballs, proxy,
credentials/client certificates, cookies and public bindings remain open; no
official ledger changes.
[Canonical API request design](api-request-design.md).

NODE-215: Add optional `pwright-api-request-transport`, which drives the raw
HTTP/1 codec exclusively through injected connection ports. Rust now owns
serial redirect budgets and pinned method/header rewriting, reset-only
exponential retries, per-operation cancellation, a fixed 32 KiB read buffer
and bounded non-fatal log truncation. Eight release tests, rustdoc and
all/pedantic strict Clippy pass. No concrete DNS/socket/TLS/proxy/decompressor
or public language path exists yet, and official ledgers remain unchanged.
[Canonical API request design](api-request-design.md).

NODE-214: Add optional `pwright-api-request-http1`, a runtime-free incremental
HTTP/1 codec that preserves raw response-header case, duplicate order and
Latin-1 values required by official `headersArray()` behavior. It bounds
request heads, cumulative informational/final/trailer bytes, response bodies
and chunk-size lines; handles fixed, chunked, HEAD/204, interim and EOF-framed
responses; and rejects trailing bytes after one owned exchange. The base and
codec crates pass 17 + 11 release tests, rustdoc and all/pedantic strict
Clippy. This is not a socket/TLS transport or an official candidate result.
[Canonical API request design](api-request-design.md).

NODE-213: Move Playwright-shaped request construction into the optional Rust
API-request owner. Rust now resolves encoded/repeated query parameters, merges
default/context/request headers case-insensitively, serializes bounded raw,
JSON, repeated form and ordered multipart bodies, infers file MIME types and
sets exact content headers. Deterministic context-owned multipart boundaries
avoid language-runtime randomness and reject collisions after four scans.
Sixteen release tests and strict Clippy pass. This still adds no HTTP backend,
cookie/storage adapter, language binding or official result.
[Canonical API request design](api-request-design.md).

NODE-212: Add the browser-independent `pwright-api-request` ownership
foundation behind an injected transport port. Record exact Playwright 1.58.0
timeout/status/disposal semantics, independent selection, request/response
bounds, cancellation and deterministic body/log reclamation. Eleven release
tests and strict Clippy pass. No concrete HTTP backend, cookie/storage adapter,
Node/Python binding or official candidate result exists yet, so the public
capability remains planned and full Core/Python conformance remains incomplete.
[Canonical API request design](api-request-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).
[Dependency topology](knowledge/pwright-dependency-topology.md).
[Resource boundaries](resource-boundaries-v2.md).

NODE-211: Add Rust-owned terminal Locator expectation observations for the
official boolean matcher diagnostics. Rust keeps the last valid received state
and, on the first mismatch while the original deadline still has time, obtains
one immediate count plus one bounded static element-preview evaluation. It
returns explicit preview/missing fields through the addon; Node only formats
the official call-log text and never receives a DOM handle or performs a
browser probe. The diagnostic attempt cannot renew or run after the public
deadline and adds no persistent owner. Pass package 257/257, focused Rust
expectation 12/12, strict TypeScript, selected native-load and selected-profile
Clippy. Browser confirmation and ledger changes remain intentionally pending:
the empty-browser safety gate correctly blocks while the restored unrelated
page remains open, so the previous 62/24 candidate report is unchanged and is
not final isolation evidence. Full Core and Python conformance remains
incomplete.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).
[Dependency topology](knowledge/pwright-dependency-topology.md).
[Resource boundaries](resource-boundaries-v2.md).

NODE-210: Establish an independent official Core oracle lane on the same
dedicated CDP Chrome and run the complete unchanged
`page/expect-boolean.spec.ts` file in both lanes. The oracle is 86/86. The
candidate progresses from 37/49 to 62/24 after checked, indeterminate,
editable/readonly, enabled/disabled and focused semantics move into existing
Rust read leaves or Rust-selected browser programs; Node only validates public
option shapes. Record the independent oracle ledger at 86/48,391 and the
candidate ledger at 416/1/31/48,029 with exact report and ledger hashes. The
first candidate run also exposed that a pre-existing nonblank dedicated-Chrome
page could be retired by fixture cleanup; restore that page and make the
official runner refuse every later runtime run while any nonblank page exists.
Keep diagnostics, 1 ms initial probes, FrameLocator cleanup, CSS error wording
and request/toBeOK support open. Full Core and Python conformance remains
incomplete.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).
[Dependency topology](knowledge/pwright-dependency-topology.md).
[Resource boundaries](resource-boundaries-v2.md).

NODE-209: Preserve the original runtime deadline while projecting the last
bounded Rust-owned click actionability state only after deadline cancellation
and browser cleanup succeed. The locator route owns one fixed progress slot;
it clears that slot at call start, on ready/runtime-replaced state and normal
completion, and consumes it once for terminal timeout formatting. The runtime
hook is synchronous, default-noop and forbids I/O, a restarted operation or a
second deadline. Node maps only fixed diagnostic keys to Playwright wording;
it adds no DOM probe, timer, retry loop or browser owner. Pass the complete
unchanged `page-click-timeout-2.spec.ts` file 2/2 and progress the controlled
12-case Page batch from 8/4 to 9/3. Record 256/256 package, runtime 114/114,
Locator backend 115/115, Chromium composition 397/16, selected host 617/14 and
22/8 host integration gates, plus the 354/1/7/48,115 Core ledger and exact
report hashes. Keep client-redirect continuation, dialog events and the private
`__testHookBeforePointerAction` case explicit and open. Full upstream
conformance remains incomplete.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).
[Dependency topology](knowledge/pwright-dependency-topology.md).
[Resource boundaries](resource-boundaries-v2.md).

NODE-208: Add independently selectable Rust-owned Locator bounding-box and
highlight leaves; `locator-highlight` composes the lower-level bounding-box
leaf while Node contributes only the public description, option/result shapes
and error presentation. Keep one bounded overlay/animation-frame state in the
selected Frame utility world, replacing it on repeated highlight calls and
letting world teardown clear it on navigation. Match Playwright's open shadow
root only under the official test environment and use a closed root otherwise.
Pass the complete unchanged `locator-highlight.spec.ts` file 1/1 and progress
the controlled 12-case Page batch from 7/5 to 8/4. Record 256/256 package,
strict type/native-load/selected-profile Clippy, 616/14 host library and 22/8
host integration gates, plus the 351/1/10/48,115 Core ledger and exact report
hashes. Keep client-redirect continuation, dialog events, one private pointer
hook and rich actionability logs explicit and open.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).
[Dependency topology](knowledge/pwright-dependency-topology.md).
[Resource boundaries](resource-boundaries-v2.md).

NODE-207: Add Rust-owned Locator state, property and regular-expression text
expectations while keeping Node limited to public descriptions, bounded value
conversion and Playwright error presentation. Fix missing-element semantics so
neither polarity of a text expectation succeeds merely because the element is
absent. Progress the complete unchanged `matchers.misc.spec.ts` file from 1/2
to 2/2 and the controlled 12-case Page batch from 5/7 to 7/5. Record 254/254
package, strict type/native-load/selected-profile Clippy, 614/14 host library
and 22/8 host integration gates, plus the 350/1/11/48,115 Core ledger and exact
report hashes. Keep redirect continuation, Locator highlight, dialog events,
one private pointer hook and rich actionability logs explicit and open.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).
[Dependency topology](knowledge/pwright-dependency-topology.md).
[Resource boundaries](resource-boundaries-v2.md).

NODE-206: Add the independently selectable Rust `response-body` leaf and its
thin Node `Response.body/text/json` facade. Preserve the completed document's
bounded native request identity through navigation, resolve the exact Page and
Frame in Rust, issue `Network.getResponseBody`, decode bounded base64 in Rust,
and cache only the copied Buffer in Node. Progress the complete unchanged Frame
goto file from 2/4 to 3/4 and the controlled 12-case Page batch from 4/8 to 5/7.
Record 249/249 package, strict type/native-load/selected-profile Clippy, 609/14
host library and 22/8 host integration gates, plus the 348/1/13/48,115 Core
ledger and exact report hashes. Keep redirect continuation, Locator highlight
and matchers, dialog events, private hooks and rich actionability logs explicit
and open.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).
[Dependency topology](knowledge/pwright-dependency-topology.md).
[Resource boundaries](resource-boundaries-v2.md).

NODE-205: Run a controlled 12-case unchanged upstream Page batch. Move enabled
state retargeting into the shared Rust selector runtime so a text-matched child
of a disabled control cannot be clicked, while preserving original-element
geometry. Preserve the Rust `FrameDetached` message through the host error port
and progress the complete Frame goto file from 1/4 to 2/4. Record the 347/1/14/
48,115 Core ledger and its exact report hashes; keep response bodies, redirect
lifecycle, Locator highlight/matchers, dialog events, private hooks and rich
actionability logs explicit and open.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).
[Dependency topology](knowledge/pwright-dependency-topology.md).
[Resource boundaries](resource-boundaries-v2.md).

NODE-204: Preserve Page/Frame/Locator public API identity while all selector
actions continue to use the same Rust Locator owner, and normalize standard
timeout punctuation. Record the complete Page click-timeout file's 0/2 to 0/2
progression and its next missing Rust-owned actionability progress diagnostic.
Record the private-hook click-during-navigation case as an explicit unexpected
result, not a hidden skip, and the 343/1/6/48,127 merged Core ledger.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).
[Dependency topology](knowledge/pwright-dependency-topology.md).

NODE-203: Progress the complete unchanged Page navigation file through
0/3, 0/3, 0/3, 2/3 to 3/3. Parse quoted Playwright text shorthand and bounded
selector chains into typed Rust recipes; extend the shared selector runtime to
submit/button input-value text semantics; and make paused-popup discovery a
fixed five-owner caller-polled startup batch without adding a task, queue,
browser owner or package edge. Record the 343/1/3/48,130 merged Core ledger.
Record 246/246 package, 606/14 host library, 22/8 host integration, 112/112
selector backend, 397/16 Chromium composition, strict type/native-load/Clippy,
format and diff gates.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).
[Dependency topology](knowledge/pwright-dependency-topology.md).

NODE-202: Add the independently selectable zero-persistent-state Rust
`request-gc` leaf and its thin Node Page facade. Progress the complete
unchanged Page request-GC file from 0/1 to 1/1 by resolving the exact Page's
current Chromium session and sending the official empty
`HeapProfiler.collectGarbage` command. Record 246/246 package, strict
type/selected-profile Clippy/native-load, 605/14 host library, 22/8
integration, and the 340/1/3/48,133 merged Core ledger.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).
[Dependency topology](knowledge/pwright-dependency-topology.md).

NODE-201: Compose the existing bounded Rust URL-wait leaf into the Node
conformance artifact, extend it from main-Frame-only observation to exact
Page-owned child Frame identity, and keep JavaScript string/RegExp/URL
predicate semantics in a bounded matcher callback. Progress the complete
unchanged Page URL-wait file from 0/11 to 11/11. Record 244/244 package,
strict type/selected-profile Clippy/native-load, 604/14 host library, and the
339/1/3/48,134 merged Core ledger.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).
[Dependency topology](knowledge/pwright-dependency-topology.md).

NODE-200: Expose the independent Rust `emulate-media` leaf through a thin Node
option adapter and progress the complete Page media file from 0/11 to 11/11.
Group optional Page N-API methods by whole feature so standalone selections
compile without unrelated exports. Compose the existing `extra-headers` leaf
and pass its complete Page file 5/5. Record 240/240 package, strict
type/Clippy/native-load, 603/14 host library, 22/8 integration gates, and the
328/1/3/48,145 merged Core ledger.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).
[Dependency topology](knowledge/pwright-dependency-topology.md).

NODE-199: Pass the complete unchanged ElementHandle type 5/5 and press 5/5
files with thin Page/Frame facades over typed Rust Locator actions. Progress the
complete Page focus file from 5 pass / one upstream skip / four fail through
8 / 1 / 1 to 9 / 1 / 0. Add a bounded Rust Page-binding publication fence so a
successful action returns only after every admitted binding call through its
topology watermark has invoked its Node callback; never wait for the callback
Promise or browser result delivery. Record 239/239 package, strict
type/Clippy/native-load gates, 568/14 host library tests, 22/7 integration
tests, and the 312/1/3/48,161 merged Core ledger.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-198: Pass the complete unchanged Locator evaluate 6/6, ElementHandle
selector-evaluate 6/6, and Locator visibility 8/8 files. Add a separately
bounded Rust `evaluateAll` collection lane, preserve retained Handle ownership
for `$eval`/`$$eval`, and route `waitFor` plus visibility reads/expectations
through typed Rust operations. Match Playwright's navigation-context recovery
and exact unknown-selector-engine error before CDP. Record 238/238 package,
strict type/Clippy/native-load gates and the selected host release suite. The
existing typed action also passes the complete Locator click file 5/5, producing
the 293/3/48,181 merged Core ledger.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-197: Pass complete unchanged Page evaluation 96/96 and evaluation-Handle
11/11 files with the optional Rust-owned large-result path. Ordinary envelopes
stay in the initial CDP reply; only a selected top-level string above 256 KiB
uses fixed-size projections and one bounded large-transfer lease. Add
result-dependent FFI admission, detached-Frame error adaptation and robust
serialization under overwritten page builtins. Record 223/223 package,
strict type/Clippy/native-load gates and the 233/1/48,243 merged Core ledger.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-192: Add the optional bounded Rust-owned Page-binding transport used by
the full upstream core conformance lane. Exact CDP binding-name/schema filtering,
64-call admission, browser argument serialization, Rust evaluation result
conversion and native Handle ownership now cover value and `{ handle: true }`
callbacks. The unchanged 24-test upstream Page expose file progressed from
21/3 to 23/1 and now passes 24/24 after independent `init-scripts`, receipt-free
Rust navigation wait, Locator-backed Page click and retained-Handle inner-text
facades were composed. Navigation-wait ticket registration is synchronous at
N-API admission, before the returned Future may yield. A focused teardown probe
still reports two retained native leases; cleanup remains an open NODE-192 gate.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-196: Continue complete unchanged upstream files beyond the Page-binding
slice. Move Page console from 9/13 to 13/13 and Page load from 0/2 through 1/2
to 2/2 with a fixed Rust loader/phase deduplicator, lazy FrameLocator composition
and Rust-owned Locator text polling. Move locator-list from 0/1 through a
selector-chain discrepancy to 1/1 with bounded `all` and Rust `Within` recipes.
Record JSHandle evaluate 2/2, asElement 4/4, jsonValue 3/3, and preserve the
Handle-preview/sparse-console discrepancy as 0/6. The merged ledger is
113 expected / 7 unexpected / 48,357 not-run; package/type/Clippy/native-load
gates pass at 219/219.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-195: Pass the complete unchanged 26-case Page wait-for-function file
without treating its dependencies as JavaScript-owned browser logic. Add
Page/Frame `$` over Rust Handle evaluation, a separately selected fixed
16-operation Rust `wait-for-timeout` leaf, and Node exposure of the existing
fixed Rust timeout-inheritance table. Omitted wait-for-function timeouts are
resolved in Rust. Retry replaceable execution-context loss while the Frame is
live and add exact retained-element `contentFrame` lookup without a temporary
Handle. Four complete upstream runs progress 22/26, 24/26, 25/26 and 26/26.
Record 1/1 timer, 2/2 timeout-settings, 6/6 wait-function Rust AAA, 214/214
package, strict type/Clippy, release build and native-load gates.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-193: Record the unchanged Page init-script candidate file progressing from
8/9 to 9/9. Add Page/Frame script-tag as a derived language facade over
existing Rust-owned Frame evaluation and retained ElementHandle publication.
Its complete official file progresses from 9/12 to 10/12 after fixing inline
module publication and CSP rejection. [Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-194: Add the independently selected `wait-for-function` Rust leaf exposed
by the remaining two script-tag failures. It owns fixed eight-operation
resource admission, native deadline/cancellation and the exact truthy Handle;
one-shot Frame attempts dispose falsy Handles and leave no polling Promise after
timeout. Node only adapts overloads and options. Record 2/2 Rust AAA, 203/203 package,
strict TypeScript/Clippy, release build and native-load gates. Subsequent
complete-file evidence reaches script-tag 12/12 under NODE-195/NODE-196.
[Node FFI checkpoint](node-ffi-compatibility-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

PY-009: Add default-off Rust timeout-settings and network-events leaves through
Python ABI 9. Record fixed Context/Page timeout state, bounded caller-polled
network waiters, Python predicate-only adaptation, exact Page/session teardown,
the official 14/14 network subset, the complete 64/60/1 Page shard, and the
83/62/1/6380 cumulative ledger. Full upstream execution remains incomplete.
[Python FFI checkpoint](python-ffi-compatibility-design.md#abi-9-network-events-checkpoint-2026-09-26).
[Official Python ledger](python-official-conformance.md).

NODE-191: Add explicit standalone/combined
`retained-serialized-payload-mutation` profiles. They consume NODE-190's
move-only graph and the retained, already-retargeted INPUT in one Rust-owned
operation, validate the package-produced payload subset, issue exactly one
large utility-world `Runtime.callFunctionOn` with `userGesture`, reproduce the
pinned `File`/`DataTransfer` plus `input` then `change` behavior, settle the
object group and terminally remove the same FFI ticket. There is no selector,
second retarget/adoption, `DOM.setFileInputFiles`, extra N-API root or public
`ElementHandle.setInputFiles`. The completed upstream differential matrix
covers payload/event semantics, late type change, listener exceptions,
cancellation, navigation/detach races and the maximum admitted payload. It
records, rather than hides, the navigation-settlement and `lastModifiedMs`
differences that the later public API must decide. [Canonical contract and
verification](knowledge/pwright-node-handles.md#node-191-terminal-rust-owned-serialized-payload-mutation).

NODE-190: Add explicit standalone/combined
`retained-serialized-payload-import` profiles after NODE-189 serialization.
The host reserves one 96 MiB Rust graph owner before the addon copies any V8
value, and the addon enforces depth/node/entry/string-byte limits while
importing package-owned JSON-compatible data. Success advances `Serialized ->
PayloadImported`, drops all eleven direct V8 roots and retains a move-once Rust
graph on the same preparation. No CDP command, browser mutation, DOM event,
public API or generic live-handle transport is added. [Canonical contract and
verification](knowledge/pwright-node-handles.md#node-190-bounded-rust-ownership-for-the-serialized-payload-graph).

NODE-189: Add the addon-only `retained-call-argument-serialization` leaf/profile
after awaited input instrumentation. Rust preserves exact session/ticket/target
authority while the pinned Node language adapter implements synchronous getter,
`toJSON`, cycle and special-value behavior. The serialized graph becomes an
eleventh root; base64 string leaves are reused and never copied into Rust by
this phase. Generic live-handle adoption, DOM mutation/events, terminal
publication/error transport and the public method remain open. [Canonical
contract and verification](knowledge/pwright-node-handles.md#node-189-late-call-argument-serialization-on-the-retained-owner).

NODE-188: Add the addon-only `retained-input-instrumentation` leaf/profile and
internal `prepareInstrumentedTarget` coordinator. Rust gates exact-session
`Prevalidated -> WaitingInputAction -> Instrumented`; Node awaits a frozen,
default-no-op `onBeforeInputAction` port with the original receiver and retained
call identity under the same deadline. No host/backend/composition feature,
package edge, FFI admission, root or native lease is added. Late serialization,
mutation/publication, error mapping and public `ElementHandle.setInputFiles`
remain open. [Canonical contract and verification](knowledge/pwright-node-handles.md#node-188-awaited-input-action-instrumentation-boundary).

NODE-187: Add an explicit `retained-target-prevalidation` leaf/profile that
consumes the exact move-only marked owner after carrier projection. Route the
existing read-only DOM preflight through the same parked FFI ticket and Node
deadline, retain marker-call/adopted-group ownership inside the new validated
owner, and commit `Projected -> Prevalidated` without release/re-adopt, new JS
roots or a second command. Record independent eight-operation backend/owner/
cleanup budgets, exact dependency fingerprints, the selected seven-lease idle
baseline and final 28/28 real-Chrome results for both artifacts. Instrumentation,
late serialization, mutation, public error mapping and
`ElementHandle.setInputFiles` remain open.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-187-same-owner-retained-dom-prevalidation).

NODE-186: Keep the successful native marking result move-only and parked on the
same FFI ticket instead of releasing it at the marker boundary. Route the
selected post-mark carrier projection, its observable getter/coercion order,
post-callback phase recheck, ten-root replacement and Rust-owned truth flags to
the canonical Handle record. Record unchanged one-command/1 MiB admission,
selected-only root growth from six to ten, the independent eight-call native
owner bound and the persistent marker-cache baseline caveat. Public retained
prevalidation, instrumentation, serialization, mutation and
`ElementHandle.setInputFiles` remain open.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-186-retained-marked-owner-and-same-ticket-carrier-projection).

NODE-185: Route the selected per-session `call@N` source, post-schema/pre-
instrumentation identity transition, post-instrumentation source recheck and one
Node-event-loop deadline to the canonical Handle record. The same progress owns
materialization and marking; Rust starts no clock and marking accepts no JS ID.
Keep the unchanged FFI/registry budgets distinct from the small unbounded-V8
timer objects, preserve the one combined Worker timeout followed by a clean
pass, and record final 26/26 live evidence for both rebuilt artifacts while
retaining earlier overloaded-Chrome failures. Public projection, prevalidation,
instrumentation, serialization, mutation and `ElementHandle.setInputFiles`
remain open.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-185-session-call-identity-and-one-node-deadline).

NODE-184: Route the default-off addon target-marking profile, typed initial FFI
progress, exact running revoker, original-Handle late upgrade, same-ticket park,
publication-before-error ordering and `Materialized` to `Marked` transition to
the canonical Handle record. Update architecture/resources, file-input/lifecycle,
dependency/testing, Node design/tasks and package mirrors. Keep the raw internal
method distinct from absent pre-materialization deadline/call identity, later
native phases and public retained upload. Record 24-case real Chrome evidence,
including cancellation, late disposal and foreign-session ownership, without
turning logical admission into an RSS claim.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-184-same-admission-node-target-marking-handoff).

NODE-183: Route the default-off Rust host target-marker cache, independent exact
Frame utility snapshot, 64-current/eight-retiring/eight-call admission, bounded
diagnostics, topology pruning and completion wakeups to the canonical Handle
record. Update architecture/resources, file-input/lifecycle/Locator evaluation,
dependency/testing, Node design/tasks and package mirrors. Keep Rust assembled
evidence distinct from unchanged addon artifacts, absent typed Node handoff,
deadline/call-ID authority and public retained upload. Preserve initial fixture
failures and final-source results, without treating native Set retention as an
RSS bound or kernel cancellation coverage as assembled pending-work shutdown.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-183-host-owned-utility-world-marker-cache).

NODE-182: Route default-off persistent exact-utility-world target marking,
separate state/call admission, raw awaited CDP settlement, cancellation-safe
group ownership and fail-closed cleanup to the canonical Handle record. Update
backend/composition READMEs, architecture/resources/lifecycle, dependency and CI
selection, Node design/tasks, file-input, package and testing mirrors. Distinguish
offline pinned-policy comparisons from real DOM marker evidence and from the
still-absent host cache, typed Node handoff, deadline/call identity and public
retained upload. Keep native Set retention and single-waiter proof limitations
explicit; Node timer delivery is not interchangeable with eager Rust expiry.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-182-persistent-native-target-marking).

NODE-181: Route inert original-NativeHandle weak binding through all selected
preparation records/snapshots, unchanged 32-entry registry layout and independent
1 MiB/16 KiB transient proofs to the canonical Handle record. Update architecture,
resources, Node design/tasks, file-input/lifecycle, dependency/testing and package
mirrors. Keep actual-ticket disposal/error-order evidence distinct from future
Weak upgrade, persistent utility-world marking, typed native handoff and public
upload. Record watch-aware active baselines/final cleanup observations and
bounded failure diagnostics separately from production behavior. Preserve both
failed sampling runs, final 20-case live passes across three executions,
268 offline checks per artifact, 161 package tests and final artifact hashes.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-181-original-native-wrapper-binding).

NODE-180: Route bounded exact-session weak FFI revocation, source/generation
binding, four-KiB selected registry storage, cancel-first admission barrier,
lock-free owner wakeups and revoke-before-drains teardown to the canonical
Handle record. Update architecture/resources, Node design/tasks, file-input/
lifecycle, dependency/testing and package mirrors. Keep initial language-only
capacity unchanged, and distinguish kernel ownership evidence from absent
actual Node promotion, marking, public upload and assembled/browser proof.
Record final selected/combined host and addon results, rebuilt artifact hashes,
268 offline checks per artifact, five strict Clippy selections and dependency
gates. Distinguish source-reviewed lock-free disconnect on retirement failure
from an unexecuted assembled-session failure-path regression.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-180-session-bound-revocation-before-native-drains).

NODE-179: Route same-admission FFI park/resume, weak revocation, actual-future
destruction ordering and the Rust raw-carrier projection kernel to the canonical
Handle record. Update architecture/resources, Node design/tasks, file-input/
lifecycle, dependency/testing and package mirrors. Keep exact HostSession
revoker registration, persistent utility-world marker ownership, real Node
ticket integration and public retained upload open. Offline projection and
budget tests do not prove browser behavior or native cleanup settlement.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-179-same-admission-ffi-handoff-and-raw-carrier-projection).

NODE-178: Route default-off retained native file-input prevalidation, exact
retargeted receiver/group ownership, one-shot result/exception allowances,
explicit userGesture/raw settlement, cancellation and fail-closed cleanup to
the canonical Handle record. Update file-input backend/composition READMEs,
adoption/lifecycle, architecture/resources, dependency/testing, Node design/tasks
and package records. Keep the 35-case offline oracle distinct from DOM/browser
proof, and host/addon selection, marking/raw carrier projection, late utility
serialization, deadlines/promotion and public retained upload open.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-178-owned-native-file-input-prevalidation).

NODE-177: Route exact owner-ticket/creation-Page registration, two Rust phase
gates around server validation, setImmediate/empty-instrumentation scheduling,
undefined Materialized return and abort ownership to the canonical Handle
checkpoint. Architecture/resources, Node design/tasks, file-input/lifecycle,
dependency/testing and package notes preserve the distinction from
world/Handle/Frame runtime retirement and the unchanged roots/dependencies/
classes/budgets/native pins. Both rebuilt selected artifacts pass twenty new
offline dispatch cases each; 17 live binding cases remain unexecuted. Exact
evidence belongs in the canonical record. Keep target marking, carrier projection,
DOM retarget/checks, onBeforeInputAction, late utility serializer and browser
atob/File ordering explicit; eager PreparedFileInput conversion and public upload
are not completed. No new browser result is claimed.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-177-deferred-registration-gates-before-materialization).

NODE-176: Route server-direction schema, raw payload materialization, exact pinned
MIME extension lookup, outer async assimilation and revocable six-root ticket
phases to the canonical Handle record and linked resource/architecture/Node,
dependency, file-input/lifecycle and testing records. Both rebuilt selected
artifacts pass 76 new offline cases; 12 actual-ticket live cases remain unexecuted. Real dispatcher
authority, error transport, native storage/promotion and public upload stay open.
[Canonical contract](knowledge/pwright-node-handles.md#node-176-server-direction-validation-and-raw-payload-materialization).

NODE-175 (2026-09-17): Route Rust raw-buffer channel schema/order, observable map/valueOf/ordinary assignment, zero-capture escaped callback safety and the ArgumentsReady-to-ChannelReady sixth-root replacement to the canonical Handle record. Record 61 channel cases per rebuilt profile, final artifact fingerprints and regression gates separately from the eight unexecuted live binding/channel cases. Keep valid-stream authority, public error wrapping, deferred materialization/submission and browser gates distinct from offline native-kernel comparisons.
[Canonical channel validation contract](knowledge/pwright-node-handles.md#node-175-rust-owned-raw-buffer-channel-validation).

NODE-174 (2026-09-17): Route canonical session/Page timeout issuance, frozen creation-Page proof, Rust producer/descendant binding, deferred lookup errors, bounded scoped-wrapper admission, cached raw debug inputs and seal-before-root-drop teardown to the canonical Handle record. Record rebuilt standalone/combined artifact fingerprints and completed offline checks separately from the new unexecuted six-case live binding gate; retain public setter/upload/channel gates.
[Canonical creation-Page policy contract](knowledge/pwright-node-handles.md#node-174-canonical-creation-page-timeout-binding).

NODE-173 (2026-09-17): Route Rust post-conversion argument assembly, live typed Context/Page timeout policy, selected metadata admission/class publication and offline argument comparisons to the canonical Handle record. Keep caller-supplied settings distinct from unimplemented production creation-Frame binding, public setters, channel validation, native deadlines and complete upload parity.
[Canonical argument and timeout contract](knowledge/pwright-node-handles.md#node-173-rust-owned-argument-assembly-and-live-timeout-settings).

NODE-172 (2026-09-17): Route Rust preflight policy, exact owner/context/conversion boundaries, narrow language ports, reentry-safe construction/replacement guards, independent synchronous scratch admission, variable root accounting and selected-addon offline kernel comparison to the canonical Handle record. Architecture/resources, Node design/tasks, dependency, file-input/lifecycle, package and testing records retain the missing public upload, native integration and browser gates.
[Canonical preflight contract](knowledge/pwright-node-handles.md#node-172-rust-owned-preflight-and-reentrant-language-phases).

NODE-171 (2026-09-17): Route exact-session Node preparation tickets, weak source metadata, pre-root admission, cleanup-hook/publication revocation, selected artifact profiles and offline Worker checks to the canonical Handle record. Architecture/resources, Node design/tasks, dependency, upload/lifecycle, package and testing records retain the unimplemented public conversion/submission and browser gates. Root counts are not arbitrary V8 graph or retained-tombstone byte bounds.
[Canonical Node preparation contract](knowledge/pwright-node-handles.md#node-171-session-bound-node-preparation-registry).

NODE-170 (2026-09-17): Route revocable FFI preparation, atomic execution promotion, environment-versus-session authority, shutdown/byte admission and additional upstream-only ingress observations to the canonical Handle record. Update architecture/resources, Node design/tasks, dependency/testing and package status. No Node registry, public upload method or addon artifact is implied.
[Canonical preparation contract](knowledge/pwright-node-handles.md#node-170-revocable-ffi-preparation-before-execution).

NODE-169 (2026-09-17): Route the host-only retained-file-input submission, read-only source receipt, atomic tracked-call admission, independent untimed phase cleanup and pinned offline ingress oracle to the canonical Handle record. Architecture/resources, dependency/Node design, upload/adoption/lifecycle/evaluation, package/test and task records preserve the explicit missing public Node integration and browser gates.
[Canonical contract and evidence](knowledge/pwright-node-handles.md#node-169-safe-host-retained-upload-and-pinned-ingress-oracle).

NODE-168 (2026-09-17): Route optional retained-reference group adoption,
unique-world verification before accepting destruction receipts, explicit
attempt scratch/diagnostic budgets, utility numeric snapshot coherence,
numeric route storage and legacy adoption cleanup failures to
[the canonical Handle checkpoint](knowledge/pwright-node-handles.md#node-168-group-owned-retained-adoption-and-deferred-upload-ingress).
Adoption, Locator evaluation, file-input, lifecycle, architecture/resources,
dependency and test records describe the boundaries. Node retained uploads and
the final browser gates remain incomplete.

NODE-167 (2026-09-17): Route current-DOM ElementHandle.ownerFrame, canonical
Frame-cache lookup, tracked read/native admission, raw object-ID description,
cancellation/cleanup and retained-upload preparation design to
[the canonical Handle checkpoint](knowledge/pwright-node-handles.md). The
Handle leaf/evaluation/operations/lifecycle, Node/package, architecture,
resources, dependency, testing and task records describe their boundaries.
Retained upload and full-corpus parity remain open.

NODE-166 (2026-09-17): Route Node getProperties, pre-admitted staged native
batches, per-ObjectId cleanup tickets, raw property settlement, complete
addressable-owner compensation, structural staging bounds, failure-safe native
and JS publication, and confirmed Page-close world receipts to
[the canonical Handle checkpoint](knowledge/pwright-node-handles.md).
Selected cleanup capacity is now 112; scalar/batch calls share the existing
eight-call and 64-result budgets. No package/feature/protocol version is added.
The Handle evaluation/operations/lifecycle, Page-close, architecture/resource,
dependency, Node/package and task/testing records describe their boundaries.
Keep the full corpus and unaddressable-owner design open; historical missing
getProperties notes below describe their original checkpoints.

NODE-165 (2026-09-17): Route explicit raw retained-result CDP acquisition/release
selection, exception-ID preservation, settlement ownership, borrowed two-ID
compensation, bounded diagnostics and cancellation/failure evidence to
[the canonical Handle checkpoint](knowledge/pwright-node-handles.md).
The Handle evaluation/operations/lifecycle records and leaf README describe
the ownership boundary; resource and dependency records retain the unchanged
package/feature/schema/API scope. Exact verification belongs in the canonical
checkpoint, not inferred from older counts. Runtime.getProperties settlement,
full property-pipeline admission, a pre-owned group for unaddressable IDs,
Node batch publication and the full compatibility corpus remain open.

Also route NODE-165's Handle-enabled owned-inspector retirement before Handle
drain, error-preserving detach/disconnect fallback, and conservative
compensation-versus-detach residual to [the canonical shutdown record](knowledge/pwright-node-handles.md#node-165-retained-acquisition-owns-failure-cleanup-and-shutdown-settlement)
and the architecture/resource/lifecycle records. Non-Handle ordering is
unchanged; no shared-browser or borrowed-Page ownership is broadened. Generic
Cleanup and an earlier host snapshot cannot substitute for the actual frozen
route/client identity plus exact native retirement receipt.

NODE-164 (2026-09-17): Route the crate-wide N-API module-local class identity, locked-source validation-order gate, native-class isolation lane, dependency-key-only rename, portable property snapshot/publication repair and abandoned cleanup failure reporting to [the canonical Handle checkpoint](knowledge/pwright-node-handles.md). The priority raw-class audit now has an implementation; exact verification and remaining gates are recorded in that canonical checkpoint. Preserve historical evidence and keep Node getProperties and broader compatibility gaps open.

NODE-163: Route Handle-targeted producers, staged source revalidation, native and JS publication rollback, compile-time future storage bounds, property Promise identity, paired producer evidence and the remaining batch ownership design to knowledge/pwright-node-handles.md and the Handle leaf README.

NODE-162: Route native-only expression programs, unchanged portable 1.2 codecs, indirect global Handle creation, ignored-argument validation/admission, deferred disposed-target errors, handle-value selection and red/green verification to knowledge/pwright-node-handles.md and the Handle leaf README.

NODE-161: Route module-local native-branded owned nested arguments, independent-addon/type-spoofing guards, Frame by-value reference evaluation, conversion/error phases, shared FFI baseline, resource proofs, type unboxing and final artifact verification to knowledge/pwright-node-handles.md and the Handle evaluation leaf README. String-expression reference arguments, adoption and other producer/action families remain unfinished.

NODE-160: Route NODE-160 public opt-in Handle creation, value reads, completion-only disposal, conversion reuse, resource accounting, artifact selection and differential evidence to knowledge/pwright-node-handles.md. The full retained-upload lane and remaining Handle producers are still open.

NODE-159: Route safe-host native-creation selection, eight-entry driver publication, pre-admitted receiver-loss/late-result compensation, typed world-retirement proof, queue/resource budgets and final test evidence to knowledge/pwright-node-handles.md. Public Node Handles and other producer families remain unfinished.

NODE-158: Route opt-in staged native Handle creation, deferred local publication, explicit compensation ownership, exact source-world revalidation, dependency/CI checks and real Chrome single-execution evidence to knowledge/pwright-node-handles.md and knowledge/pwright-handle-evaluation.md. Host admission/queue/recovery and public Node Handle APIs remain unfinished.

NODE-157: Route host-native Handle release scheduling, queue-free concurrent close, source-world retirement witnesses, bounded shutdown draining and red/green browser evidence to knowledge/pwright-node-handles.md. Creation scheduling and public Node Handle parity remain open.

NODE-156: Route optional native Handle release, exact route-owner receipts, pre-I/O commit admission, live Chrome invalidation and dependency/CI evidence to knowledge/pwright-node-handles.md and knowledge/pwright-handle-lifecycle.md. Host scheduling and public Node Handle integration remain pending.

NODE-155: Route queue-free runtime disposal admission, recoverable busy commit boundaries and retained capacity tests to knowledge/pwright-node-handles.md. Native release receipts, host scheduling and public Node Handle publication remain pending.

NODE-154: Route safe-host pending Handle results, revocable cleanup tickets, descriptor-versus-native work lifetimes, explicit admission and native shutdown evidence to knowledge/pwright-node-handles.md. Public N-API Handle producers remain gated.

NODE-153: Route abandoned Handle batch partial-failure cleanup, red/green service regression evidence and still-pending Node delivery ownership to knowledge/pwright-node-handles.md and knowledge/pwright-handle-evaluation.md. No public Node API or artifact changes.

NODE-152: Route the optional safe-host handles profile, shared leaf ownership, native capacity/retirement tests and still-gated Node publication to knowledge/pwright-node-handles.md. Existing addon artifacts must not select handles.

NODE-151: Route Node Handle prerequisites, 18-case oracle/paired-red evidence and the borrowed payload-target boundary to knowledge/pwright-node-handles.md and the file-input checkpoint. Public Node Handles remain absent.

NODE-150: Route Page/Frame upload APIs and Rust-scoped query ownership to the file-input record: standalone 43/43 x3, combined 43/43, 100 package contracts and 30 shared units pass. Full corpus remains 53/57 matching. Raw-click capture and its reversed mismatch are recorded separately in pwright-raw-mouse.
[Page/Frame upload contract and verification](knowledge/pwright-file-input.md#node-150-page-and-frame-selector-taking-upload-apis).

NODE-149 verification: both selected artifacts pass 32 upload cases; full shared corpus is 53/57 matching and remains non-green. See the canonical file-input record for exact remaining rows.

NODE-149: Route opt-in addon profiles, Buffer ingress/type validation, public Locator uploads, packaging notices, Worker teardown and differential verification to the file-input record.
[Selected Node upload boundary and verification](knowledge/pwright-file-input.md#node-149-selected-node-buffer-ingress-and-public-locator-uploads).

NODE-148: Route selected safe-host upload APIs, shared resolver/Frame lifetime, source admission and shutdown drain changes to the file-input record. Public addon/JS integration remains pending.
[Selected safe-host upload contract](knowledge/pwright-file-input.md#node-148-selected-safe-host-upload-integration).

NODE-147: Route optional large outbound CDP admission, fragmented transport, backend selection and near-limit browser evidence to the file-input record. Ordinary command/inbound ceilings and public Node artifacts remain unchanged.
[Large outbound admission and remaining gates](knowledge/pwright-file-input.md#node-147-independently-admitted-large-outbound-commands).

NODE-146: Adds admitted exact-element upload continuations and an explicit CDP settlement wait. Route dropped-waiter, timeout, source/group/route ownership, cleanup health, selected dependency and test changes to the file-input record. Public Node upload and native directory completion remain open.
[Upload continuation and remaining gates](knowledge/pwright-file-input.md#node-146-single-use-upload-continuations).

NODE-145: Adds the opt-in Chromium payload mutation backend and real-browser source-to-File evidence. Route DOM behavior, encoding/scratch admission, group-owned by-value CDP calls and exception cleanup, native ownership and upload validation changes to the file-input record. Native path/directory completion, cancellation composition and public Node wiring remain open.
[Chromium payload backend and evidence](knowledge/pwright-file-input.md#node-145-chromium-payload-mutation).

NODE-144: Adds the optional admitted Tokio file-input executor. Route worker admission, cancellation, drain and queue/source lifetime changes to the file-input knowledge record. Browser backend, large-message transport and public Node upload are still open.
[Admitted executor contract and evidence](knowledge/pwright-file-input.md#node-144-admitted-blocking-executor).

NODE-143: Adds optional filesystem path preparation, admitted descriptor/path ownership and a pinned MIME provider. Route filesystem/MIME changes to the file-input knowledge record; executor admission, browser actions and public Node upload remain open.
[Filesystem preparation and evidence](knowledge/pwright-file-input.md#node-143-filesystem-preparation-and-pinned-mime).

NODE-142: Adds the optional ledger-admitted file-input content owner, selected only as a standalone leaf. Update source ownership, failure/cancellation evidence and dependency routing. Path preparation, executor admission, browser upload and public Node wiring remain open.
[File-input content ownership and remaining work](knowledge/pwright-file-input.md#node-142-ledger-admitted-content-ownership).

NODE-141: Begins optional Rust file-input preparation with a zero-dependency model and pinned real-browser oracle characterization. Update file-input semantics, ownership/resource and dependency boundaries, architecture, Node/package status and implementation tasks. No public upload API or native addon selection is implemented yet.
[File-input policy, evidence and remaining work](knowledge/pwright-file-input.md).

NODE-140: Reopens the complete Frame and shared API corpora and adds bounded Page-close/body and raw-click diagnostics. Update testing strategy, Frame/close and raw-Mouse evidence, request-capacity status, Node/package scope and design tasks. Preserve non-green full gates separately from prior focused passes.
[Expanded verification and retained failures](knowledge/testing-strategy.md#node-140-broadened-node-ffi-verification-remains-non-green).

NODE-139: Retain Page lifetime through Frame.goto publication and result projection, with a borrowed close wait on existing lifecycle owners. Update Frame navigation, close-cascade interpretation, lifecycle/ownership, resource admission, architecture, Node/package status and testing/tasks. The earlier natural error comparison used different native commit phases; do not label it a proved semantic discrepancy.
[Page lifetime publication contract and verification](knowledge/pwright-frame-navigation.md#node-139-page-lifetime-through-navigation-publication).

NODE-138: The opt-in Node driver now owns concurrent native closes and latest-state local retirement. Update close-cascade, lifecycle/ownership, resources/architecture, package profiles and verification status. Default/external Context paths and ordinary builds are unchanged.
[Node close-driver integration and final verification](knowledge/pwright-close-cascade.md#node-138-opt-in-node-close-driver-integration).

NODE-137: Adds native-proven synchronous local close retirement and a returned post-lock notification owner. Update close-cascade, architecture/resources, object ownership, Node lifecycle/compatibility status and tasks/testing. The Node driver and controlled browser gate remain unwired.
[Native local retirement and post-lock notification contract](knowledge/pwright-close-cascade.md#node-137-native-proven-local-retirement-and-post-session-lock-notification).

NODE-136: Joined close-capacity admission and consuming native completion now compose move-only reservations with borrowed evidence. Update close-cascade, architecture/resource/object ownership, Node lifecycle/compatibility status and tests/tasks. Keep the full Node coordinator and browser discrepancy open.
[Joined close capacity and native completion contract](knowledge/pwright-close-cascade.md#node-136-joined-close-capacity-and-consuming-native-completions).

NODE-135: Adds runtime-locked local publication plus deferred disposal, preventing transport mutation between the two phases of a selected close commit. Update close-cascade, architecture/resource/object ownership, Node lifecycle/compatibility status and testing/tasks. This does not wire the full native close coordinator.
[Runtime-locked publication/disposal contract and evidence](knowledge/pwright-close-cascade.md#node-135-runtime-locked-local-publication-and-disposal).

NODE-134: Adds optional runtime-owned control outbox backing admission and fallible preallocation through the existing disposal-reservations feature. Update close-cascade, runtime/resource ownership, architecture/dependencies, Node lifecycle/compatibility status and testing/tasks. Graph/container/commit scratch and the full close coordinator remain unfinished.
[Preallocated runtime outbox contract and evidence](knowledge/pwright-close-cascade.md#node-134-preallocated-runtime-control-outbox-storage).

NODE-133: Selected native-world receipts retain the owning Page target and the driver marks them on actual native target destruction, even after local routes disappear. Update close-cascade, topology/Locator/native ownership, resource and lifecycle contracts, architecture, Node/package status and verification tasks. Public close scheduling remains separate.
[Native Page world-retirement contract and evidence](knowledge/pwright-close-cascade.md#node-133-retained-worlds-follow-exact-native-page-destruction).

NODE-132: Adds attachment-bound native evidence validation and latest-topology callback serialization. Update close-cascade, architecture, native ownership/resource contracts, Node lifecycle/compatibility status, tasks and tests. It is not the complete retirement coordinator.
[Native close commit-scope contract and evidence](knowledge/pwright-close-cascade.md#node-132-attachment-bound-native-close-commit-scope).

NODE-131: Adds selected deferred runtime commits, collection-payload peak admission and explicit post-lock notification ownership. Update close-cascade, runtime resources/object ownership, lifecycle/architecture, Node status, tasks and testing. Full close coordinator and host integration remain unfinished.
[Deferred runtime close commit contract and evidence](knowledge/pwright-close-cascade.md#node-131-deferred-runtime-disposal-notifications).

NODE-130: Adds independent deletion-only route retirement, strict survivor validation, atomic subscription pruning and a separate post-commit wake port. Update close-cascade, route/resource ownership, Frame navigation/publication, lifecycle integration status, architecture/dependencies, Node/package and tasks/testing. Full close transaction and Node scheduler integration remain unfinished.
[Deletion-only close route contract and evidence](knowledge/pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-129: Adds opt-in bounded native close scheduling and guarded Page command initiation. Update close-cascade, native ownership/resources, lifecycle/publication integration status, dependency/architecture, Node/package and testing/tasks. The Node host and retirement commit are still unwired.
[Bounded native scheduler contract and evidence](knowledge/pwright-close-cascade.md#node-129-bounded-native-close-scheduler).

NODE-128: Adds opt-in exact native Context ancestor authority, sealed Page bindings, bounded command receipts and current-route certification. Update close-cascade, Page/Browser lifecycle ownership, resources, architecture/dependencies, Node/package status and testing/tasks. Keep native scheduler/publication integration and the browser gate unfinished.
[Native ancestor authority contract and evidence](knowledge/pwright-close-cascade.md#node-128-exact-native-context-ancestor-authority).

NODE-127: Adds split native command acknowledgement and destruction-wait ownership. Update close-cascade, Page/Browser close, native/lifecycle ownership/resources, architecture/dependencies, Node/package status and testing/tasks. Keep the coordinator and ancestor-dispatch gate explicitly unfinished.
[Native close phase contract and evidence](knowledge/pwright-close-cascade.md#node-127-native-command-and-destruction-wait-phases).

NODE-126: Adds optional deferred close publication admission and latest-view preparation. Update close-cascade, publication/lifecycle/ownership/resources, architecture/dependencies, Node/package status and testing/tasks. Native scheduling and ancestor receipts remain unwired.
[Deferred publication contract and evidence](knowledge/pwright-close-cascade.md#node-126-deferred-close-publication-capacity).

NODE-125: Adds opt-in runtime disposal/outbox reservations, queue-slot accounting and latest-state preparation. Update close-cascade, lifecycle/object/resource contracts, architecture/dependencies, Node/package status and testing/tasks. Selected/combined/ordinary tests pass 142/159/113; native intent scheduling and publication rebasing remain unwired. Preserve closed-transport precedence red/green evidence separately from the unchanged browser discrepancy.
[Runtime reservation contract and evidence](knowledge/pwright-close-cascade.md#node-125-opt-in-runtime-disposal-and-outbox-reservations).

NODE-124: Adds opt-in topology transition reservations with an independent budget/pending limit and latest-state exclusive commit. Update close-cascade/topology contracts, resources/ownership, architecture/dependencies, Node/package status and design/testing tasks. Keep the feature unwired in host/addon and retain the ancestor-dispatch gap. Selected topology passes 76 tests; ordinary passes 49; normal dependency/features are unchanged.
[Topology reservation contract and evidence](knowledge/pwright-close-cascade.md#node-124-opt-in-topology-transition-reservations).

NODE-123: Adds close-cascade evidence/design and bounded transparent/controlled probes. Fixes Context interruption semantics and seals admission when driver reply producers or request queues disappear before the normal stop block. Update Browser/Page/Frame close, lifecycle/publication, object/resource ownership, architecture, Node/package status and design/testing tasks. Preserve the intermediate peer-repeat failure and distinguish final 50-case matrices plus repeated confirmation gates from the red ancestor-dispatch probe. Concurrent intents and reservation/commit separation remain unimplemented.
[Close cascade contracts and evidence](knowledge/pwright-close-cascade.md).

NODE-122: Adds selected native Page destruction confirmation, bounded source/call/error ownership, an independent reclaiming cursor and terminal-settlement fencing. Update Page/Browser close, topology, lifecycle/publication, dependency/resource/object ownership, architecture, Node/package status, design tasks and verification evidence together. Also update Frame navigation for the combined-profile fixture harness and retained 27/29 arrival-boundary failures. Both final artifacts pass 48 close/lifecycle/connection cases; historical failed runs remain distinct from final gates and overall compatibility remains unfinished.
[Native confirmation contract and evidence](knowledge/pwright-page-close.md#node-122-selected-native-destruction-confirmation).

NODE-121: Adds bounded original-scenario close tracing and a pinned-source-based native close confirmation plan. Update Browser/Page close contracts, lifecycle/testing evidence, resource boundaries, Node/package status and design tasks. Diagnostic code changes do not imply a native behavior fix or new addon artifact.
[Transparent trace and next confirmation contract](knowledge/pwright-browser-close.md#node-121-transparent-original-scenario-tracing).

NODE-120: Adds bounded Rust native-driver error attribution and diagnostic-only close probes. Update Browser/Page close, lifecycle evidence, architecture/resource boundaries, Node/package status, testing and design tasks. Preserve natural failure evidence separately from controlled injection and later successful repeats.
[Close attribution contract and evidence](knowledge/pwright-browser-close.md#node-120-attribute-retained-native-driver-failures).

NODE-119: Connects native ready/linkage production through masked collection commits and per-Page Rust acknowledgement state. Update lifecycle/publication, creation/close, object/resource ownership, architecture/dependencies, Node/package status, design tasks and tests. Record both complete event corpora and exact Frame/cleanup resource assertions; keep the overall compatibility/stability gaps explicit.
[Native ready-source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-119-native-ready-and-popup-source).

NODE-118: Adds precommit ready/popup descriptor conversion, missing-wrapper requests and metadata-root binding. Update publication/lifecycle, object/resource ownership, creation/close, Node/package, architecture/dependencies, tasks and tests. Keep conversion evidence separate from pending production native creation sources.
[Creation-conversion contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-118-precommit-creation-wrapper-conversion).

NODE-117: Joins committed external disposal effects with independently pre-admitted close capacity. Update lifecycle/publication, creation/close, object/resource ownership, architecture/dependencies, Node/package status, tasks and testing. Preserve the two creation-event gaps and distinguish native Context-cascade tests from live external Page-close proof.
[External-close source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-117-pre-admitted-external-close-source).

NODE-116: Wires explicit-close attachment factories and pre-admitted native receipts into the existing Node dispatcher. Update lifecycle/publication, close/creation, ownership/resources, architecture/dependencies, Node/package, tasks and tests; preserve compact Context-cascade semantics and the explicit external-close/creation gaps.
[Explicit-close source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-116-explicit-close-source-integration).

NODE-115: Selects native terminal production in the Node addon and adds one bounded environment-thread dispatcher across all publication paths. Update lifecycle/publication, ownership/resources, architecture/dependencies, Node/package status, close/creation, tasks and testing; distinguish working terminal events from unfinished ordinary sources.
[Terminal dispatcher contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-115-environment-thread-terminal-dispatch).

# Documentation manifest

NODE-114: Integrates an opt-in terminal native source with pre-initialization group admission and adds a real host-shutdown gate. Update lifecycle/publication, resource/ownership, architecture/dependencies, Node/package status, Page/Context/Browser boundaries, tasks and verification. Preserve the distinction between native source completion and pending Node event delivery.
[Terminal source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-114-pre-admitted-native-terminal-source).

NODE-113: Adds pre-admitted groups to the existing Rust publication queue and extends listener characterization. Update lifecycle/publication, object/resource ownership, architecture/dependencies, Node/package status, creation/close, tasks and testing. No selected Node event support is claimed.
[Group contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-113-bounded-dependent-publication-groups).

NODE-112: Adds the optional prepared/committed native close observer, preserving ordinary
service fields and existing native lock order. Update lifecycle/publication,
Page/Context/Browser source coverage, ownership/resources, architecture and
dependencies, Node/package status, tasks and verification evidence.
[NODE-112 close receipt contract and verification](knowledge/pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: Adds the borrowed committed driver-effect observer, without a host event journal
or Node callback integration. Update topology events, lifecycle/publication,
creation/close source coverage, ownership/resources, architecture/dependencies,
Node/package status, tasks and testing; distinguish native input from delivery.
[NODE-111 source contract and verification](knowledge/pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-110 characterizes listener failure/reentry and adds scoped Rust exception
reporting for existing Browser disconnected dispatch. Update lifecycle events,
Browser close, Frame publication, Context/Page creation, ownership/resources,
architecture/dependencies, Node/package, tasks and testing. Preserve pre-fix
error-channel evidence and distinguish the fixed Browser cases from missing
Page/Context notifications and unimplemented dependent native cause groups.
[NODE-110 contracts, fix and verification](knowledge/pwright-node-lifecycle-events.md#node-110-listener-failure-boundaries-and-scoped-exception-reporting).

NODE-109 introduces the exclusive Rust metadata drain and its fixed storage
allowance, without wiring Node callback delivery. Update lifecycle-event status,
Frame publication, Context/Page creation, Page/Browser close, object ownership, resources, architecture,
dependencies, Node/package, design tasks and testing. Distinguish raw queue
advancement from atomic event-state publication; record rebuilt artifact evidence.
[NODE-109 contract and verification](knowledge/pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-108 adds bounded native lifecycle TRACE identities and an additive attach-
probe report field. Update topology diagnostics, event admission trace ordering,
resources/dependencies, Node/package, lifecycle implementation status, tasks and
testing. Distinguish freshly rebuilt native probes from unchanged Node addons;
preserve prior initialization failures and do not claim a behavioral timeout fix.
[NODE-108 receipts and verification](knowledge/pwright-chromium-topology-events.md#node-108-bounded-native-lifecycle-identity-receipts).

NODE-107 also records newly reproduced initial-attachment failures and bounded
command receipts on the rebuilt profiles. Update topology-event driver diagnostics,
Node/package, tasks and testing to mark the live gate partial. Do not attribute
setup timeouts to missing lifecycle notifications or erase earlier failures.

NODE-107 adds cross-revision visibility rebasing, metadata-domain identity and
Context settings identity validation. Update lifecycle-event status, Frame
publication, ownership/resources, architecture/dependencies, Context creation,
Page/Browser close, Node/package, tasks and testing. Record the new layout and
rebuilt artifacts without claiming the step owner or callbacks are implemented.
[NODE-107 contracts and evidence](knowledge/pwright-node-lifecycle-events.md#node-107-validated-cross-revision-visibility-rebasing).

NODE-106 introduces immutable Rust collection visibility and changes the selected
published-collection return type and layout allowances. Update lifecycle-event
status, Frame publication, resources, object ownership, architecture/dependencies,
Page/Browser close, Context creation, Node/package, tasks and testing. Record
rebuilt artifacts; distinguish this single-revision foundation from the still
unimplemented cross-revision step cursor and lifecycle callback dispatch.

NODE-105 adds a RED Node lifecycle-event gate and stepwise publication design.
Update lifecycle-event design, Frame publication, Page/Context creation and close,
Browser close, ownership/resource/dependency design requirements, Node/package,
tasks and testing. This introduces no production behavior, budget or dependency;
keep callback-state requirements separate from already-green collection getters.

NODE-104 adds paired creation/discovery collection publication characterization.
Update Frame publication, Context/Page creation, topology, Node/package, tasks
and testing. Preserve the concurrent-order and FFI-lane harness corrections;
record unchanged artifacts and separate collection coverage from missing events.
No production resource, dependency or architecture contract changed.

NODE-103 wires selected Rust collection producers and Node getters, atomic batch
publication, pre-admitted retirement pools/creation rollback, and Rust baseURL
TypeError validation. Update Frame publication, topology, resources, ownership,
architecture/dependencies, Page/Browser close, Context creation, Node/package,
tasks and testing. Preserve older red/unwired checkpoints as historical evidence;
record rebuilt artifact hashes and final matrices, not broad parity claims.

NODE-102 implements bounded Rust collection revision storage on the existing
publication queue. Update Frame publication, resources, object ownership,
architecture/dependencies, Page/Browser close, Context creation, Node/package,
tasks and testing. Record measured logical budgets and distinguish storage
implementation from still-unwired producers/getters and the unchanged red gate.

NODE-101 adds Context/Page collection publication characterization and a RED
candidate lane. Update Frame publication, Page/Browser close, Context creation,
resource-design requirements, Node/package, tasks and testing. No production
capability or new byte budget is implemented; preserve artifact-specific failures
and distinguish an oracle harness-order correction from a product discrepancy.

NODE-100 changes Browser connection publication timing and Rust-owned event
dispatch suppression. Update Browser/Page close, Frame publication, object and
resource ownership, architecture/dependencies, Node/package, tasks and testing.
Keep native admission separate from public state, preserve the red baseline,
and identify rebuilt artifacts; do not imply complete lifecycle compatibility.

NODE-099 implements Rust-published Page.isClosed and pre-admitted terminal queue
ownership. Update Page/Browser close, Frame publication, object/resource contracts,
architecture/dependencies, Node/package, tasks and testing. Preserve the earlier
red baseline as historical evidence; only rebuilt artifacts establish support.

NODE-098 adds Page.isClosed reference characterization, a red candidate lane and
a terminal-publication design gate. Update Page-close/publication, Node/package,
tasks and testing. This changes no production API, resource contract or artifact;
reference-only success must not be reported as candidate compatibility.

NODE-097 isolates a native cross-client Page auto-attachment trigger and expands
the peer lane to direct failure/recovery comparisons. Update topology/navigation,
Browser-close and header stress attribution, Node/package, tasks and testing.
No production API/resource/architecture changes or broad compatibility claims.

NODE-096 adds bounded navigation timing/retirement diagnostics and the controlled
navigation-peer-disconnect test. Update topology/navigation evidence, header
fixture observation rules, Node/package, tasks and testing. Correct the earlier
untimed load inference; no production architecture/resource contract changes.

NODE-095 adds selected reader-level pending-session retirement and typed detach
handling, with bounded command identity storage. Update topology/close/publication,
headers, ownership/resources, architecture/dependencies, Node/package, tasks and
testing evidence. Preserve default/v1 opt-out and the distinct no-pending-call
navigation timeout; do not claim full CRSession or API parity.

NODE-094 changes selected Context fallback for stale discovery snapshots and
adds queued-context-retirement. Update topology, Context/object/publication/close,
architecture/resources, Node/package, tasks and testing records. It does not
normalize root-detach errors or prove full concurrent/API compatibility.

NODE-093 preserves typed header initialization errors and expands attachment
occurrence/child-header command diagnostics. Update header/topology, Node/package,
tasks and testing evidence. Resource/feature/dependency contracts are unchanged;
do not promote targeted retirement coverage to full concurrent disconnect parity.

NODE-092 corrects exact second-stage native child-detach retirement and adds a
controller-owned browser Worker race/control lane. Update topology/close and
publication, Node/package, ownership/resources, architecture/dependencies, tasks
and testing. Do not generalize this receipt into all cleanup/Worker parity.

NODE-091 adds request-scoped native session-close watches and a composed optional
header observer, plus the public header-child-sessions differential lane. Update
headers, Node/package, topology/publication/navigation, ownership/resources,
architecture/dependencies, tasks and testing. Broader shutdown and API parity
remain unproven.

NODE-090 changes discovered-Page initialization failure handling and prepared
lifecycle-owner retirement. Update topology, publication/navigation, ownership,
resources, architecture/dependencies, Node/package, tasks and testing records.
The attachment-retirement lane is not proof of full connection/API parity.

NODE-089 corrects Context Page sequencing, defers later-Page header snapshots,
and adds real-Chrome receipt-gated public success/failure comparisons. Update
header, Node/package, architecture, ownership/resource, dependency, task and
testing records. Preserve NODE-088 as historical evidence, not current dispatch
semantics; shutdown, OOPIF/Worker and broader compatibility remain open.

NODE-088 adds caller-polled header jobs, bounded native fanout, early primary
rejection with retained compensation, and native-capacity backpressure. Update
headers, topology/publication/creation/close, Node/package, architecture,
resources/ownership, dependencies, tasks and testing. Shutdown retirement and
real-CDP failure-timing fidelity remain unproven.

NODE-087 moves selected header/navigation preparation to the synchronous Rust
call boundary, freezes native dispatch and selects nine metadata writers. Update
Node/package, header/navigation, ownership/resource, architecture, dependency,
task and testing records. Native concurrency/early settlement remains unfinished.

NODE-086 reproduces a pending header-setter/navigation ordering discrepancy and
adds explicitly bounded overlapping metadata writers. Update header, navigation,
Node/package, ownership/resource, dependency, task and testing records. Internal
128-case agreement must not be presented as fixed public scheduling.

NODE-085 wires Page-only referrer policy into public Rust Page/Frame navigation.
Update header/navigation, Node/package, ownership/resources, dependency, task and
testing records. Paired first-line/wire checks do not establish call-log fidelity,
pending setter races, worker/OOPIF routing or complete navigation compatibility.

NODE-084 adds selected Rust-owned Context creation headers and public differential
coverage. Update header/Context creation, Node/package, ownership/resources,
publication/dependencies, tasks and testing records; creation fault injection and
broader header/navigation parity remain open.

NODE-083 separates bounded desired-header storage from native syntax rejection,
and isolates failed explicit Page initialization through exact-Page compensation.
Update headers, Context/Page creation and close, topology/publication, ownership,
resources, Node/package, dependency, task and test records.

NODE-082 exposes experimental selected Node header setters and phase-aware
language conversion. Update header, Node/package, resources/ownership,
Context/publication, dependency, tasks and testing records. Nine paired groups
and one admission test do not establish failure timing, workers or full parity.

NODE-081 selects the Rust native header host and fixes obsolete-session header
state. Update header, topology/publication, Context, ownership/resources,
dependencies, Node/package, tasks and testing records. Native wire tests are
not public Node/worker coverage or upstream failure-timing compatibility.

NODE-080 implements bounded Rust desired-header transactions and a 54-case
pinned internal setter comparison. Update header, ownership/resources, Context,
Node/tasks, dependency, package and testing records. Native installation and
public setters remain open; uncertainty tracking must not block desired reads.

NODE-079 extends native session initialization with borrowed portable ownership
while preserving legacy callbacks. Update topology, header, publication/object,
resource/dependency, Node/task and testing records. This is not a settings
registry, native header update or public FFI implementation.

NODE-078 adds optional Rust header snapshots/merge policy and pinned internal
and public-oracle characterization. Update header, Context/navigation, resource,
dependency, Node/task, package and testing records. Native installation and
public setters are still open; internal agreement is not full API compatibility.

NODE-077 extends public navigation with executed-document close, timeout
continuation and candidate Worker ownership/queued-result tests. Update Frame
navigation, Page close/reload relationship, object/resource ownership, Node/task
and package/testing records. Preserve the intermittent native close failure;
clean Worker exit is not a post-destruction RAM measurement or full parity.

NODE-076 exposes optional public Node Frame.goto and composable native profiles.
Update navigation, Node/package, dependency, object/publication/resource and
task/testing records. Keep failed oracle close runs distinct from independently
verified Rust cleanup and do not promote subset results to full compatibility.

NODE-075 connects typed navigation values to the Rust host executor with
independent result leases. Update navigation, ownership/resources, publication,
dependency, Node/task and test records. Host result projection is distinct from
generated server installation and public Node differential completion.

NODE-074 adds separately selected navigation 1.2 contracts, typed codecs and
server bindings. Update navigation, generator/IDL, dependencies, object/resource
ownership, Node design, tasks and testing records. Typed stream verification is
not production host integration or a public Node/browser differential result.

NODE-073 joins retained native routes and ordered navigation into the selected
Rust host executor. Update navigation, ownership/resources, publication/topology,
dependency, Node design, tasks and testing records. Dynamic streaming cancellation
tests remain distinct from generated/public Node differential evidence.

NODE-072 adds ordered Page navigation registrations and separate Rust host
selection. Update navigation, topology/publication, ownership/resources,
dependency, Node design, tasks, testing and shared comparison records. Actual
host observation is separate from host-owned dispatch and public Frame.goto.

NODE-071 adds an optional per-invocation navigation matcher/response owner and
pinned internal oracle. Update navigation, ownership/resources, dependencies,
Node design, tasks, testing and shared comparison records. Do not treat the
standalone state machine as a production host or public Frame.goto gate.

NODE-070 adds bounded multi-Page lifecycle composition and opt-in Rust host
installation. Update navigation, topology, publication, resource/ownership,
dependency, Node, task and test records. Readiness selection is not a navigation
response/operation implementation or public Node compatibility gate.

NODE-069 adds opt-in production ordered lifecycle observation and a one-Page
resource owner. Update navigation, topology, publication, resource/ownership,
dependency, Node, task and test records. Driver integration is not completion of
multi-Page/Worker ownership, navigation waiters or public Frame.goto.

NODE-068 changes production document-commit descendant retirement and its
runtime/route publication. Update navigation, topology, Frame publication,
object/resource ownership, Node design, task and test records. Live renderer
handoff does not establish full lifecycle or public Frame.goto parity.

NODE-067 adds bounded native Frame lifecycle normalization, borrowed notices and
live single-renderer CDP verification. Update navigation, topology, resource,
ownership, dependency, Node, task and test records. Native-adapter success is not
production event-driver integration, OOPIF/worker parity or public Frame.goto.

NODE-066 adds an optional transaction-published retained Frame navigation table.
Update navigation, topology publication, resources/ownership, dependencies, Node
and task/testing records. Rust route tests do not establish public Frame.goto or
native cancellation; the host has not selected this owner yet.

NODE-065 adds the default-off normalized Frame lifecycle reducer and pinned
internal differential traces. Update navigation, resource/ownership/dependency,
Node/task and testing records. Neither native normalization nor public Frame.goto
is completed by this state-machine gate.

NODE-064 adds world-independent borrowed Frame navigation routes, explicit
native Frame initiation and six pinned local/OOPIF observations. Update the
navigation design, ownership/resources/dependencies, Node/task and testing docs.
Native initiation and final-state probes do not establish public Frame.goto parity.

NODE-063 public Frame lookup binds Rust selection to scoped Node language values
and cache-aware selected ancestry. Update lookup/publication, resource/ownership,
dependency, Node/task and testing records; distinguish the 41-case public gate
from broad compatibility and still-open Frame/event/connection work.

NODE-062 Context baseURL ownership/navigation and its original-layout live gates
now pass: 16 public observations, four boundary groups, Page 7/7 and glob
1647/1647 plus seven checks. Update Context creation, resources, object ownership,
dependency, Frame lookup/publication, Node/task and testing records. Keep earlier
attachment failures and the unresolved reliability investigation visible.

NODE-061 reuses and corrects the existing attachment collector, adds an async
layout wrapper and refreshes unchanged-artifact original-layout results. Update
topology diagnostics, lookup, Node/task and testing records; preserve the open
connection reliability investigation separately from the newly passing runs.

NODE-060 adds the actual N-API URL-value port, native-copy admission and a
labelled browser diagnostic. Update lookup/publication, resource, ownership,
dependency, Node/task and testing records. Preserve failed original-layout
attachment/Page regressions separately from the successful late-observer run.

NODE-059 adds Rust URL-glob preprocessing/restoration and a separately owned
URL-resolution port. Update the lookup, resource, ownership, dependency, Node,
task and test/package records; distinguish diagnostic parser behavior from its
still-required production allocation admission and public API integration.

NODE-058 implements caller-admitted Rust lookup traversal and resolved-glob
planning. Its resource proof, offline oracle bridge and source attribution update
the Frame lookup, ownership, resource, dependency, Node/task and testing records.

NODE-057 adds a version-pinned Frame lookup design/characterization record and
optional Mouse focus/setup diagnostics. Keep diagnostic experiments distinct
from candidate conformance and artifact-specific regression results.

NODE-056 navigation names update the Frame publication, object model, resource,
dependency, Node FFI, task and test/package records. Name storage is an explicit
optional ledger; public lookup and Frame events remain separate unfinished work.
The zero-input Mouse diagnostic and failing regression are additionally tracked
in `knowledge/pwright-raw-mouse.md`; historical passes are artifact-specific.

Update the matching document and its freshness timestamp when a trigger changes.
Existing documentation is indexed here; listing does not imply every legacy page
has been re-reviewed. Canonical technical designs remain Markdown.

Retained Frame host projection, pre-native metadata/root-binding admission, parent-scoped
child release and paused-OOPIF initialization are tracked in
`knowledge/pwright-frame-publication.md`, with linked resource, dependency,
Node package, input-regression and testing records.

Cache-aware descriptor reads, native Page-tree boxing, four-state pre-creation
admission and Node user-script activation additionally update the value-evaluation
contract and the public Page-enumeration comparison records.

Optional runtime lifetime handoff, its last-reference signal-byte budget and
Frame-to-Page input phase contract are recorded in `knowledge/pwright-chromium-drag.md`,
with resource/dependency summaries and linked Locator, typing, Node and test docs.

Selected Node artifact build/loading, compiled-capability verification, shared
artifact provenance and native/public input results also update the Node package
README, Node FFI design, dependency topology and comparison/testing guides.

| File | Covers | Update When |
| --- | --- | --- |
| [README.md](README.md) | pwright Documentation | Crate/package layout, capabilities, or documentation navigation changes |
| [architecture-v2.md](architecture-v2.md) | Composable architecture v2 (discussion draft) | Behavior, API, or architecture described in this document changes |
| [architecture.md](architecture.md) | Architecture | Behavior, API, or architecture described in this document changes |
| [api-request-design.md](api-request-design.md) | Playwright-compatible browser-independent API request ownership, transport boundary, resources and conformance gates | Request transport, cookies/storage, public bindings, limits, lifecycle or official APIRequest evidence changes |
| [bugs/gmail-cdp-click-limitation.md](bugs/gmail-cdp-click-limitation.md) | Gmail (and similar SPAs) cannot be controlled via pwright click | Behavior, API, or architecture described in this document changes |
| [bugs/index.md](bugs/index.md) | Bug Reports | Behavior, API, or architecture described in this document changes |
| [bugs/tab-leak-bug-report.md](bugs/tab-leak-bug-report.md) | Bug Report: `with_page` Tab Leak Under CDP Connection Failure | Behavior, API, or architecture described in this document changes |
| [bugs/targetinfo-http-parse-failure.md](bugs/targetinfo-http-parse-failure.md) | Bug: TargetInfo struct fails to parse Chrome HTTP /json/list response | Behavior, API, or architecture described in this document changes |
| [cli-guide.md](cli-guide.md) | CLI Guide | Behavior, API, or architecture described in this document changes |
| [code-quality-findings.md](code-quality-findings.md) | Code Quality Findings | Behavior, API, or architecture described in this document changes |
| [codelabs.md](codelabs.md) | Codelabs: pwright Usage Guide | Behavior, API, or architecture described in this document changes |
| [design-tasks-v2.md](design-tasks-v2.md) | Pwright Rust v2 design tasks | Design decisions, implementation checkpoints, packaging reproducibility, or verification gates change |
| [exploration/github-trending-structure.md](exploration/github-trending-structure.md) | GitHub Trending HTML Structure | The investigated site structure or automation recipe changes |
| [exploration/reddit-structure.md](exploration/reddit-structure.md) | Reddit HTML Structure | The investigated site structure or automation recipe changes |
| [exploration/weibo-structure.md](exploration/weibo-structure.md) | Weibo HTML Structure | The investigated site structure or automation recipe changes |
| [exploration/wikipedia-structure.md](exploration/wikipedia-structure.md) | Wikipedia HTML Structure | The investigated site structure or automation recipe changes |
| [exploration/x-structure.md](exploration/x-structure.md) | X.com (Twitter) HTML Structure | The investigated site structure or automation recipe changes |
| [exploration/xueqiu-structure.md](exploration/xueqiu-structure.md) | Xueqiu HTML Structure | The investigated site structure or automation recipe changes |
| [exploration/zhihu-structure.md](exploration/zhihu-structure.md) | Zhihu HTML Structure | The investigated site structure or automation recipe changes |
| [feedback-from-gemini-pdf-extractor.md](feedback-from-gemini-pdf-extractor.md) | docs/feedback-from-gemini-pdf-extractor.md | Behavior, API, or architecture described in this document changes |
| [getting-started.md](getting-started.md) | Getting Started | Behavior, API, or architecture described in this document changes |
| [grpc-api.md](grpc-api.md) | gRPC API Reference | Behavior, API, or architecture described in this document changes |
| [guides/remote-cdp.md](guides/remote-cdp.md) | Connecting to Chrome on a Remote Machine | Behavior, API, or architecture described in this document changes |
| [index.md](index.md) | pwright - Project Guide | Crate/package layout, capabilities, or documentation navigation changes |
| [knowledge/api-parity.md](knowledge/api-parity.md) | API Parity: gRPC vs CLI vs Playwright Bridge | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/cdp-codegen-design.md](knowledge/cdp-codegen-design.md) | Design: CDP Protocol Codegen Crate (`pwright-cdp-gen`) | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/chrome-devtools-mcp-comparison.md](knowledge/chrome-devtools-mcp-comparison.md) | pwright CLI vs chrome-devtools-mcp CLI | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/code-review-2026-03-29.md](knowledge/code-review-2026-03-29.md) | Code Review Findings - 2026-03-29 | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/critical-review-2026-03-29.md](knowledge/critical-review-2026-03-29.md) | Critical Review Findings - 2026-03-29 | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/feature-comparison.md](knowledge/feature-comparison.md) | Feature Comparison: Playwright vs PinchTab vs pwright | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/network-capture-design.md](knowledge/network-capture-design.md) | Design: Network Capture CLI | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/playwright-api.md](knowledge/playwright-api.md) | Playwright API Compatibility | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/playwright-design-course.md](knowledge/playwright-design-course.md) | Playwright design and capability course | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-artifacts-and-streams.md](knowledge/pwright-artifacts-and-streams.md) | Pwright Artifact and Stream boundary | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-browser-close.md](knowledge/pwright-browser-close.md) | Playwright-aligned Browser close and disconnect design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-chromium-topology-events.md](knowledge/pwright-chromium-topology-events.md) | Chromium topology-event driver | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-chromium-drag.md](knowledge/pwright-chromium-drag.md) | Drag primitives, opt-in Mouse/Keyboard/Locator host selection, Page quarantine and retained utility-world snapshots | Feature selection/dependencies, original-Arc native extension views, Frame source/keyboard snapshot selection and retention, single-world destruction receipts, live-state reads, Mouse branches, Page drag state, capacity/admission, terminal retirement/quarantine/driver lifetime, host integration or native/differential evidence changes |
| [knowledge/pwright-cdp-event-admission.md](knowledge/pwright-cdp-event-admission.md) | Optional pre-admission of CDP events, selected Frame-world policy and extension cursor accounting | Event classification, profile selection, fanout derivation, resource/validation ordering or verification changes |
| [knowledge/pwright-context-creation.md](knowledge/pwright-context-creation.md) | Playwright-aligned BrowserContext creation design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-dependency-topology.md](knowledge/pwright-dependency-topology.md) | Pwright v2 dependency topology | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-element-handle-adoption.md](knowledge/pwright-element-handle-adoption.md) | ElementHandle adoption | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-generator-output-ownership.md](knowledge/pwright-generator-output-ownership.md) | Pwright generator output ownership | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-handle-evaluation.md](knowledge/pwright-handle-evaluation.md) | Handle evaluation, raw retained-result acquisition and bounded failure compensation | Capability, native acquisition/settlement, parser ownership, resource contract or verification changes |
| [../crates/pwright-composition-chromium-handle-evaluation/README.md](../crates/pwright-composition-chromium-handle-evaluation/README.md) | Independent Handle evaluation leaf and optional staged native creation | Public leaf APIs, selection, acquisition/settlement, native/publication ownership or continuation obligations change |
| [knowledge/pwright-node-handles.md](knowledge/pwright-node-handles.md) | Node retained-Handle integration, upload prerequisites and public conformance lane | Node Handle producers, publication, ownerFrame/disposal/adoption, borrowed payload target, budgets or verification changes |
| [knowledge/pwright-handle-lifecycle.md](knowledge/pwright-handle-lifecycle.md) | Handle lifecycle, native compensation, shutdown ordering and continuation failure reporting | Native ownership, release settlement, shutdown-before-drain order, retirement proof, resource contract or verification changes |
| [knowledge/pwright-handle-operations.md](knowledge/pwright-handle-operations.md) | Handle-local operations, property snapshots and acquisition ownership | Property/result semantics, acquisition/cleanup, batch ownership, resource contract or verification changes |
| [knowledge/pwright-improvements.md](knowledge/pwright-improvements.md) | pwright Improvements | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-locator-focus.md](knowledge/pwright-locator-focus.md) | Pwright Locator focus and blur design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-locator-evaluation.md](knowledge/pwright-locator-evaluation.md) | Locator evaluation, paired preparation, host-owned continuation, Node conversion rendezvous and native retirement receipts | Resolution/adoption deadlines, route/proof retention, world destruction versus session-object retirement, host lifecycle, async storage bounds, Node roots/exception/binary timing or verification changes |
| [knowledge/pwright-locator-press.md](knowledge/pwright-locator-press.md) | Locator press, retained Frame focus / Page keyboard selection, cancellation and native detach lifetime discrepancy | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-sequential-typing.md](knowledge/pwright-sequential-typing.md) | Public Rust-backed typing, opt-in Page input policy and artifact-specific regression evidence | Typing semantics, bounds, cancellation, modifier fields, wire/FFI integration or oracle evidence changes |
| [knowledge/pwright-raw-keyboard.md](knowledge/pwright-raw-keyboard.md) | Keyboard contracts, bounded services, controlled dispatch, retained Page input and public Rust host/Node API | Named key resolution, schemas/generated adapters, Page routing/revocation, shared input state, FFI exposure, budgets, dependency proofs or oracle evidence changes |
| [knowledge/pwright-raw-mouse.md](knowledge/pwright-raw-mouse.md) | Raw Mouse, six-command protocol, Page-selected multi-phase dispatch, standalone button retirement, retained Page/runtime invocations, host retirement, bounded first-poll ordering and public Node facade | Mouse semantics, drag delegation, Page provider selection/teardown, live-state reader retention, schema/adapter ownership, validation, concurrency, scheduling, native/route/provider/FFI accounting, cancellation identity, receipt delivery, host/Worker shutdown, Node API or artifact evidence changes |
| [knowledge/pwright-locator-screenshot.md](knowledge/pwright-locator-screenshot.md) | Playwright Locator screenshot composition | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-file-input.md](knowledge/pwright-file-input.md) | Rust file-input preparation, native/payload locality and Node upload implementation | File-input source/target semantics, ownership, resource policy, selected dependency closure or verification changes |
| [../crates/pwright-file-input-fs/README.md](../crates/pwright-file-input-fs/README.md) | Optional file-input filesystem provider and pinned MIME data | Filesystem API, supported platform, MIME licensing or verification commands change |
| [../crates/pwright-file-input-tokio/README.md](../crates/pwright-file-input-tokio/README.md) | Admitted blocking file-input executor | Queue/source admission, cancellation, drain, runtime or verification requirements change |
| [../crates/pwright-backend-cdp-file-input/README.md](../crates/pwright-backend-cdp-file-input/README.md) | Chromium file-input mutation and live validation | DOM semantics, encoding/admission, native ownership, licensing or verification commands change |
| [../crates/pwright-composition-chromium-file-input/README.md](../crates/pwright-composition-chromium-file-input/README.md) | Single-use upload continuation and cleanup | Operation admission, settlement, cancellation, cleanup ownership or verification changes |
| [../crates/pwright-file-input-source/README.md](../crates/pwright-file-input-source/README.md) | Optional file-input content ownership | Materialization API, ownership or execution requirements change |
| [knowledge/pwright-locator-scroll-into-view.md](knowledge/pwright-locator-scroll-into-view.md) | Pwright Locator scroll-into-view design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-locator-select-option.md](knowledge/pwright-locator-select-option.md) | Pwright Locator select-option design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-locator-set-checked.md](knowledge/pwright-locator-set-checked.md) | Pwright Locator set-checked design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-network-lifecycle.md](knowledge/pwright-network-lifecycle.md) | Pwright network lifecycle | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-extra-http-headers.md](knowledge/pwright-extra-http-headers.md) | Optional Rust Context/Page header policy, ownership, native installation design and pinned comparison | Header merging/referrer rules, budgets, native fanout/rollback, FFI selection or verification changes |
| [knowledge/pwright-object-model.md](knowledge/pwright-object-model.md) | Pwright browser object-model ownership | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-frame-publication.md](knowledge/pwright-frame-publication.md) | Retained Frame metadata, bounded tree reads/bindings, public collections and shared event-loop publication | Frame identity, parent/detach/close behavior, host/Node integration, JS cache ownership, publication sequencing, resource contracts or paired evidence changes |
| [knowledge/pwright-frame-lookup.md](knowledge/pwright-frame-lookup.md) | Version-pinned Frame lookup semantics, Rust/FFI design and implementation gates | Oracle version, matcher behavior, traversal/cache ownership, callback/exception handling, budgets, Context baseURL/order dependencies or verification changes |
| [knowledge/pwright-frame-navigation.md](knowledge/pwright-frame-navigation.md) | Frame-native routing/initiation, lifecycle/FFI design and pinned local/OOPIF evidence | Route/session identity, command schemas, navigation waits/errors/cancellation, resource ownership, public Frame.goto or differential evidence changes |
| [knowledge/pwright-page-close.md](knowledge/pwright-page-close.md) | Playwright-aligned Page close design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-close-cascade.md](knowledge/pwright-close-cascade.md) | Concurrent native close intents, ancestor retirement and serialized publication design | Native close scheduling, reservation/transaction ownership, ancestor proof, Context interruption semantics or differential evidence changes |
| [knowledge/pwright-node-lifecycle-events.md](knowledge/pwright-node-lifecycle-events.md) | Node lifecycle callback ordering, red differential evidence and stepwise Rust publication design | Event arguments/state/order, Rust publication ownership, FFI callback/admission, artifact evidence or implementation status changes |
| [knowledge/pwright-page-content.md](knowledge/pwright-page-content.md) | Playwright Page content semantics and Pwright boundary | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-page-reload.md](knowledge/pwright-page-reload.md) | Typed reload, Node FFI, bounded host/raw-CDP close controls and loader/request correlation | Reload behavior, ownership, wire/API exposure, diagnostic bounds or paired evidence changes |
| [knowledge/pwright-page-creation.md](knowledge/pwright-page-creation.md) | Playwright-aligned Page creation design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-process-memory-metrics.md](knowledge/pwright-process-memory-metrics.md) | Pwright process-memory attribution | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-playwright-server-connect.md](knowledge/pwright-playwright-server-connect.md) | Pinned Playwright Server `BrowserType.connect` protocol map, opt-in client/header/Page-event and close-only boundaries, and fixture-adapted diagnostic evidence | Remote wire protocol, header policy, Page-event or close-only projection, language adapters, lifecycle ownership, close semantics or official connect evidence changes |
| [knowledge/pwright-python-connect-refused-strict.md](knowledge/pwright-python-connect-refused-strict.md) | Official Python closed-endpoint `BrowserType.connect` fixture-adapted evidence scope | Fixture adaptation, exact six-case selection, sandbox, source pins, receipts or paired result status changes |
| [knowledge/pwright-protocol-idl.md](knowledge/pwright-protocol-idl.md) | Pwright protocol IDL decision record | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-screenshot.md](knowledge/pwright-screenshot.md) | Playwright screenshot semantics and Pwright boundary | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-tracing.md](knowledge/pwright-tracing.md) | Pwright tracing boundary | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-ws-logging-security-design.md](knowledge/pwright-ws-logging-security-design.md) | Namespaced WebSocket logging fork, upstream provenance, TRACE/TLS proof and release gaps | Transport dependency, log exposure, TLS validation, fork packaging or security verification changes |
| [knowledge/pwright-value-evaluation.md](knowledge/pwright-value-evaluation.md) | Playwright-aligned value evaluation design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/recipe-catalog.md](knowledge/recipe-catalog.md) | Recipe Catalog | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/recipes-design.md](knowledge/recipes-design.md) | Recipes Design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/script-runner-design.md](knowledge/script-runner-design.md) | Script Runner Design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/testing-strategy.md](knowledge/testing-strategy.md) | Testing strategy, bounded lifecycle tracing and per-case/aggregate supervision | Capability, ownership, resource contract, supervisor limits or verification evidence changes |
| [knowledge/url-capture-design.md](knowledge/url-capture-design.md) | Raw URL Capture Design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [known-issues.md](known-issues.md) | Known Issues | Behavior, API, or architecture described in this document changes |
| [lightpanda-compat.md](lightpanda-compat.md) | Lightpanda Browser Compatibility | Behavior, API, or architecture described in this document changes |
| [node-ffi-compatibility-design.md](node-ffi-compatibility-design.md) | Playwright-compatible Node FFI design and bounded attachment/retained-world diagnostics | Node facade, Rust host/addon, runtime extension/lifecycle, compatibility, dependencies, attachment pressure, retained-world observations or differential evidence changes |
| [python-ffi-compatibility-design.md](python-ffi-compatibility-design.md) | Playwright-compatible Python/PyO3 boundary, packaging, ownership and differential verification plan | Python facade, PyO3 ABI, Rust host integration, capability profiles, resource limits, packaging, cleanup or differential evidence changes |
| [python-official-conformance.md](python-official-conformance.md) | Pinned official Playwright Python corpus and exact-source oracle/candidate lane | Upstream pin, source digest, selected node IDs, public/private inventory, harness isolation or official-test evidence changes |
| [upstream-playwright-conformance.md](upstream-playwright-conformance.md) | Full pinned Playwright core JS/TS and Python inventory, completion rule, ledgers and execution state | Upstream pins, suite/config inventory, expanded counts, exclusions, ledger hashes, candidate/oracle outcomes, selected implementation gates, wheel verification evidence or completion criteria change |
| [recipes-for-agents.md](recipes-for-agents.md) | Pre-compiled Browser Skills for AI Agents | Behavior, API, or architecture described in this document changes |
| [resource-boundaries-v2.md](resource-boundaries-v2.md) | Pwright v2 resource ownership and limits | Behavior, API, or architecture described in this document changes |
| [tech-debt.md](tech-debt.md) | Tech Debt | Behavior, API, or architecture described in this document changes |
| [todo.md](todo.md) | TODO | Behavior, API, or architecture described in this document changes |
| [Node package README](../packages/pwright/README.md) | Local native build, supported public subset, screenshot ownership and differential commands | Package scripts, native-profile selection, loading, API coverage, supported targets, resource contracts or test setup changes |
| [Python package README](../packages/pwright-python/README.md) | Base PyO3 wheel build, supported async/sync slice, ABI/profile and verification commands | Python package layout, loading, API coverage, capability profiles, resource contracts, wheel reproducibility, official corpus selection or test setup changes |
| [Shared comparison README](../tests/README.md) | Shared Node-FFI paired corpus, legacy golden lanes, report classifications and oracle diagnostics | Shared adapters, suite catalog, fixtures, report schema, commands, limits or verification evidence changes |
| [MANIFEST.md](MANIFEST.md) | Documentation update routing | A document is added, removed, or changes responsibility |
