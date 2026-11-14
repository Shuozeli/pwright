<!-- agent-updated: 2026-10-21T03:43:07Z -->

Current full-corpus execution coverage checkpoint (2026-10-21):

- [x] Audit frozen Core v22 against all 48,477 reviewed invocation classes
  without editing the immutable paired ledger. The
  [reconciliation report](../.work/official-conformance/audits/core-v22-reconciliation-v1.json)
  pins every input SHA-256 and identifies 2,610 source-only `expected` rows
  per lane. Its 19 reviewed-public rows (nine plain public and ten public with
  a private test helper) remain applicable but runtime-unproven; do not count
  them as executed Pwright passes or silently exclude them. Eight focused
  audit tests pass. No v22 outcome changes or parity credit follow.

- [x] Recollect the full 6,501-ID Python `core` profile on the rebuilt
  response-security wheel and an isolated official Oracle environment.
  Both source-bound provenance sidecars verify and import the intended
  candidate shim/wheel versus official package. This is collect-only:
  JUnit records zero executed cases, the 25 TodoMVC examples remain a
  separate profile, and strict v10 outcomes receive no new credit.

- [x] Run the complete pinned 13-case Core Chromium
  [frame-hierarchy manifest](../tests/official/playwright-core-frame-hierarchy-public-chromium-v1-manifest.json)
  through source-bound dedicated-CDP candidate and Oracle lanes. The
  [candidate](../.work/official-conformance/results/core-frame-hierarchy13-rust-owner-candidate-20261021-v1.json)
  passes 8/13 and the
  [Oracle](../.work/official-conformance/results/core-frame-hierarchy13-oracle-20261021-v1.json)
  13/13; both provenance sidecars verify and the source/ID/asset checker is
  4/4. Four REDs are explained by the frozen `frame-element` addon/profile
  lacking opt-in `frame-events` publication: dynamic/frameset attach counts,
  same-document navigation event, and reattachment waiter. The fifth fails
  during its first `page.goto` before event assertions; the associated network
  Request-frame/topology error has no established causal trace. Keep these
  failures separate. The independently replayed immutable
  [v22 paired receipt](../.work/official-conformance/outcomes/core-frame-hierarchy13-rust-owner-strict-paired-v22/promotion-receipt.json)
  (SHA-256 `535148fa2b02b5af9e5519604cebef296f6e7daa1ff3c43dcba79139f933270a`)
  changes exactly 13 rows per lane, preserves 48,464 other raw rows per lane,
  inherits the v21 frame-element instability, and passes 48 verifier/mutation
  tests. Current Core totals are candidate 4,674 expected / 87 unexpected /
  21 skipped / 43,695 not-run; Oracle 4,452 / 11 / 20 / 43,994. No parity.
  The new default-off `frame-events` bridge and separate `frame-detach-events`
  pass Node 225/225 and host release 598 passed / eight ignored. A corrected
  explicit combined profile includes `frame-element` too (frozen addon
  SHA-256
  `1681505babfcf49f0e62233537f471af911fd73c247cef284ced648543c8c332`).
  The first profile accidentally omitted it: full hierarchy 13/13 but
  frameElement 0/6 at its capability gate. The corrected snapshot's complete
  official attempts are hierarchy 10/13 then 12/13, and frameElement 5/6
  then 6/6. All six new raw source-bound sidecars verify. The repeated
  pre-assertion `TargetClosed` and network-frame/topology teardown error need
  a traced cause and stability fix before any strict promotion or parity
  claim; preserve v22 and every raw attempt.
  Four focused release characterization tests now prove both independent
  network/topology cursor orderings can leave an available Request Frame absent
  from live topology, and that terminal effects retain the creation descriptor.
  The Node `Request.frame()` detached-wrapper gap is fixed narrowly for a
  previously known canonical wrapper; the focused suite is 87/87 with no
  TODO. Network-before-topology publication and the browser `TargetClosed`
  cause remain separate open work, with no strict promotion.
  The next complete 11-case Core Chromium Library Context page-event file is
  source/ID/asset-pinned (checker 4/4) and has two full runs per lane. Oracle
  is 6/11 twice under the dedicated-CDP launch adapter; the same frozen
  candidate is 10/11 then 9/11. All four provenance sidecars verify. The
  opener case times out in both lanes, while candidate Ctrl-click fails only
  on the second run. Do not promote this file from an Oracle-red, unstable
  candidate pair; investigate the fixture and CDP hang separately.
  Python
  [sync network manifest](../packages/pwright-python/tests/official/sync-network-full-v1-manifest.json)
  pins all 18 browser-expanded IDs from six upstream functions; its checker
  passes 3/3. The six Chromium cases ran source-bound: usable
  [candidate v2](../.work/official-conformance/results/python-sync-network6-chromium-candidate-20261021-v2.xml)
  1/6 versus [Oracle](../.work/official-conformance/results/python-sync-network6-chromium-oracle-20261021-v1.xml)
  3/6. The earlier [candidate v1](../.work/official-conformance/results/python-sync-network6-chromium-candidate-20261021-v1.xml)
  had six setup errors from the CDP Page-lane capacity limit and is retained
  as infrastructure-invalid evidence, not six compatibility failures. All
  three provenance sidecars verify. Oracle itself fails the loopback-only
  `server_addr` assertion under required Tailscale fixture binding and two
  service-worker cases; do not count those as candidate-only. Two
  security-details cases are candidate-only REDs. The immutable
  [Python v10 paired receipt](../.work/official-conformance/outcomes/python-sync-network6-chromium-strict-paired-v10/promotion-receipt.json)
  (SHA-256 `c696c1cab0976ac45564c54814ebf12199268a904f243672203c1ababd9f7f13`)
  updates only six rows per lane and preserves 6,520 other rows per lane;
  44 verifier/mutation controls pass. Current Python totals are candidate
  1,651 passed / 124 failed / three errors / 37 skipped / 4,711 not-run;
  Oracle 1,436 / 13 / 37 / 5,040. Twelve non-Chromium IDs remain unrun.
  A separate source-pinned
  [Oracle-only secure-fixture diagnostic](../.work/official-conformance/results/python-sync-network-sw2-secure-chromium-oracle-20261021-v1.xml)
  passes both Service Worker cases over trusted Tailscale HTTPS (2/2), with
  independently verified provenance. A separate pinned
  [paired secure diagnostic](../packages/pwright-python/tests/official/sync-network-service-worker-secure-chromium-paired-v1-manifest.json)
  reports candidate 1/2, Oracle 2/2, both source-bound. Candidate passes the
  response-from-Service-Worker case; BrowserContext `serviceworker` events
  remain missing. Keep the original six-case HTTP result and v10 receipt
  unchanged; this diagnostic is not ledger promotion.
- [x] Expose Rust-owned `frameattached` and subframe/same-document
  `framenavigated` through a separate default-off addon feature and stable
  Frame identity mapping, with detach as a separate opt-in.
- [ ] Trace the exact event/effect behind the pre-assertion `page.goto` close;
  implement a bounded, generation-safe Request Frame identity across the two
  cursors and retain the canonical Node Frame wrapper after detach. Rerun both
  complete upstream files repeatedly before any stable parity claim.
- [ ] Complete `:light` selector coverage as separately composable, default-off
  capabilities. The first `light-selectors` feature and the new
  `official-conformance-page-check-light-v1` native profile leave the frozen
  v1 selector codec and former page-check profile unchanged. A fresh immutable
  [candidate](../.work/official-conformance/results/core-selectors-misc21-light-candidate-20261021-v1.json)
  ran the same 21 pinned `selectors-misc.spec.ts` cases once: 11/21 passed,
  versus 10/21 in the old candidate and 21/21 in the unchanged Oracle. The
  only changed ID is `should work for open shadow roots`, which exercises
  `id:light` and `data-testid:light` under `page.$`/`$$`; its source-bound
  provenance verifies. Ten candidate-only REDs remain. Do not generalize this
  result: the v20 frozen addon does not contain the later Rust-only
  `data-test-id:light` and `data-test:light` alias extension. The live opt-in
  compiler now has all four upstream attribute aliases, with release selector
  tests 92/92 enabled and 87/87 default-off, including quote, backslash and
  newline escaping; no new browser artifact or official rerun verifies the
  added aliases yet. `locator.count()`, `$eval`/`$$eval`, and
  ElementHandle-scoped queries still reject
  the new BrowserPlan. The browser's `querySelectorAll` result-array size and
  cleanup under high cardinality are not yet bounded/proved; add escape,
  scope, >64-match, cancellation and profile-isolation gates before broad use.
  The independently replayed immutable
  [v20 candidate-only receipt](../.work/official-conformance/outcomes/core-selectors-misc21-light-strict-paired-v20/promotion-receipt.json)
  (SHA-256
  `2889851a3db6106bb726018ade7b8f2f96e72852496d40123ea825a80f3e5bd9`)
  changes exactly one candidate row, preserves the other 48,476 candidate
  rows and all 48,477 Oracle rows byte-for-byte, and passes 39 verifier/
  mutation tests. At v20, Core totals were candidate 4,660 expected / 88
  unexpected / 21 skipped / 43,708 not-run; Oracle 4,439 / 11 / 20 / 44,007.
- [ ] Verify and stabilize the new composable Rust-owned parent-frame DOM
  owner lookup and thin JS/TS `Frame.frameElement()` facade. The complete
  pinned six-case Core
  Chromium [manifest](../tests/official/playwright-core-frame-frame-element-public-chromium-v1-manifest.json)
  has a 3/3 source/ID/asset checker; all six were `not-run` in v18. The
  unchanged source-bound [candidate](../.work/official-conformance/results/core-frame-frame-element6-candidate-20261021-v1.json)
  fails 0/6 and [Oracle](../.work/official-conformance/results/core-frame-frame-element6-oracle-20261021-v1.json)
  passes 6/6. Every candidate case first fails at missing
  `Frame.frameElement()`; later identity, detach, frameset, and shadow-root
  assertions remain untested in candidate. Follow the pinned Chromium
  delegate: obtain the owner backend node from the **parent** CDP session,
  recheck parent attachment, then adopt into the parent main world and publish
  a bounded retained `ElementHandle`; do not substitute parent CSS lookup,
  which cannot reach closed shadow roots. Handle `Frame has been detached.`,
  native compensation on cancellation, exact Page/session/frame identity and
  v1 compatibility. The independently verified immutable
  [v19 paired receipt](../.work/official-conformance/outcomes/core-frame-frame-element6-strict-paired-v19/promotion-receipt.json)
  (SHA-256
  `9303f3e34a31f15b92f9f834b734392c88b8589c5628a3a118ba379d3d92ab4b`)
  changes exactly six target rows per lane and preserves 48,471 other raw
  rows per lane byte-for-byte. At v19, Core totals were candidate 4,659
  expected / 89 unexpected / 21 skipped / 43,708 not-run and Oracle 4,439 /
  11 / 20 / 44,007. A new default-off `frame-element` Rust/Node implementation
  and `official-conformance-page-frame-element` profile now pass release
  backend 5/5, composition 4/4, Node adapter/contract 190/190, native load,
  and TS checks. Its source-consistent frozen addon is SHA-256
  `440ec056a5cfceeacbf851c698ee24c33c3db063ad92262c215d996a568db6a0`.
  The unchanged complete-file candidate [first run](../.work/official-conformance/results/core-frame-frame-element6-rust-owner-candidate-20261021-v1.json)
  was 5/6: frameset failed during `page.goto` before `frameElement()`.
  Identical-artifact full [v2](../.work/official-conformance/results/core-frame-frame-element6-rust-owner-candidate-20261021-v2.json),
  [v3](../.work/official-conformance/results/core-frame-frame-element6-rust-owner-candidate-20261021-v3.json),
  and [v4](../.work/official-conformance/results/core-frame-frame-element6-rust-owner-candidate-20261021-v4.json)
  runs were each 6/6; all four provenance sidecars verify. This is observed
  intermittent navigation failure, not yet proven stable parity. Keep all
  raw reports and diagnose the cause before a stable-parity claim. The
  independently replayed immutable
  [v21 candidate-only receipt](../.work/official-conformance/outcomes/core-frame-frame-element6-rust-owner-strict-paired-v21/promotion-receipt.json)
  (SHA-256
  `3f717d16e9cdca75e601ecdcea8185c82df9c42a6c4beebb94cb67ea76f5324d`)
  enforces all four raw runs/provenances, upgrades exactly six candidate rows
  at five cumulative attempts each, preserves the other 48,471 candidate
  rows and all 48,477 Oracle rows byte-for-byte, and passes 47 verifier/
  mutation tests. It explicitly marks frameset unstable, root cause unknown,
  and public-API parity false. At v21, Core totals were candidate 4,666
  expected / 82 unexpected / 21 skipped / 43,708 not-run; Oracle 4,439 / 11
  / 20 / 44,007. Next, add bounded internal navigation-close source tags
  and a controlled old-context-close/new-page-goto barrier test; do not infer
  that tab capacity or fixture overlap caused the first failure without trace.
- [x] Run the complete pinned 21-case Core Chromium
  `selectors-misc.spec.ts` file in isolated dedicated-CDP lanes. Its
  [manifest](../tests/official/playwright-core-selectors-misc-public-chromium-v1-manifest.json)
  and two checker tests verify all source/ID-pinned public-behavior cases;
  they were `not-run` in both v17 ledgers. The unchanged complete-file
  [candidate](../.work/official-conformance/results/core-selectors-misc21-candidate-20261021-v1.json)
  passes 10/21 and the
  [Oracle](../.work/official-conformance/results/core-selectors-misc21-oracle-20261021-v1.json)
  21/21. Each case ran once, with no skipped/flaky or top-level report errors;
  both source-bound provenance sidecars independently verify. The 11
  candidate-only REDs remain. The independently verified immutable
  [v18 paired receipt](../.work/official-conformance/outcomes/core-selectors-misc21-strict-paired-v18/promotion-receipt.json)
  (SHA-256
  `0bcdeef5540013619a32bc6e5eabf9f456b2b7195dd5f05d2b1bb3e17054c261`)
  changes exactly 21 target rows per lane and keeps the other 48,456 raw rows
  per lane byte-identical to v17. At v18, Core totals were candidate 4,659
  expected / 83 unexpected / 21 skipped / 43,714 not-run and Oracle 4,433 /
  11 / 20 / 44,013. The 11 new REDs remain explicit; this is not broad parity.
- [ ] Triage and repair the remaining ten candidate-only
  `selectors-misc.spec.ts` REDs without treating them as one feature. The
  original 11-RED report exposed `id:light`
  parsing, layout pseudo-class, browser-program first/all-match gates, XPath
  relative/pipe/error semantics, and `internal:has`/`has-not`/`or`/`chain`
  composition. The shadow-link click times out; trace its actual cause before
  assigning it to parser, action or navigation. Preserve raw reports and
  compare each rerun to the unchanged Oracle.
- [x] Run the complete Core Chromium `selectors-css.spec.ts` file in
  isolated candidate/Oracle lanes. Its source-bound
  [manifest](../tests/official/playwright-core-selectors-css-public-chromium-v1-manifest.json)
  and nine checker/mutation tests pin all 27 public-behavior IDs; only the
  default `deep-shadow.html` asset is referenced. All 27 unchanged tests ran
  once per lane on dedicated Chrome 155: the frozen clean
  [candidate](../.work/official-conformance/results/core-selectors-css27-candidate-20261021-v1.json)
  passes 14/27 and the
  [Oracle](../.work/official-conformance/results/core-selectors-css27-oracle-20261021-v1.json)
  27/27. Both provenance sidecars pass independent source-bound verification;
  there are no skipped/flaky cases or top-level report errors. The 13
  candidate-only REDs remain unresolved. The independently verified immutable
  [v17 paired receipt](../.work/official-conformance/outcomes/core-selectors-css27-strict-paired-v17/promotion-receipt.json)
  (SHA-256
  `8edfeb4781c19bdc2e9f1eddb5ebf7f9f7863396972a1a69f9c59e0cfd70fd18`)
  updates exactly 27 target rows per lane and keeps 48,450 other raw rows per
  lane byte-identical to v16. At v17, Core totals were candidate 4,649
  expected / 72 unexpected / 21 skipped / 43,735 not-run and Oracle 4,412 /
  11 / 20 / 44,034; all 13 candidate-only REDs remain explicit. The fixture
  is adapted CDP and no full-corpus or unadapted parity claim follows.
- [ ] Triage and repair the 13 candidate-only `selectors-css.spec.ts` REDs.
  Preserve the raw 14/27 result, avoid weakening the upstream assertions, and
  keep four independently audited causes separate: seven cross-shadow CSS
  ancestor/combinator/pseudo-class failures (test starts 77, 123, 301, 364,
  378, 392, 406), four scoped/relative/sibling-selector failures (224, 308,
  332, 450), one capture+XPath composition unsupported gate (152), and one
  `$eval` utility/main-world mismatch (180). The line-77 test actually fails
  on source line 82, `css=div div span`; earlier escaped-attribute assertions
  pass. Both lanes share pinned test source, project, endpoint, and fixture
  bind/host, with no demonstrated fixture-caused discrepancy. This is
  read-only diagnosis, not a passing rerun.
- [ ] Add an independent, default-off Playwright CSS composed-tree selector
  leaf. The current runtime's per-root native `querySelectorAll(selector)`
  cannot match a single compound selector across shadow hosts and mishandles
  scoped/sibling/relative selectors. Keep the existing basic CSS leaf. A
  proposed `css-composed` leaf parses bounded selector AST in Rust (not in the
  backend-neutral model), then evaluates via a versioned utility-world helper
  with composed-parent/sibling semantics, `:has` scope rebinding, deduplication
  and composed order; no arbitrary JS or main-world fallback. Reuse the
  existing Rust `>>` chain scanner rather than splitting strings again. Walk
  DOM/open-shadow candidates incrementally instead of materializing
  `querySelectorAll('*')` for the whole page. Add shared work limits for
  tokens, arms, pseudo nesting, scanned elements, shadow roots, recursive
  `:has`, candidates, result count and deadline beyond the current 64 KiB /
  32-depth / 256-node selector input budget; exhaust explicitly. A single
  native `Element.matches()` call remains a synchronous CPU caveat.
  The smallest safe first slice recognizes only leading child combinator
  `>span` with an Element root, compiling a proven `:scope > span` form;
  reject document/shadow/`+`/`~`/nested cases until the evaluator exists.
  This may unblock one assertion but not necessarily the entire official
  line-450 case, which also uses `:has(> span)`. Prove the seven cross-shadow
  plus four scoped cases separately before repeating all 27.
- [ ] Add a separate default-off browser-program capture composition path for
  `*css=li >> ../.. >> text=Hello`; ordinary same-node capture already works.
  Do not label all capture selectors absent or silently route this through an
  unbounded page script.
- [ ] Align `Page/Frame.$eval` user callbacks with upstream main-world
  execution through Rust-owned utility-world selection, exact element
  adoption to main, callback, and handle release. The current
  `CollectionMode::One` runs both selection and callback in utility world;
  source line 180's `window.div` identity RED is a strong code-path inference,
  not a traced proof. Do not move the entire selector runtime into page main
  world. The existing persistent Handle route cannot simply adopt across
  worlds: its Host profile deliberately lacks element-adoption capability and
  returns `UnsupportedCapability`. The temporary Locator adoption route has
  cleanup, but its strict waiting resolver rejects multiple matches. First
  add a separate default-off owned first-match resolver (zero returns no
  element; one/two select first) that composes with temporary utility-to-main
  adoption, main callback, and confirmed release. Cover zero/one/two matches,
  identity, main-world DOM-query monkey-patching, cancellation and cleanup
  failure in fake CDP AAA; rerun the exact case, existing official 26-case
  `$eval` and nine-case `$$eval` slices, then CSS27. No code change or new
  passing evidence exists for this path yet.
- [x] Pin and execute the complete 10-case Core Chromium
  `page-dialog.spec.ts` file in source-bound candidate/Oracle CDP lanes. The
  fixed manifest/source checker passes 8/8; the frozen candidate passes 9/10
  and the Oracle 10/10. One candidate-only RED remains at source line 67,
  where an open alert blocks until the 30-second test timeout. The immutable
  [v15 paired receipt](../.work/official-conformance/outcomes/core-page-dialog10-strict-paired-v15/promotion-receipt.json)
  independently verifies exactly ten changed rows per lane and leaves 48,467
  other raw rows per lane intact; candidate cumulative totals are 4,634
  expected / 60 unexpected / 21 skipped / 43,762 not-run, versus Oracle
  4,385 / 11 / 20 / 44,061. This is adapted Chromium evidence, not full parity.
- [x] Diagnose and repair the candidate-only open-alert dialog evaluation
  ordering in the existing default-off evaluation capability, then rerun the
  complete unchanged official file in the candidate lane. Preserve the
  original 9/10 RED and Oracle 10/10 results; do not claim a fresh Oracle run.
  A trace-only build with an upstream-shaped synchronous non-Promise path
  still times out. In its second trace, initial `Runtime.callFunctionOn`
  returns a primitive string with no result/exception `objectId`; alert opens
  +21 µs later; `Runtime.releaseObjectGroup` sends at +24 µs but receives no
  reply, Driver Fence, or JS settlement before timeout. This demonstrates
  that awaiting release blocks this corrected candidate case; the old async
  wrapper's role in the historical frozen RED remains unproven. A narrow
  no-release fast path now applies only to an inline string with no remote
  `objectId` anywhere and no `exceptionDetails`; retained/exception paths
  keep awaited cleanup. The clean release addon passes 21 Rust and 15 Node
  tests, and its source-bound
  [official candidate rerun](../.work/official-conformance/results/core-page-dialog10-candidate-sync-clean-20261021-v1.json)
  passes 10/10, including line 67. The independently verified immutable
  [v16 successor](../.work/official-conformance/outcomes/core-page-dialog10-strict-paired-v16/promotion-receipt.json)
  (SHA-256
  `60277dcb60601de2f890c8f3ced2d8475363d0aa9c1fcffab65c90d7e5df5c1f`)
  changes only that historical candidate RED row; 48,476 other candidate
  rows and all Oracle JSONL/summary bytes remain unchanged. At v16, Core
  totals were candidate 4,635 expected / 59 unexpected / 21 skipped /
  43,762 not-run; Oracle 4,385 / 11 / 20 / 44,061. This does not imply a
  fresh Oracle run or full-corpus parity.
- [ ] Close three independently audited bounds in the already selected
  `large-evaluation-results` path before claiming memory safety or broad
  evaluation parity. First, retain `Runtime.callFunctionOn` and
  `Runtime.releaseObjectGroup` through cancellation at initial call, plan,
  and read; a dropped future can otherwise retain a browser string of up to
  128 MiB. Second, acquire a suitable large-result admission before the
  initial CDP call, not only after a retained object is returned; eight
  concurrent operations can precede the current one-reader gate. Third,
  enforce the smaller 256 KiB inline-reply budget before Rust JSON parsing,
  including malformed CDP replies. Add cancellation, capacity, and forged
  oversized-reply tests; do not infer safety from an isolated dialog pass.
- [ ] Remove the evaluation runtime's dependency on inherited
  `c.constructor.prototype` before claiming resilience to page-world mutation.
  `Object.prototype.constructor = null` in the same browser realm can make a
  subsequent evaluation fail before user code runs. Add a two-call same-realm
  regression and capture a stable intrinsic without relying on mutable page
  globals. This pre-existed the provisional synchronous dialog change.
- [x] Execute the complete pinned 45-case Core Chromium `Page.fill` file in
  isolated candidate/Oracle lanes. The earlier candidate result 44/45 is
  retained as historical RED; the freshly frozen candidate and Oracle both
  pass 45/45. The immutable
  [v13 paired successor](../.work/official-conformance/outcomes/core-page-fill45-strict-paired-v13/promotion-receipt.json)
  (SHA-256
  `8a10291c3301a8e6e0877387b9d6d279927eba31a1831ef0e3c420353d1c46de`)
  changes only the historical candidate source-line-388 row to `expected`;
  all other 48,476 candidate rows and all 48,477 Oracle rows are unchanged.
  The adapted `connectOverCDP` fixture establishes bounded Chromium browser
  execution, not unadapted public-API parity or complete Core coverage.
- [x] Execute the complete pinned 80-ID Chromium Python
  `tests/async/test_navigation.py` file in isolated candidate/Oracle lanes.
  The original ABI127 [v1 RED receipt](../.work/official-conformance/outcomes/python-navigation-full-chromium-strict-paired-v1/promotion-receipt.json)
  records candidate 57 passed / 23 failed versus Oracle 77 passed / three
  failed. Five Frame cases first pass in the independently verified
  [v2 focused successor](../.work/official-conformance/outcomes/python-navigation-frame-chromium-strict-paired-v2/promotion-receipt.json).
  The latest frozen ABI130-v2 complete-file candidate rerun records **77
  passed / two failed / one synthetic report error**; the immutable
  [v6 full-file successor](../.work/official-conformance/outcomes/python-navigation-full-chromium-strict-paired-v6/promotion-receipt.json)
  (SHA-256
  `928da6d927abd7cf0667b63e88a89eb9fab260fe15ff0c50c04db358a295cb33`)
  independently verifies the 80 target IDs, preserves 6,446 off-target
  candidate rows and all Oracle rows. Candidate cumulative totals are 1,617
  passed / 104 failed / four errors / 37 skipped / 4,764 not run. The earlier
  ABI128 72/7/1 [v3 result](../.work/official-conformance/outcomes/python-navigation-full-chromium-strict-paired-v3/promotion-receipt.json)
  remains sealed. This is fixture-adapted Chrome 155 execution, not public-API
  parity or full-corpus completion.
- [x] Implement Rust-owned forward history under the optional
  `history-navigation` feature and verify the unchanged official
  `test_go_back_should_work_with_history_api[chromium]` on a fresh ABI129
  installed wheel: focused candidate 1/1. The independently verified
  [v4 focused successor](../.work/official-conformance/outcomes/python-navigation-history-api-chromium-strict-paired-v4/promotion-receipt.json)
  (SHA-256
  `541e82e5805207de17299295b9e9dfe3ca4c04c20771f36981b1db5e2ef0c4b6`)
  promotes only that row; cumulative candidate totals become 1,613 passed /
  108 failed / four errors / 37 skipped / 4,764 not run. The other 6,525
  candidate and all Oracle rows are byte-identical to v3. This is not a
  full-file rerun.
- [x] Make URL-matched navigation a single bounded native registration across
  rejected candidates, with one original deadline, same-document URL
  observation, lock-free predicate evaluation, and deterministic cancellation;
  do not re-arm a one-shot Python waiter between events. The three unchanged
  official cases pass 3/3 on the frozen ABI130-v2 wheel; the strict
  [v5 focused receipt](../.work/official-conformance/outcomes/python-navigation-url-match-chromium-strict-paired-v5/promotion-receipt.json)
  is verified and the v6 full-file report retains those passes. Callable
  predicates currently run on a Tokio worker, not necessarily the caller's
  asyncio thread; audit parity before claiming full compatibility.
- [ ] Close the remaining Python navigation reliability gap. The installed
  ABI131 wheel adds default-off `frameattached`/`framenavigated`: the unchanged
  `window.stop()` subframe case passes in a focused source-bound run, but its
  first complete 80-case run records 76 pass / two fail / two synthetic errors
  from `TargetClosedError` plus duplicate teardown reports. A second complete
  run on that same wheel records 78 pass / two fail / zero errors, including
  the subframe case and the other formerly errored case; the two explicit
  failures also occur in the Oracle lane. Its independently verified immutable
  [v9 recheck receipt](../.work/official-conformance/outcomes/python-navigation-recheck-chromium-strict-paired-v9/promotion-receipt.json)
  replaces exactly 80 candidate rows after v8, preserving 6,446 other
  candidate and all 6,526 Oracle raw rows. Current candidate totals are
  1,650 pass / 119 fail / three errors / 37 skip / 4,717 not-run. Preserve
  both source-bound RED reports and investigate the non-reproduced
  target-close race rather than inferring that it cannot recur.
- [ ] Finish gates for the default-off, Rust-owned `frameattached` and
  `framenavigated` implementation. Its installed-wheel Python contract suite
  passes 290/290, but the 30 filtered Rust frame-tree assertions printed
  PASS before their test command was interrupted (exit 130); a subsequent
  clean `cargo test --release -p pwright-node-host --features frame-events
  frame_trees --lib` exits zero with 80 passed and two dedicated-Chrome tests
  ignored. The real-Chrome attach/navigation and close/cancel lifecycle gate
  remains before declaring the capability complete.
- [x] Supply the canonical main Frame as Page navigation `Response.frame`
  without the optional `request-history` feature. This source-only facade
  repair postdates the frozen ABI130-v2 wheel but is included in the ABI131
  installed wheel, whose Python contract suite passes 290/290. It does not
  retroactively alter earlier official reports.
- [ ] Resolve the pinned 50-case Python Chromium `test_network.py` manifest:
  all 50 cases ran in each source-bound HTTP/Tailscale lane (candidate 35 pass /
  15 fail, Oracle 47 pass / three fail). Twelve are candidate-only REDs. A
  [v8 strict paired receipt](../.work/official-conformance/outcomes/python-network50-chromium-strict-paired-v8/promotion-receipt.json)
  promotes exactly those 50 manifest rows per lane, preserves the other 6,476
  raw rows per lane, and still declares neither full-file nor corpus parity. A
  separate trusted-HTTPS two-case Service Worker diagnostic passes 2/2 in the
  Oracle and 1/2 in the candidate; its remaining candidate failure is missing
  `Response.body`. The HTTP `server_addr` assertion hard-codes loopback and
  cannot establish parity on a Tailscale-bound fixture; the candidate also
  lacks `server_addr()`, so it is not a pure fixture-only discrepancy. Keep
  fixture variants and their outcome scopes distinct; do not rewrite the
  50-case raw reports.
- [ ] Repair five candidate-only official network50 header cases with a
  default-off `network-raw-headers` leaf over `network-events`, without
  forcing `request-history`. Two `all_headers()` cases omit browser-added
  wire fields such as `accept`, while Request/Response `headers_array()` and
  Response header readers are absent. Rust should own bounded provenance
  keyed by Page, exact CDP session, native request ID, and redirect hop;
  forward late `requestWillBeSentExtraInfo` and `responseReceivedExtraInfo`
  through the existing ordered event batch, preserving repeated per-name
  values. Add pending-hop, age, byte, count, and FFI-copy limits with explicit
  exhaustion and close/failure retirement. CDP Headers is an object, so do
  not promise byte-exact cross-name line order. Python's early lossy
  `headers` dictionary cannot solve this alone. Audit of the pinned v9 REDs
  found that request ExtraInfo currently produces no publication batch,
  response ExtraInfo has no adapter arm, and the Python addon flattens header
  arrays to a `BTreeMap`; repair the ordered sidecar, raw-owner update fence,
  and opt-in async/sync APIs separately, without forcing `request-history`.
  Account for Python FFI container overhead, not only string bytes. The
  separate `Route.fulfill`
  `net::ERR_FAILED` test may be related but its callback cause is not proven
  by the report, so diagnose it independently.
- [x] Add the first default-off `network-raw-headers` Rust value leaf in
  `pwright-network-model`. It distinguishes CDP extra-info newline-separated
  values from provisional headers and retains repeated array entries. An
  explicit reader policy joins first-seen unique values for Python while
  preserving duplicates for Node, matching their different pinned upstream
  `RawHeaders` implementations. Input, expanded entries, retained strings,
  and FFI copies are bounded. Its release test run is 21/21 (nine raw-header
  tests), default-off model tests are 12/12, all-feature model tests are 71/71,
  and release Clippy passes with warnings denied. This
  is only a value model:
  CDP `ExtraInfo` hop pairing, bounded owner lifetime, codec/host projection,
  and Python/Node readers remain unimplemented. No official network50 outcome
  or cumulative ledger count changes because of this source-only step.
- [ ] Implement a bounded Rust-owned navigation `Response.finished()` owner
  join for Page/Frame goto within the existing default-off
  `response-finished` leaf, without requiring `request-history`. The Python
  async/sync method and 256-slot Rust owner already exist; Page result lacks
  owner projection, Frame projection is incorrectly tied to
  `response-request`, and the weak network map can retire at the terminal
  event before goto returns. Pre-admit a finite navigation join lease (design
  target at most 32 concurrent) and strongly retain the selected **final**
  document response owner through result projection. Correlate Page, exact
  CDP session, native request ID, and redirect hop; add a bounded ordered
  transport sequence if needed because the two CDP event consumers lack a
  shared hop identity. `loadingFinished` and `loadingFailed` after a Response
  should both settle `finished()` to `None`, matching pinned Chromium
  Playwright; pending page/context/session close rejects `TargetClosed`.
  Prove terminal-before-result, result-before-terminal, redirect ID reuse,
  cross-session same ID, no Response, close, and capacity reclamation in AAA
  fakes before exact official reruns. Do not project a missing owner or add an
  implicit dependency merely to make three network assertions green.
  An interim source slice now adds explicit default-off
  `navigation-response-finished` with `request-history` correlation, and its
  isolated release wheel advances one unchanged HTTPS official test beyond
  `finished()` to missing `security_details()` in two complete 1/6 six-case
  runs. This does **not** close the independent-owner, redirect, terminal,
  and close proof above or update the v10 ledger.
- [ ] After a bounded `Response.finished()` owner exists, rerun the two direct
  network lifecycle cases and the two security-detail cases separately. The
  older HTTPS security run first failed on `finished()` and then reached the
  formerly absent `security_details()` API; the HTTP case failed directly
  because that method was absent. The new security source slice is not yet
  official-browser-proved. Do not count blocked assertions as passed.
- [ ] Add a separate default-off `response-security-details` leaf. Preserve
  only CDP's five typed fields (`issuer`, `protocol`, `subjectName`,
  `validFrom`, `validTo`) in a bounded response sidecar, correlate the exact
  Page/session/native request and redirect hop with the final navigation
  Response, and project only the canonical Python camelCase dictionary.
  Distinguish a genuinely observed HTTP response with no TLS metadata from
  an unobserved response; absence of the capability must stay Unsupported.
  Prove malformed and oversize CDP inputs, redirects, close/reuse, bounded
  retention, and both unchanged sync/async official security cases. Do not
  infer HTTPS details or fabricate `None` from a missing event.
  A focused four-ID Chromium sync/async security manifest is now source-,
  fixture-, certificate-, and inventory-pinned (checker 3/3); browser execution
  and both complete-file reruns are still required.
  The independent default-off source slice now has bounded typed CDP, host,
  and Python projections; focused release adapter 30/30, host 2/2, facade
  5/5, and rebuilt installed all-capability wheel contracts 300/300 pass.
  The full addon dependency/release-feature matrix and exact minimal
  security-only closure/compile gate pass. Its wheel
  SHA-256 is `0c09817458bc9329c3bbc7c00153c635ae913d3a58567746a2666ed24fa2d2cc`.
  Keep this task open until the unchanged official sync/async cases and
  complete files pass under a working dedicated CDP endpoint.
- [ ] Design and verify a default-off network-response-body leaf for ordinary
  `Response.body()` / `text()`. The trusted-HTTPS Service Worker RED has
  `body_owner=None` despite the existing navigation-only `response-body`
  feature being selected; only `Route.fulfill` currently supplies an event
  response body owner. Proposed Rust ownership should bind Page, exact CDP
  session and request ID, await the terminal event, lazily read and bound the
  decoded body, and reject redirect/failure/close/cross-session reuse. This
  is a design proposal, not an implemented capability or a passing test.
- [x] Match the official indexed error location for invalid Page extra HTTP
  header values while preserving Context validation. The source-only facade
  fix passes 293/293 browserless contract tests; it has no fresh installed
  official-run credit yet.
- [x] Compile and browser-check the new opt-in Node `Page.goForward()` binding
  against an unchanged official Core HistoryAPI case. The isolated and full
  profile release addons build, 220/220 browserless tests and TypeScript pass,
  and the [candidate](../.work/official-conformance/go-forward-abi129-browser/candidate-history-api.json)
  and [Oracle](../.work/official-conformance/go-forward-abi129-browser/oracle-history-api.json)
  each pass 1/1 on source-bound fixture-adapted Chrome. No Core ledger row
  is promoted. The adjacent cross-document `@smoke` fails in both lanes at
  different steps and remains inconclusive.
- [x] Execute the complete pinned 32-case Core Chromium `Page.selectOption`
  file in paired candidate/Oracle lanes. The immutable v11 receipt preserves
  the candidate's 31/32 navigation-context RED versus Oracle 32/32. A
  default-off `select-option-navigation-commit` profile and a no-Page-binding
  publication-drain guard yield a fresh candidate 32/32; the immutable
  [v12 successor receipt](../.work/official-conformance/outcomes/core-page-select-option32-strict-paired-v12/promotion-receipt.json)
  (SHA-256
  `40c861162bbec094b8729927366000ac3b3026586ce98d16e48dcedb1e6c2dc3`)
  changes only that one candidate row, preserves all Oracle rows, and
  independently replays. The original assertions are unchanged, but the
  `connectOverCDP` fixture is adapted. This is Chromium browser execution,
  not full public-API parity or completion of the 48,477-case Core inventory.
- [ ] Scope Page-binding publication draining to the affected Page. The
  current global registered/pending/early-binding check avoids a retired
  context evaluation when no binding exists anywhere, but an unrelated
  Page's binding may still trigger the same post-navigation evaluation.
- [x] Add a relocatable, fail-closed [Core paired-outcome snapshot packer and verifier](../scripts/package_playwright_core_paired_snapshot.py).
  The locally ignored 16-input package contains the exact historical
  inventory, six manifests, and three generations of candidate/oracle ledgers
  and receipts. Its sidecar SHA-256 is `da9b093b...`; a clean-directory replay
  verifies the receipt chain and all 48,477 rows. A deterministic, ignored
  8,215,934-byte archive has SHA-256 `e5a7ab7a...`; bounded manual USTAR
  extraction, a mandatory external hash pin, and 49/49 focused audit/package
  tests pass. This is historical snapshot integrity, not official re-execution
  or current candidate compatibility. Public distribution still needs a
  provenance-path review and a clean-CI artifact path.
- [x] Reproduce the complete pinned Playwright Python collection from fresh
  Python 3.12 core and hash-locked Python 3.10 examples environments, and add
  the same full-corpus collection/hash check to the local CI workflow.
  All 6,526 expanded node IDs from 156 source files produce the existing
  `3dfb11b6...` inventory SHA-256. This is collection only, with no official
  assertion execution or candidate/oracle outcome promotion; remote CI has
  not run the new step yet.
- [x] Capture and strictly promote 15 exact upstream Python browser-marker
  skips in an isolated
  [strict dual-lane diagnostic](../packages/pwright-python/tests/official/upstream-skip-markers-strict-v1-manifest.json).
  The final-v2 candidate and oracle each record 15/15 `skipped` before test-body
  execution; all 15 pairs independently reverify. The immutable
  [paired successor receipt](../.work/official-conformance/outcomes/python-upstream-skip-markers-strict-paired-v1/promotion-receipt.json)
  has SHA-256 `21c65650...`; each 6,526-row lane changes exactly 15
  `not-run` rows to `skipped`, leaving the other 6,511 row bytes unchanged.
  At that v1 checkpoint, candidate had 4,853 `not-run`/26 `skipped` and oracle
  had 5,187/26; passed counts were unchanged and no public-behavior/parity
  credit was awarded. Six
  capture-verifier and ten promoter mutation controls pass. The local CI
  workflow runs the capture-verifier controls.
- [x] Strictly promote 11 more upstream Python marker skips whose
  `launch_persistent`/`launch_persistent_context` fixtures only supply a
  callable during setup. The unchanged official test bodies never execute:
  all 11 candidate/oracle pairs independently verify setup `skipped` under a
  deny-exec/deny-outbound sandbox and a pre-body guard. The immutable
  [v2 paired successor receipt](../.work/official-conformance/outcomes/python-upstream-skip-callable-fixtures-strict-paired-v2/promotion-receipt.json)
  has SHA-256
  `1b8ce858fa3ab643fb10e8f12142c12b32e3164b8719e2c627d2f6e4a66dbb4a`.
  Exactly 11 more rows per 6,526-row lane change from `not-run` to `skipped`;
  the other 6,515 row bytes and all passed/failed/error counts are unchanged.
  Candidate is now 1,542 passed / 102 failed / three errors / 37 skipped /
  4,842 not-run; oracle is 1,309 passed / four failed / 37 skipped / 5,176
  not-run. Ten promoter and six verifier mutation controls pass. These 26
  cumulative upstream skips earn zero new public-behavior/parity credit.
- [x] Keep the full pending-worktree Python strict-Pyright analysis scope and
  add an [exact debt gate](../scripts/check_playwright_python_pyright_baseline_v1.py).
  The remaining 66 diagnostics in five SHA-pinned historical conformance
  plugins are matched byte-for-byte by a versioned baseline across 91
  analyzed files. The gate now explicitly uses the isolated Python
  interpreter for dependency resolution; 17 mutation controls, local Ruff
  and the pinned local gate pass. The two pinned
  plugins with historical Ruff I001 debt are excluded from that CI lint command
  only while the gate checks their exact source hashes. This is an interim
  regression gate, not zero-error strict typing or a remote-CI green result.
- [ ] Replace the five historical plugins with versioned typed successors and
  recapture/re-attest affected evidence before removing the 66-error baseline
  and restoring unexcluded Ruff checks. Do not edit frozen plugin source in place.
- [ ] Preserve fixture-order semantics for upstream Python skip markers. A
  diagnostic of `test_cdp_session.py::test_should_work[firefox]` fails during
  session-scoped `browser` setup before function-scoped `skip_by_browser` runs;
  the 22 similarly marked CDP-session nodes cannot be counted as skipped from
  source markers alone. No ledger row changed from this diagnostic.
- [x] Run a narrow browserless Core version-mismatch diagnostic with the pinned
  upstream test and `run-server` fixture. Its URL-only v1 pair records oracle
  1/1 and a candidate failure on a fixture-private owner pointer; separate v2
  adds only a hash-pinned test owner adapter and records oracle 1/1 versus a
  candidate generic WebSocket error at the unchanged public assertion. Both
  pairs have immutable raw-runner/envelope evidence, but are fixture-adapted
  diagnostics and grant no Core ledger credit.
- [x] Repair the Playwright Server version-mismatch path without leaking an
  arbitrary remote rejection body through the full-session Node mapping:
  finite 8 KiB-bounded no-Content-Length HTTP 428 capture in the opt-in live
  connector, followed by typed constrained-version parsing. A fresh isolated
  v3 pair of the unchanged official assertion passes oracle 1/1 and candidate
  1/1; its evidence and new addon are hash-pinned, and sealed v2 still verifies.
  Transport 34/34, server-connect 19/19, native mapper 3/3, and focused
  release Clippy pass. This is fixture-adapted diagnostic evidence only, with
  no general WebSocket-redaction claim. A later [immutable paired successor](../.work/official-conformance/outcomes/core-server-connect-version-mismatch-url-owner-strict-paired-v3/promotion-receipt.json)
  (receipt SHA-256
  `5e6180a6b2e0a90d764276af84b933b14b6117cc4dd799a0f925622ea347e1b8`)
  records that single internal `not-run` to `expected` outcome in both
  48,477-row lanes, preserving the other 48,476 raw lines. Independent replay
  and 49 mutation controls pass. This is zero public-API or browser parity
  credit; the upstream fixture starts a remote server but not a browser.
- [ ] Separately audit legacy opt-in WebSocket diagnostic ports for arbitrary
  HTTP rejection-body exposure. The new full-session version-mismatch mapping
  must be sanitized, but `WsError::Display` and older diagnostic ports still
  render a remote response body; do not describe the narrow 428 repair as
  global WebSocket redaction or change frozen error evidence silently.
- [x] Execute the four previously unrun Firefox/WebKit Library
  `browsertype-connect.spec.ts:122` DNS-error invocations in strict paired
  browser-free lanes. Oracle and candidate each pass 4/4; the new immutable
  [v3 successor receipt](../.work/official-conformance/outcomes/core-server-connect-ws-error-other-engines-strict-paired-v3/promotion-receipt.json)
  has SHA-256
  `59c7cbb11ad7284ab8aedadb8c850cd4552ffaa0b035b70430c8b4e2e4ef0ef7`.
  Exactly four `not-run` rows become `expected` per lane, with all 48,473
  other raw lines unchanged. At this predecessor checkpoint, candidate was
  4,548 expected/55 unexpected/21 skipped/43,853 not-run; oracle was
  4,297/8/20/44,152. The common fixture is
  adapted in memory, so the receipt keeps this an implementation-internal
  diagnostic with no public-API or browser parity credit.
- [x] Execute six previously unrun Bidi Library
  `browsertype-connect.spec.ts:122` WS DNS-error invocations across Chromium,
  Chrome stable, and Chrome canary projects, using the pinned Bidi config and
  unchanged upstream assertion. Oracle and candidate each pass 6/6, one clean
  attempt per ID, with no browser launch. The immutable
  [v4 paired successor receipt](../.work/official-conformance/outcomes/core-server-connect-ws-error-bidi-chromium-strict-paired-v4/promotion-receipt.json)
  has SHA-256
  `f7c72c003d4306ade21f077b8b424a60a21d265a72512c5b8ac249c21886d2c6`.
  Exactly six `not-run` rows become `expected` per 48,477-row lane; the other
  48,471 raw lines are unchanged. Candidate is now 4,555 expected / 43,846
  not-run / 21 skipped / 55 unexpected; oracle is 4,304 / 44,145 / 20 / 8.
  Independent replay and 48 focused strict/promoter controls pass locally.
  The fixture adaptation, implementation-internal classification, and zero
  public-API/browser parity credit are explicit.
- [x] Execute the three `connect-over-cdp.spec.ts:572` custom-WebSocket-Close
  invocations selected by the pinned Bidi Chromium/Chrome config. The unchanged
  assertion passes 3/3 in each strict browser-free oracle/candidate lane,
  with one attempt per ID. The immutable
  [v5 paired successor receipt](../.work/official-conformance/outcomes/core-cdp-close-bidi-chromium-strict-paired-v5/promotion-receipt.json)
  has SHA-256
  `13d7df7ccf58fb1838c5f523471a77fadd3b7ad92330d086f13c8d3ac64df068`.
  Exactly three `not-run` rows become `expected` per 48,477-row lane, with the
  other 48,474 raw lines byte-identical to their v4 predecessors. Candidate
  is now 4,558 expected / 43,843 not-run / 21 skipped / 55 unexpected;
  oracle is 4,307 / 44,142 / 20 / 8. Independent replay and 36 focused tests,
  including mutation controls, pass. The test calls `connectOverCDP`; Bidi
  names are project labels only. Its Tailscale fixture adaptation and frozen
  v3 candidate package make this an implementation-internal diagnostic with
  zero public-API, browser, Bidi-protocol, or current-live-package parity
  credit. The sealed v4 capture runner/verifier retain historical
  project-config Ruff E501 debt;
  the three new v5 promotion sources pass strict project-config Ruff. The
  clean-checkout CI step SHA-gates only those two v4 files for E501, strictly
  lints v5 and runs 15 synthetic tests. Full 36/36 and ledger replay remain
  local until a verified clean-CI evidence bundle exists; remote CI has not
  run this step.
- [x] Execute five pinned Bidi Chromium Library browserless
  `browsertype-connect.spec.ts:221`, `:228`, and `:1056` timeout/redirect
  invocations. The unchanged official assertions pass 5/5 in each fresh strict
  oracle/candidate lane, one attempt per ID; two localhost WebSocket literals
  and the common fixture host/bind are adapted. The immutable
  [v6 paired successor receipt](../.work/official-conformance/outcomes/core-server-connect-timeout-redirect-bidi-chromium-strict-paired-v6/promotion-receipt.json)
  has SHA-256
  `43b8f240c16d7c5517fcb16c90f77828c0818552be5e19e368a900356ca266ba`.
  Exactly five `not-run` rows become `expected` per 48,477-row lane; all
  48,472 other raw lines are unchanged. Candidate is now 4,563 expected /
  43,838 not-run / 21 skipped / 55 unexpected; oracle is 4,312 / 44,137 /
  20 / 8. Independent full-ledger and raw-evidence replay passes. The source
  remains implementation-internal and fixture-adapted, with a frozen native
  candidate snapshot and zero public-API, browser, BiDi-protocol, or current-
  package parity credit. A clean-checkout CI tooling step SHA-pins the seven
  manifest/source files, applies exact project-config Ruff without ignores,
  and runs 91 pure focused tests, including mutation controls. It does not
  run the ignored `.work` evidence replay or official tests; remote CI has
  not run.
- [x] Execute the six pinned Bidi Chromium Library browserless
  `browsertype-connect.spec.ts:116` HTTP-401 invocations. Unchanged upstream
  assertions pass 6/6 in each strict oracle/frozen-candidate lane, one attempt
  per ID; only a localhost WebSocket URL host and common fixture bind are
  adapted. The immutable
  [v7 paired receipt](../.work/official-conformance/outcomes/core-server-connect-http-401-bidi-chromium-strict-paired-v7/promotion-receipt.json)
  has SHA-256
  `ef2b8a1d16fd3758c784071e50777df0b35935e04b9c1a3eaa8bf3a60a339f9a`.
  Exactly six `not-run` rows become `expected` per 48,477-row lane; the other
  48,471 raw lines remain byte-identical. Candidate is now 4,569 expected /
  43,832 not-run / 21 skipped / 55 unexpected; oracle is 4,318 / 44,131 /
  20 / 8. Independent full-ledger replay passes. This is
  implementation-internal, fixture-adapted diagnostic evidence only: no
  public-API, browser, BiDi-protocol, or current-package parity credit. The
  clean-CI tooling step hash-pins seven manifest/source files and locally
  passes strict project-config Ruff plus 114 pure focused tests, including
  mutation controls. One additional upstream/snapshot pin test passes locally
  but requires ignored inputs and is explicitly excluded in clean CI. The
  step does not replay ignored `.work` evidence or official tests, and has
  not run remotely.
- [x] Execute six pinned Bidi Chromium Library browserless
  `browsertype-connect.spec.ts:130` custom Close invocations. Unchanged
  official assertions pass 6/6 in each strict oracle/frozen-candidate lane;
  the guard independently records twelve ordered per-project WebSocket
  upgrade/message/Close `4123 / Oh my!` triplets across both lanes, with no
  proven per-spec transport attribution. The immutable
  [v8 paired receipt](../.work/official-conformance/outcomes/core-server-connect-close-error-bidi-chromium-strict-paired-v8/promotion-receipt.json)
  has SHA-256
  `4c0a72a233440263a38bf718fa003fa2e8f8c36ef0243aaac9811c314e14dd68`.
  Exactly six `not-run` rows become `expected` per 48,477-row lane and the
  other 48,471 raw lines remain unchanged. Candidate is now 4,575 expected /
  43,826 not-run / 21 skipped / 55 unexpected; oracle is 4,324 / 44,125 /
  20 / 8. This remains implementation-internal, fixture-adapted diagnostic
  evidence using a frozen addon, with no public-API, browser, BiDi-protocol,
  or current-package parity credit. Strict Ruff and 98 local focused tests
  pass; the clean-CI tooling step runs 97 pure tests, explicitly excluding
  one local upstream/snapshot pin test. Remote CI has not run.
- [ ] Make latest Core ledger integrity independently replayable in clean CI.
  The existing relocatable snapshot captures an older v1 chain. A separate
  v8 ledger-only packer now produces a deterministic 5,870,164-byte ignored
  local archive (SHA-256 `5d457866faaf95b7ed40130282de4ba00c07c1a49160f0c4aecb84614a0d9a95`);
  its independent relocated verifier replays both 48,477-row lanes and the
  exact historical v8 six-row transition. It does not contain the v9 successor.
  The archive includes local absolute paths and is not published or available
  to clean CI. The clean-CI tooling step pins its
  scripts and runs 24 pure tests only. A privacy-reviewed distribution and
  pinned CI row/byte replay gate remain open; this is not official re-execution or
  current-package parity. Full raw-provenance replay is a separate task.
- [x] Capture six public `BrowserType.launch` failure tests under the pinned
  BiDi project configurations in isolated v2 oracle/candidate lanes. The
  joint 12-sidecar verifier passes: all oracles GREEN with a reaped exit-1
  dummy Node child, all candidates RED at the original `Browser logs:`
  assertion because merged `firefoxUserPrefs` is rejected before spawn.
  The earlier v1 Firefox guard-generated apparent pass is excluded. No real
  browser, public-API parity, or BiDi-protocol parity is claimed. Clean CI
  pins the v2 tools and runs 43 pure tests; remote CI has not run.
- [x] Record those six mixed-status public rows in a separate immutable Core
  ledger successor. The independently verified
  [v9 paired receipt](../.work/official-conformance/outcomes/core-dummy-node-bidi-strict-paired-v9/promotion-receipt.json)
  has SHA-256
  `f35fa767281337803ea16ece4a12bb36de52523fe9e348fc7f4467402c3f215f`.
  Only indexes 6,110-6,115 changed: candidate `not-run` to `unexpected`,
  oracle `not-run` to `expected`; the other 48,471 raw rows per lane are
  byte-identical to v8. Candidate totals are 4,575 expected / 61 unexpected /
  21 skipped / 43,820 not-run; oracle totals are 4,330 / 8 / 20 / 44,119.
  The fixture-adapted, frozen-addon receipt has no browser-execution,
  public-API-parity, or BiDi-protocol-parity credit. Clean CI pins the three
  v9 tooling scripts and runs 30 pure focused tests including mutation
  controls; ignored evidence is
  unavailable there and remote CI has not run.
- [x] Implement a narrow default-off `bidi-startup-failure` Node/Rust profile
  for an explicit executable under the three Chromium and three Firefox BiDi
  channel aliases. It accepts the inherited `firefoxUserPrefs` shape without
  applying preferences, prepares an optional trace directory, and delegates
  failure-only process observation to the browser-neutral Rust leaf. Startup
  observation is capped at two seconds and 3 KiB combined logs; Rust owns the
  temporary profile and kills/reaps the process group. It never returns a
  Browser or establishes BiDi. Local gates pass: Rust leaf release 8/8,
  adapter 46/46, native integration 3/3, addon ABI/load smoke, full browserless
  package 829/829, strict TypeScript, and focused addon Clippy. The expanded
  package-boundary check passes locally and proves the selected exact Cargo
  closure adds only its addon marker and browser-neutral startup leaf beyond
  `playwright-trace`; older profiles remain unchanged. Full-dependency strict
  Clippy still hits pre-existing `pwright-node-host` `too_many_arguments`
  warnings at lines 1268 and 1324.
  A browserless selected-profile CI step is added but not remotely run.
- [x] Rerun and independently verify all six unchanged official
  `browsertype-launch.spec.ts:64` launch-failure tests against the new profile
  in isolated candidate/oracle lanes. The
  [strict v3 capture](../.work/official-conformance/results/core-dummy-node-bidi-strict-v3/)
  records 12/12 single-attempt `passed`/`expected` outcomes: six candidate,
  six oracle. The paired verifier checks actual exit-1 dummy Node descendants,
  process/profile cleanup, unchanged assertions, and candidate public
  `BrowserType.launch` admission; Firefox uses the real `--profile` argument.
  This is adapted, failure-only public method evidence, not a usable Browser,
  actual browser execution, broad public-API parity, or BiDi-protocol parity.
  The clean-CI step SHA-pins all six v3 source inputs and passes 58 pure
  mutation controls locally; it Ruff-lints the frozen Python sources but
  cannot project-format them without changing evidence-linked hashes. The
  ignored captures are unavailable there, and remote CI has not run.
- [x] Publish a separately verified immutable Core ledger successor for the
  v3 candidate-GREEN result. The
  [v10 receipt](../.work/official-conformance/outcomes/core-dummy-node-bidi-strict-paired-v10/promotion-receipt.json)
  has SHA-256
  `2ae0478daf066539db4c9904e344dc7c8b102ff65eda8ab9587f2b591f8903e2`.
  Independent post-publication replay checks the six target rows at indexes
  6,110-6,115 and 48,471 byte-identical non-target raw rows per lane.
  Candidate totals become 4,581 expected / 55 unexpected / 21 skipped /
  43,820 not-run; oracle remains 4,330 / 8 / 20 / 44,119. The immutable v9
  receipt and ledgers stay historical candidate-RED; v10 structurally seals
  their bytes because the old v9 verifier cannot live-replay the frozen v2
  sidecars against the new source. The v3 sidecars are replayed live. The
  fixture is adapted and failure-only, with no Browser, broad public-API, or
  BiDi-protocol parity credit. A new clean-CI step pins three v10 tooling
  sources and passes 35 pure tests locally; ignored captures/ledgers cannot
  be replayed there, and remote CI has not run.
- [ ] Resume public-API Python browser-dependent conformance. A read-only
  fixture audit of the latest paired ledger found 4,355 shared
  `public-api-source` `not-run` IDs and no honest next browserless subset;
  this is scouting, not a formal exclusion or test result. Browser-dependent
  execution awaits a verified dedicated Chrome CDP lane.
- [x] Add a clean-checkout, browser-free CI tooling step for the v4 Bidi slice.
  It SHA-pins the five sealed source files requiring narrow Ruff exceptions:
  capture runner and capture verifier E501; capture mutation test I001;
  promoter E501 and I001; promotion mutation test I001. Those exceptions
  cover 24 existing project-config diagnostics only for the exact frozen
  bytes. The independent paired verifier has no exception. Default Ruff
  format checks all six files, and CI executes 35 pure synthetic mutation
  cases; upstream source-pin and full paired-evidence checks remain local
  because ignored .work inputs are not available in clean CI. Remote CI has
  not run this step yet.
- [ ] Replace the five v4 Ruff-debt sources with corrected, versioned v5-style
  capture/promotion inputs, recapture and promote under a new immutable
  receipt, then remove their hash-scoped CI exceptions. Do not edit sealed v4
  source merely to satisfy formatting.
- [x] Add a browser-free, fail-closed [full Core source inventory preflight](../scripts/verify_playwright_core_full_source_inventory.py)
  to the pinned-checkout CI lane. It verifies the exact v1.58.0 commit and
  manifest SHA, 558 tracked spec sources, all 20 declared suites and 48,477
  declared invocations, tracked runner/config inputs, and four explicitly
  non-collected fixture specs; 15 focused controls pass. The generated
  `playwright-test` runner is deferred until dependency installation, so the
  preflight instead verifies its tracked package source. It executes zero
  tests and does not verify paired outcomes.
- [x] Add a pinned, fail-closed read-only [Core adapter coverage report](../scripts/report_playwright_core_adapter_coverage.py)
  with 29/29 focused tests. It reconciles 48,477 rows in 20 suites and 58
  projects with the current Firefox/WebKit RED successor receipt, chained to
  the skip-only and strict-launch predecessors. Of 43,857 candidate
  `not-run` rows, 33,188 are in suites with no configured candidate adapter;
  21,810 of those are public API candidates. Another 7,675 Firefox/WebKit
  `library` rows have only a Chromium-specific preload; 2,994 Chromium
  `library` rows have a configured route but not an execution result. The
  report grants neither applicability exclusions nor official passes.
- [x] Account for ten pinned upstream Core skips per lane in an immutable
  [paired successor](../.work/official-conformance/outcomes/core-upstream-skips-strict-paired-v1/promotion-receipt.json).
  The independent promoter/verifier and 47 focused tests verify exactly ten
  `not-run`→`skipped` changes per lane and no other changed row bytes. The
  receipt states `publicApiParityEvidence=false` and
  `candidateRuntimeAttested=false`; no public method was proven.
- [x] Record strict official browser-free Firefox/WebKit
  `browsertype-launch.spec.ts:64` RED in a new immutable
  [paired successor](../.work/official-conformance/outcomes/core-dummy-node-other-engines-strict-paired-v1/promotion-receipt.json).
  Oracle 1/1 per engine; candidate 1/1 unexpected per engine at the original
  assertion because launch is unsupported. Independent verification and 27
  focused verifier/promoter tests pass. Only these two rows change in each
  48,477-row ledger; candidate runtime/profile/process attestation and
  parity credit are explicitly false.
- [ ] Design and verify protocol-correct candidate and oracle adapters for
  the 19 non-`library` suites before attempting their official runs; the
  Chromium-CDP preload cannot be silently reused for WebDriver BiDi,
  Android, Electron, component, or runner-internal projects. BiDi is the
  largest blocked route: 22,194 candidate `not-run` rows, 17,196 currently
  classified as public API candidates. Its Chromium and Firefox channels
  require separate protocol and browser evidence.
- [ ] Make the pinned full Core inventory, paired ledgers and receipts
  available to clean CI through a verified artifact/input path. They are
  currently local ignored `.work` evidence; the new relocatable snapshot
  verifier and its 8.2 MB compressed archive do not themselves publish a
  verified input to CI. The
  read-only report is not yet a reproducible green clean-checkout outcome
  gate, and neither the source-only preflight nor the local snapshot closes
  this task. Preserve hash pins and receipt chain when distributing the bundle.
- [ ] Resolve the dedicated Chrome CDP endpoint failure before paired
  browser-backed executions. Do not interrupt the existing browser process
  without user approval or attach to a normal Chrome profile.
- [ ] Establish a real-browser/real-server path for Python's 4,842 remaining
  `not-run` rows. Current source/fixture triage found no directly runnable
  browser-free public-behavior row that preserves the original `launch_server`
  fixture. The 26 accounted upstream marker skips are not compatibility
  passes; the 22 CDP-session markers above also remain `not-run` because their
  session browser setup precedes the function-level skip.
- [x] Freeze a four-case synthetic remote-kill Python RED predecessor:
  [oracle 4/4, ABI125 candidate 2/4](../.work/official-conformance/results/python-connect-remote-kill-synthetic-v1-oct20-final/attestation.json).
  Candidate misses `disconnected` and ordered Page→Context→Browser close
  events after abrupt remote termination. The unchanged upstream assertions
  and 23 mutation controls are verified, but `launch_server` is synthetic,
  so no official ledger credit is earned.
- [x] Verify the source-level remote terminal-event correction against a
  rebuilt versioned wheel and the same four-case
  [fixture-adapted diagnostic](../.work/official-conformance/results/python-connect-remote-kill-synthetic-v2-terminal-fix-oct20/attestation.json):
  oracle 4/4 and candidate 4/4 in each of five independent runs, with 19 v2
  verifier/mutation tests. This is not an unadapted official pass and promotes
  no ledger row.
- [x] Reorder Rust session terminal publication: finish Page notices, publish
  Closed, then release pending/current command terminal errors. Three focused
  regression tests cover pending, send-failure, and ordered child-notice
  paths; selected page-events release tests pass 53/53 with strict Clippy.
  These external contract tests need not deterministically fail under the
  old single-thread scheduling, so they do not prove the former race rate.
- [x] Build an ABI126 selected release wheel with the Rust terminal-order
  source and default-off `frame-title` composition. Its typed `Page.title()`
  verifies the main Frame GUID/generation and adopted Page ancestry. A fresh
  installed-wheel, Tailscale-bound synthetic WebSocket test verifies the
  real `Frame.title` command after Frame→Page `__adopt__`; 70/70 focused
  Python tests, 37/37 browser-ops and 48/48 session release feature tests,
  and strict addon Clippy pass. The supervisor actor now fences the full
  lineage and adoption epoch before send and at the ordered reply; Page
  `close` tombstones the GUID without preempting later `__dispose__`.
  The wheel SHA-256 is
  `ac2f027be220ce59160e6df716c38bb6bd74f48fcfa17f3fed8af22dbfa77f05`.
  The earlier pre-fence ABI126 wheel remains historical evidence.
- [x] Seal the historical fixture-adapted Python remote-kill RED/GREEN
  sources, harness, ledgers, wheel and results in an immutable `.work`
  snapshot. Sealed replay and 10/10 mutation controls pass after live
  source drift; the snapshot still needs a portable, clean-CI input path.
- [x] Re-run a typed remote Page command during abrupt termination through
  the ABI126 installed wheel. A synthetic Tailscale-bound WebSocket server
  aborts after receiving `Frame.title`, before replying; the command
  settles with `TargetClosedError` and exact Page→Context→Browser events.
  Its focused test passes 10/10 independent reruns.
- [ ] Establish live Playwright Server/browser proof of typed remote Page
  operations and termination. The synthetic fixtures and
  frozen remote-kill diagnostics promote no official Python ledger row.
- [x] Implement a separate, default-off ABI127 close-notice source path for
  Python remote sessions. The additional Rust `ProjectCloseNotices` projection
  retains only Browser/Context/Page closure topology and generation identities,
  not Request/Response notices; the base supervisor still retains validated
  protocol objects under its own 16,384-GUID/4 MiB bounds. It emits ordered Page→Context→Browser
  and terminal notices. The native reader and one Python watcher dispatch
  server-origin Page/Context callbacks against generation-keyed wrappers,
  including notices that race wrapper publication. The ABI126 profile remains
  unchanged without the new feature.
- [x] Build a fresh opt-in ABI127 release wheel (SHA-256
  `8b524b5b424ff2b5dd3a8f871af3a6dbae961fddfdaf895b6000ecc5ff919cb9`)
  and verify server-originated Page close while Context/Browser stay connected,
  Context close, and abrupt disconnection through a Tailscale-bound synthetic
  WebSocket fixture. It pauses between `close` and `__dispose__` to check that
  native live inventories filter closed objects. The wheel passes 84 focused
  Python tests and ten additional independent fixture reruns; Rust all-feature
  release tests pass 41/41 browser-ops, 22/22 page-events, 65/65 session;
  selected addon strict release Clippy passes. Fake-native facade tests cover
  `Browser.close()` notice draining, ordering, GUID reuse, and watcher failure.
  A separate close-notices-only ABI127 wheel omits `frame-title` and its
  native Page.title method, while 51 close/ABI/fake-connect tests pass. A
  default-feature ABI127 wheel passes 661 browserless units with 13 explicit
  opt-in-profile skips and `uv pip check`. Targeted Ruff/Pyright pass with
  zero errors. This synthetic work promotes no official ledger row.
- [ ] Validate ABI127 against a real Playwright Server/browser and unchanged
  official Python tests; resolve the pre-existing whole-project strict
  Pyright gate and verify clean-CI dependency boundaries. Promote ledger
  rows only from independently verified official receipts.

Current remote Playwright Server Page-event checkpoint (2026-10-20):

- [x] Pin three unchanged Core `run-server` waiter assertions and record a
  fixture-adapted RED: oracle 3/3, candidate 0/3 after Context/Page creation.
  Candidate failure is at its first missing waiter call, before the assertions;
  no event-delivery or terminal-order parity is implied.
- [x] Add a standalone default-off Rust Page-event projection leaf for
  validated, wire-ordered Request/Response/Download and terminal notices.
  Sixteen release tests and strict Clippy pass, with no Node/CDP/WS dependency.
- [x] Wire an opt-in, eagerly drained, single ordered Rust event consumer to
  typed Node notices. Preserve GUID generations, Page attribution and
  Page→Context→Browser terminal order; bound conversion and backpressure.
  The selected session suite passes 50/50 release tests; the separate Node
  addon builds and passes a native-load check. Runtime notice delivery is
  separately evidenced by the paired diagnostic below.
- [x] Acknowledge Page request/response subscriptions before publishing the
  selected Page in Rust browser-ops (22/22 selected release tests).
- [x] Verify selected JS waiter APIs (25/25 focused tests, including early
  terminal-publication and terminal-predicate regressions), positive
  Request→Response→Download waiter/Page-listener delivery, and terminal
  teardown behavior through the rebuilt addon. The independently verified
  [v5 GREEN diagnostic](../.work/official-conformance/results/core-connect-page-events-synthetic-v2-green-20261020-v5/attestation.json)
  reruns the three unchanged pinned Core assertions 3/3 in isolated oracle
  and real-addon candidate lanes, with zero skips and five verifier mutation
  controls passing. The server launcher is a browser-free synthetic fixture;
  this internal diagnostic is not an unadapted official pass, real Playwright
  Server/browser or full public-API parity proof, and promotes no ledger row.
- [x] Admit generation-bearing remote Browser/Context/Page commands in the
  supervisor actor before wire send. A queued-recycle regression proves an
  actor-observed successor GUID cannot receive the stale Context close, and
  the rejected command consumes no wire ID. This is not an atomic remote
  compare-and-send and cannot detect disposal not yet received on the wire.

Next browser-free official public-API slice:

- [x] Re-run pinned Core Chromium Library
  `tests/library/browsertype-launch.spec.ts:64` (spec ID
  `d1a604bdf6287c6776f3-522221011eea38945220`) through isolated
  oracle/candidate lanes using its exit-1 dummy Node executable. The fresh
  [strict pair](../.work/official-conformance/outcomes/core-dummy-node-launch-strict-paired-v1/promotion-receipt.json)
  passes 1/1 per lane with zero skips and verifies public launch, live
  process/profile ownership, child reap, and profile removal. The independent
  verifier and fail-closed promoter change exactly one `not-run` row to
  `expected` per 48,477-row ledger; all other row bytes are unchanged. The
  historical receipts remain diagnostic. The strict pair is fixture-adapted,
  not an unadapted full Playwright run or Chrome/Context-tracing proof.
- [ ] Clear the ten targeted Ruff findings in the strict Core harness/test
  files (shebang permissions and existing exception/import/context-manager
  style), then recapture a new immutable evidence pair before relying on any
  changed harness bytes. Focused behavioral tests pass, but this targeted
  style gate is not green yet.
- [x] Pin the current ABI125 closed-target wheel and rerun the six unchanged
  async/sync Python `test_browser_type_connect_set_browser_connected_state`
  bodies through isolated browser-free synthetic-server oracle/candidate
  lanes. The independently verified [attestation](../.work/official-conformance/results/python-connect-lifecycle-synthetic-v2-closed-target-oct20/attestation.json)
  records 6/6 in each lane and 13 verifier/mutation tests. The server-launch
  fixture is adapted; this earns no Python ledger credit or engine proof.
- [ ] Design a separate strict real-server or scoped fixture-eligible proof
  before considering those six Python rows for official promotion. Keep the
  existing Python ledgers unchanged until that evidence exists.

Current typed selector-capture checkpoint (2026-10-12):

- [x] Source-implement one CSS/text `*` capture point for Page `$eval` and
  `$$eval`, retaining prefix and suffix across later filter/Locator
  composition before selecting deduplicated captured ancestors. An audit
  corrected an initial eager-rewrite error. Release host source and test
  compilation with handles and selector registration, formatting, and diff
  checks pass. The selector module passes 89/89 release tests.
- [ ] Rebuild and native-load the candidate profile, then rerun the four pinned official
  Core cases in paired candidate/oracle lanes before changing ledger rows.

Current official CDP conformance checkpoint (2026-10-12):

- [x] Collect all 6,526 pinned official Python cases in fresh ABI119
  candidate and official-oracle environments; collect the same 21 selected
  CDP/tap cases in four paired manifests. These are collection receipts only.
- [x] Source-implement independent opt-in Node Browser-root CDP sessions and
  a bounded `Target.targetCreated` return fence for both Page-creation APIs,
  including Page-selecting discovery filters and cleanup on post-create flush
  failure. Node browserless tests pass 773/773; selected release host source
  and test compilation pass. Both CDP leaf release addon checks and the
  zero-added-package dependency script pass on the current source. Focused CDP
  host release tests pass 32/32.
- [ ] Complete the topology-driver release tests (its previous compilation
  exceeded safe memory headroom), then rebuild and native-load the combined
  Node profile containing this new feature.
- [ ] Run the unchanged 12-case official Core Chromium CDP file and 21 selected
  Python CDP/tap cases in paired browser-backed lanes. Keep the ledgers
  unchanged until exact source-bound outcomes exist; `Page.coverage` is a
  separate remaining Core surface gap.
- [ ] Rebuild the ABI119 Python wheel with current README metadata, rerun its
  installed-wheel ABI checks, and verify payload reproducibility.

Python/Core full-official-corpus checkpoint (2026-10-11):

- [x] Source-wire opt-in async/sync Python
  `ElementHandle.scroll_into_view_if_needed()` through the exact retained-node
  Rust host action. ABI113 forwards Page/Context timeout, treats explicit zero
  as no host deadline, and keeps the addon dependency closure equal to
  `handles`; focused fake-session facade tests are source-only.
- [ ] Build/install ABI113, run focused package tests and the selected official
  Chromium scroll cases in independent candidate/oracle lanes, then audit
  exact source-bound receipts before claiming browser parity or ledger credit.
- [x] Source-wire async/sync Python `ElementHandle.owner_frame()` through the
  existing opt-in `element-handle-dom` and Rust retained-Handle owner query.
  ABI112 uses current BrowserContext Page/Frame reconciliation, including
  cross-Page adoption and canonical sync wrappers, without a new Cargo
  feature or package dependency. Focused facade tests are source-only.
- [ ] Build/install ABI112, run focused package tests and the pinned seven
  async/six sync official Chromium owner-Frame cases in candidate and oracle
  lanes, then audit exact source-bound receipts before any strict-ledger
  promotion.
- [x] Source-wire opt-in Python `ElementHandle.get_attribute`, `inner_html`,
  `input_value`, and `is_checked` through one exact retained-Handle Rust DOM
  read port. ABI111 prevents mixing with an older wheel; the additive feature
  depends only on `element-handle-dom` and has a package-closure proof.
- [ ] Build and install an ABI111 full-profile wheel, run focused facade tests
  and complete pinned async/sync official ElementHandle files in both lanes,
  then promote only source-bound audited rows. Do not credit `input_value`
  while its official case still stops at missing `ElementHandle.fill`; assess
  explicit `input_value(timeout=...)` deadline semantics separately.
- [x] Wire opt-in Python `ElementHandle.bounding_box()` through the existing
  retained-Handle Rust host geometry operation. The Python source ABI is 110;
  release addon check, formatting and Ruff pass. This is source-only, with no
  official candidate pass credit.
- [ ] Build/install ABI110 with the intended complete feature profile, run
  focused facade units and the complete pinned async/sync ElementHandle files,
  verify source-bound provenance, and update strict ledgers only for exact
  independently evidenced rows.
- [x] Wire the existing Rust-owned `element_handle_preview` into the opt-in
  Python `Locator.element_handle` and `JSHandle.evaluate_handle` return paths;
  the base `handles` profile remains unchanged. Eight focused facade tests
  pass, including the four exact official preview strings. This is source-only
  until a new wheel and the pinned two-case browser rerun pass; review
  fail-closed preview errors versus upstream asynchronous refresh separately.
- [x] Inventory and execute the complete pinned Chromium Python async/sync
  `test_tap.py` (7+3) and `test_cdp_session.py` (6+5) files in independent
  source-bound candidate/oracle lanes. Candidate fails all 21 on the frozen
  100-capability wheel; oracle passes all 21. Strict successors change exactly
  those 21 rows, with zero exclusions or pass credit for candidate failures.
- [ ] Design and implement opt-in `cdp-session` across the topology driver,
  bounded host session ownership/event queue, Python native bridge, and
  async/sync public API. Use an auxiliary browser-root CDP session with an
  exact-parent topology discriminator; do not reuse the Page's primary
  session, which would violate detach semantics. Validate Page/Frame ownership,
  close and detach races, event-publication ordering, and both complete
  official test files before promotion.
- [ ] Design opt-in touch/tap capability and
  `ElementHandle.evaluate_handle` without implying touch support from the
  existing mouse feature. The ten complete official tap cases remain
  candidate failures.
- [ ] Generalize deferred Python route-handler exceptions from the current
  one-shot `Page.goto` preflight to the upstream next-command boundary.
  The narrow source fix passes both previously failing request-intercept
  cases on the new 100-capability wheel; the complete async/sync files now
  pass 14/14 candidate and oracle with source-bound receipts.
- [x] Pin and execute the complete Core public Chromium
  `elementhandle-misc.spec.ts` 10-case file against the frozen candidate and
  official oracle. Source-bound strict successors change exactly ten rows:
  candidate one expected / nine unexpected, oracle ten expected. Seven
  candidate failures are absent handle methods; two are evaluation-world
  `Node` lookup errors.
- [x] Remove the selector runtime's dependency on the mutable page-global
  `Node` constructor by using local standard `nodeType` constants. The runtime
  version is 13; its ten release unit tests and strict release Clippy pass.
  This is source-only hardening, not official browser pass credit.
- [ ] Design a bounded opt-in utility-world selector/ElementHandle path with
  cross-world adoption into the canonical Frame world. Built-in `page.$`
  currently evaluates in the main world, whereas Playwright queries in the
  selector's world and adopts the result. The Node host currently selects no
  persistent ElementHandle adoption profile, so switching worlds requires an
  explicit resource budget and cancellation-safe ownership proof. The seven
  absent public handle methods remain a separate compatibility gap.

Python/Node full-official-corpus checkpoint (2026-10-10):

- [x] Build and install the frozen ABI108 reload/click v2 wheel (SHA-256
  `862c7074e3bbd6a9782b11af0e1ff66f9ffd84c13e0a40b1490b2858d48fa027`).
  Python 3.9 and 3.12 each pass 418 local units; installed native ABI is 108
  with 97 features. Exact unchanged official candidate and oracle runs now
  pass Context init-script 1/1 and the five previous click timeouts 5/5 in
  each lane, with all four schema-5 sidecars verified. New six-case-only
  6,526-row strict successors and invocation audits pass: candidate 1,319
  pass / five fail / 11 skip / 5,191 not run; oracle 1,097 / five / 11 / 5,413.
- [ ] Resolve the remaining full click-file discrepancy without waiving
  evidence: v2 candidate full70 is 60 pass / nine fail / one fail-closed
  malformed isolated report; v2 oracle is 69 pass / one shared 404-console
  failure. Eight candidate failures lack `ElementHandle.click`; the ninth
  handle case produced two pytest testcase records and remains an error.
  Both complete reports verify source-bound but are diagnostic only, not
  strict-ledger promotion. A separate non-official observer confirmed the
  shared request is `/favicon.ico` returning 404 on `/offscreenbuttons.html`;
  it must never count as official pass evidence.
- [ ] Build a versioned, source-bound fixture variant that serves a real icon
  only for the exact offscreen-button case. Do not mutate the existing runtime
  tree in place: historical schema-5 sidecars pin its hash. Rerun the
  unchanged official candidate and oracle case without the observer before
  changing either ledger.
- [ ] Wire the new opt-in `retained-element-click` backend leaf through host,
  addon and Python while preserving exact handle identity. Its top-level-only
  source passes 130/130 feature-on and 123/123 default release tests, but it
  rejects child/OOPIF Frames and has no browser pass credit. Implement safe
  frame-to-top-level CSS coordinates before claiming the iframe case.
- [x] Build/install the complete ABI108 Python wheel (SHA-256
  `bcfa4da75e539534701d41c5a9bcca4347e993bf98aab211cd8ad9c32f6d796e`),
  confirm the installed ABI/features, and execute exact paired official
  click-eight and Context init-script selections with four verified
  source-bound reports. Candidate click improves 4/8 to 7/8; oracle remains
  8/8. The Context callback now reaches `context` but still misses `page`
  after reload; candidate 0/1, oracle 1/1. Immutable strict successors and
  full audits record candidate 1,313 pass / six fail / 11 skip / 5,196 not run
  and oracle 1,097 pass / five fail / 11 skip / 5,413 not run.
- [x] Execute all 70 Chromium click cases diagnostically on the installed
  ABI108 candidate: 53 pass / 14 fail / three fail-closed report errors,
  source-bound. Eight failures need `ElementHandle.click`, one is the shared
  404 console assertion, and five cases time out (two previously unsupported
  offsets, one unsupported force, and two prior report errors). Other-task
  Chrome pages appeared by the end, so this has no strict-ledger promotion.
- [x] Investigate and rerun the five v1 timeout cases in uncontested Chrome:
  v2 candidate and independent oracle each pass 5/5. The complete v2
  click-file candidate also passes those five, but its separate malformed
  handle-case record still bars whole-file strict promotion.
- [x] Add scoped source-only actionability repairs for the five ABI108 click
  timeout cases: requested-point scrolling through nested overflow containers,
  clickable-ancestor hit retargeting, and terminal forced-hidden errors. Rust
  formatting/diff checks, a synthetic DOM exercise, release `cargo check`,
  strict release Clippy, and the backend's 123/123 release library tests pass;
  the frozen v2 wheel and exact paired official 5/5 runs now prove this bounded
  browser behavior. The source-only wording describes the earlier checkpoint.
- [x] Design a retained-handle click backend primitive before exposing Python
  `ElementHandle.click`. Selector reacquisition would violate handle identity,
  while the existing host tap sends iframe-local coordinates to Page input;
  exact frame-to-top-level mapping and shared pointer cleanup are required.
  The proposal is in `docs/retained-handle-click-design.md`; implementation
  and official pass credit remain open.
- [x] Diagnose the post-reload Page init-script callback in an uncontested CDP
  window: both script markers are true, but `reload()` returns with only
  `['context']`; one subsequent `evaluate()` observes `['context', 'page']`.
  This is callback-publication lag, not missing Page-script installation.
- [x] Add a same-main-frame binding-publication fence to successful Page
  reload, bounded by the original reload timeout and shutdown cancellation.
  It does not reuse the existing global publication watermark. The rebuilt
  ABI108 v2 wheel passes the unchanged official candidate and oracle case
  1/1 each with source-bound provenance; focused release tests pass 4/4.
- [x] Promote the exact frozen-v6 `BrowserContext.expose_binding` official
  Chromium row after candidate and oracle each pass 1/1 with source-bound
  reports; the prior candidate `AttributeError` belonged to an older wheel.
  The separately source-bound complete 70-case click oracle is 69 pass / one
  404-console failure. Preserve its failure and promote only its lane; the
  candidate 70-case diagnostic remains unpromoted without an uncontested
  repeat. New immutable 6,526-row successors are candidate 1,310 pass /
  nine fail / 11 skip / 5,196 not run and oracle 1,097 pass / five fail /
  11 skip / 5,413 not run, with full invocation audits passing.
- [x] Stage bounded early binding events until an exact main-world route is
  available and clean up the exact initialization record if its future is
  cancelled. Focused Rust release tests pass 34/34, but no rebuilt wheel or
  official init-script callback rerun has proved the browser behavior.
- [ ] Close the intentional dialog-interruption gap in early binding staging.
  If an init script calls a binding and then opens a dialog before Page
  discovery finishes, the dialog cancels initialization and the generic
  cancellation guard removes the staging record; queued binding events can
  then arrive without a record and be discarded. Preserve only the exact
  retryable session generation across that interruption, bound its queued
  calls, retire it on true session close, and prove the event ordering with
  focused tests and a fresh official browser run. This is a source-review
  risk, not a browser-confirmed diagnosis of the existing callback failure.
- [x] Implement source-only Python async/sync `Page.dblclick`,
  `Locator.click(steps)`, and `Page.click` position/button/click-count/force
  forwarding through additive PyO3 options and existing Rust `ClickOptionsV2`.
  The exact native ABI handshake is now 108. Minimal and selected-feature
  release checks pass; Python native click-option release tests pass 14/14
  with the installed Python 3.12 linker. The subsequently built ABI108 wheel
  and exact paired official runs above provide bounded browser credit.
- [ ] Complete the pinned Playwright Python Page action signatures. Current
  `Page.click` still lacks `delay`, `no_wait_after`, and `trial`;
  `Page.dblclick` lacks `button`, `delay`, `force`, `modifiers`, `position`,
  `no_wait_after`, and `trial` in both async and sync facades. Keep unsupported
  options explicit until the native semantics and official tests prove them.
- [ ] Complete the ABI108 installed package gates: the exact full-feature
  wheel imports on Python 3.9 and 3.12, and each interpreter passes 418 unit
  cases with one default-profile-only ABI/profile assertion deselected. The
  default-profile build, release dependency checks, and wheel reproducibility
  still need verification.
  The first full-feature build attempt was stopped before output under memory
  pressure; the successful second build above uses the intended feature set.
- [x] Build and install the ABI107 Python Context-binding/media v4 wheel;
  verify all 379 local package units on both Python 3.9 and 3.12, strict
  Python gates, normalized release-payload reproducibility, and exact
  opt-in dependency closures. The installed wheel corrects a Python 3.9
  prior-loop-close `asyncio.Lock` failure found in its first diagnostic build.
- [x] Repair the v3 full-feature browser regression where exact same-session
  `Target.attachedToTarget` replay initialized Page bindings twice and broke
  ordinary `new_page()`. The Rust topology guard preserves distinct-session
  detach, passes both focused release regressions and strict Clippy; the v4
  installed wheel passes the unchanged official Chromium navigation smoke
  1/1 with verified source-bound provenance. No full-corpus claim follows.
- [x] Rerun the full release `pwright-composition-chromium` library suite
  after fixing three stale test-only fixtures/assertions: 411 executed pass,
  zero fail, 16 explicitly ignored. Strict release Clippy, workspace format,
  and diff-whitespace gates pass; this does not promote official-test rows.
- [x] Run the pinned official Python Chromium Context forced-colors case
  through the installed v4 candidate and independent Playwright oracle: both
  pass 1/1 with source-bound provenance. Promote exactly the applicable
  candidate `failed` row to `passed`; latest candidate ledger is 1,295 pass /
  nine fail / 11 skip / 5,211 not run, with all other rows unchanged.
- [x] Pin the next Core JS/TS full-file Page set-content shard: 17 Chromium
  cases collect exactly, all still `not-run`; two use upstream-private `toImpl`.
- [x] Rerun the pinned seven-case Chromium `BrowserContext.clear_cookies`
  selection against installed v4 candidate and independent oracle: both
  pass 7/7 with source-bound provenance. Promote only seven previously unrun
  oracle rows; refresh evidence for seven already-passing candidate rows.
  Strict ledgers are candidate 1,295 / nine / 11 / 5,211 and oracle
  1,021 / four / 11 / 5,490; candidate-pass/oracle-not-run is 280.
- [x] Build the opt-in Node Page official-union-plus-burst-20 addon and prove
  browserless native loading while preserving default/prior profiles. The
  addon SHA-256 is
  `3dfb2ac8698de70f5b2416688b2d15de879251d9d7bf5c5549474ecb9fbd5c83`;
  package tests pass 565/565 and focused contracts 122/122. This is not an
  official browser run or ledger promotion.
- [x] Pin a separate exact one-row Core Page.goto burst-20 manifest and
  fail-closed runner/provenance checks for its source, project, file, ID and
  manifest-owned title filter. Runner/provenance tests pass 50 and official
  browserless collection finds exactly one row. The paired browser attempt
  refused two unrelated XLQY pages before execution; no Core credit was
  recorded and no other-task page was closed.
- [ ] Execute the pinned burst-20 row in candidate and independent oracle
  after the shared dedicated Chrome has an uncontested safe window, then
  source-bind reports and assess the exact strict-ledger row.
- [x] Pin the Python Chromium `emulation_focus` full-file shard: seven
  public-API rows were `not-run` in both pre-v5 strict ledgers. The manifest
  fixes upstream test/fixture hashes and both collections verify 7/7.
- [x] Execute all seven focus cases in both lanes in an uncontested CDP window:
  candidate 6/7, oracle 7/7, all source-bound. The candidate failure is
  missing `Frame.focus`; promote six candidate and seven oracle passes while
  recording the candidate failure, with no hidden exclusions.
- [x] Add a browserless Python facade regression for the exact Context `add`
  → Page `mul` → Context `sub` awaited-binding sequence, with separately gated
  registration, dequeue, acknowledgment, delivery and bounded stage waits.
  The focused file passes 25/25; it does not reproduce the official timeout.
- [x] Correct the source-level new-Page init-script CDP parameter gap:
  inherited scripts now send `runImmediately: true`; dynamically registered
  scripts retain the old future-document behavior. Two fake-CDP payload tests
  pass, as do six focused release tests and strict Clippy under standalone
  `init-scripts` and combined `context-bindings,init-scripts` profiles. The
  v4 wheel predates this change. A later v5 official rerun still fails the
  unchanged callback case, so this CDP parameter repair alone is insufficient.
- [x] Build the isolated ABI107 v5 wheel with the exact v4 96-feature profile
  and the init-script repair, then independently rebuild it: normalized
  payloads and native hash match. Python 3.9/3.12 installed package suites
  pass 380/380 each; all 81 release dependency checks, addon strict Clippy,
  Ruff, runtime type audit and expanded CI Pyright pass. Bounded v5 browser
  evidence and strict-ledger changes are listed below; these browserless
  checks alone gave no credit.
- [x] Recollect all pinned Python v1.58.0 IDs from isolated v5 candidate and
  independent official oracle installs: each yields 6,501 core and 25 examples
  IDs, with all four schema-5 sidecars source-bound and both complete
  6,526-ID inventories byte-identical to the existing pinned inventory.
  Collection adds no official execution or strict-ledger pass credit.
- [x] Type-check the four previously omitted official Python tool test
  scripts under pinned Pyright 1.1.405, add them to CI's path list and keep
  their test semantics. Expanded Pyright has zero errors; focused pytest
  passes 101/101 and CI unittest discovery passes 208/208.
- [x] Rerun the unchanged source-pinned official init-script callback case
  against v5 and independent oracle: candidate still fails `[] != ['context']`,
  oracle passes. Keep the failed candidate row and investigate the remaining
  binding/init-script lifecycle discrepancy.
- [x] Run the independently pinned basic Context exposed-function case under
  v5: both candidate and oracle pass 1/1 with schema-5 provenance. Its earlier
  v4 90-second partial-run timeout remains unexplained, not a proven fix.
- [x] Create immutable v5 Python strict successors after the three paired
  selections and complete invocation audit: candidate 1,302 pass / nine fail /
  11 skip / 5,204 not run; oracle 1,028 / four / 11 / 5,483. Six new focus
  candidate passes and seven oracle passes are real browser credit, not
  collection credit; most of the corpus is still unexecuted.
- [x] Pin the full Python async `test_click.py` Chromium file in the corrected
  v2 manifest: 70 applicable public-API cases, with 48 Page.click, nine
  ElementHandle.click, nine check/uncheck, two mouse, one double-click, and one
  Locator.click primary actions. The original v1 manifest is retained for
  historical receipt verification but mislabels 13 primary actions. Separate
  installed-v5 candidate and independent-oracle browserless v2 collections
  each find 70 exact cases with verified schema-5 provenance; 69 candidate
  and all 70 oracle rows were not-run in the then-current strict ledgers. This
  preparation earns zero new execution credit.
- [ ] Execute those 70 click cases in isolated candidate/oracle browser runs
  when dedicated Chrome is uncontested; preserve the separately verified
  eight-case strict evidence, bound the three `timeout=0` cases with outer
  deadlines, and audit every resulting row.
- [x] Run the frozen v6 candidate over the complete 70-case Chromium click
  manifest as a diagnostic: 48 pass / 19 fail / three fail-closed malformed
  isolated-case report errors, all source-bound. Four unrelated XLQY Chrome
  pages appeared near the end and memory pressure rose. Keep this unpaired
  candidate report out of strict ledgers until an uncontested repeat and
  independent full-file oracle exist; close only the task-owned blank tab.
- [x] Add thin async/sync Python `Frame.focus` facades routing the target
  child frame into the already-owned Rust native focus capability. Five focused
  browserless tests, Ruff, and touched-file Pyright pass. The later installed
  v6 candidate/independent-oracle complete focus file passes 7/7 in both
  lanes, promoting the former candidate failure with source-bound evidence.
- [x] Build the isolated ABI107 v6 Python wheel with the previous 96 selected
  features plus only `context-strict-selectors` (97 actual selected), confirm
  packaged source equality, and pass 35/35 focused and 389/389 fresh installed
  full-profile units. The one frozen default-profile assertion passes 1/1 in
  the separate prior profile; its v6 deselection is explicit.
- [x] Run the exact strict-Context plus duplicate-registration pair, complete
  seven-case focus file, and init-script callback in both installed-v6
  candidate and independent official oracle lanes. Results are candidate
  2/2, 7/7, 0/1 versus oracle 2/2, 7/7, 1/1, with six source-bound execution
  sidecars and no altered assertions. The one failure remains real.
- [x] Recollect the complete pinned Python v1.58.0 corpus with fresh v6
  candidate and independent-oracle installs: 6,501 core IDs under Python
  3.12 plus 25 TodoMVC example IDs under the separate pinned Python 3.10 /
  pytest-playwright profile. All four sidecars verify source-bound; no
  browser execution or pass credit comes from collection.
- [x] Promote only those ten selected v6 evidence rows in immutable 6,526-row
  successors and complete invocation audits: candidate 1,305 pass / six fail
  / 11 skip / 5,204 not run (SHA-256
  `fcd87c82d42af74b3749a41dd2d7b3d060fc51af8e36a34daf677d46635cce7d`),
  oracle 1,028 / four / 11 / 5,483 (SHA-256
  `8d11467c197f0f224b64cf20ba7cef7baf88e2d9150402248b3e3183258493c4`).
  The candidate transitions exactly three `failed` rows to `passed`; 5,197
  applicable candidate rows remain not-run.
- [x] Pin a separate exact eight-case subset of the corrected official Python
  click file, preserving clean v1.58.0 test and fixture hashes. Its source
  checker passes; paired isolated Chromium runs are candidate 4/8 and
  independent oracle 8/8 with verified schema-5 sidecars. The four candidate
  failures are absent Python `ElementHandle.click`, `Page.click(position)`,
  `Locator.click(steps)` and `Page.dblclick` surfaces in frozen v6; no assertion
  or test was removed.
- [x] Promote exactly those eight new rows per lane in immutable 6,526-row
  successors and rerun complete invocation audits. Candidate is 1,309 pass /
  ten fail / 11 skip / 5,196 not run (SHA-256
  `5c0bdcb6538b5fa8ac946df6762b5b8996b1a7141bb7b7befd4ca6f81cf8b514`);
  oracle is 1,036 / four / 11 / 5,475 (SHA-256
  `5ad9aef3152b7f8c4a73ee7a40744952bbe04010197ae8af81e5a5581b0da310`).
  Full click-file rows remain candidate five pass / four fail / 61 not run,
  oracle eight pass / 62 not run.
- [x] Narrow the Context init-script callback failure with an independent
  CDP diagnostic: the script runs while `woof` is a function, but its early
  callback is lost; a later call on the ready Page is delivered. The Rust
  observer's early no-route return is a strong hypothesis, not yet a proven
  causal trace or repaired behavior. No official credit follows.
- [x] Prepare the exact one-case Chromium Context init-script callback
  manifest, preserving the upstream source/fixture hashes from the pinned
  39-case selection. Old-v4 candidate and independent oracle collect 1/1
  source-bound, but collect-only JUnit records zero executed cases.
- [x] Pin the public-only Core Chromium Page set-content selection: 15 exact
  spec IDs, with the two `toImpl` timeout cases explicitly inventoried outside
  this bounded run. The manifest-owned invert filter and exact ID checks pass
  browserless collect 15/15 and runner/provenance tests 55 plus 18 subtests.
  No browser result or strict-ledger credit follows from collection.
- [x] Pin the next Core Chromium Page-click public-only selection from the
  clean v1.58.0 checkout: 81 exact `chromium-page` public cases, with six
  upstream-private hook cases explicitly inventoried outside the bounded run.
  A fixed runner/provenance validator enforces source, IDs and grep-invert;
  61 focused tests pass and upstream browserless listing finds 81. No Core
  candidate collection or browser/ledger credit follows.
- [x] Partition those 81 public Core click cases by primary API: 60
  Page.click, two Frame.click, six ElementHandle.click, two Page.dblclick,
  11 Locator.click. Implement thin Node Page/Frame selector click
  default-first/explicit-strict semantics and Page/Frame dblclick through
  Rust-backed Locator actions. Full Node package suite passes 570/570 and
  TypeScript check passes. No official browser result follows from this
  browserless facade work.
- [x] Implement opt-in Node `Locator.click({steps})` forwarding into bounded
  existing Rust click input (integers 1–10,000). Runnable release Rust tests
  pass 2/2, full Node package tests 574/574, and TypeScript checks pass; this
  does not change the frozen Python wheel or establish official browser parity.
- [ ] Implement missing Node `ElementHandle.click` with retained-handle identity,
  actionability and correct frame-to-Page coordinates; a source audit found
  that directly re-querying via Locator.click would violate handle identity.
  Rebuild the current-source native addon before paired Core Page-click runs.
- [ ] Execute the 81 public Page-click cases in both official lanes only after
  a current-source Node addon rebuild, candidate runtime preflight, and an
  uncontested dedicated Chrome window; keep six private-hook rows accounted
  for separately.
- [x] Pin all 55 public `chromium-page` Page-route cases from the clean Core
  v1.58.0 file. The exact source/ID manifest and two independent browserless
  listings agree on 55 cases; both strict Core ledgers still say not-run, with
  no private-hook exclusions or saved result sidecar.
- [ ] Before paired Page-route execution, provide a verified trusted HTTPS
  fixture for its service-worker activation case or record the HTTP-origin
  failure explicitly. Run all 55 in both lanes only in an uncontested Chrome
  window and account for every selected row.
- [ ] Execute the 15 public Page set-content cases in both official lanes
  when the shared CDP profile has no other-task nonblank pages; keep the two
  private `toImpl` rows explicit in the full corpus inventory.
- [ ] Diagnose and repair the official Python Context binding lifecycle:
  the interrupted v4 39-case shard produced an init-script callback failure,
  a 90-second basic exposed-function timeout, then unreachable shared Chrome
  CDP. Preserve partial reports without promotion. The dedicated Chrome later
  recovered externally; v5 basic binding passes but v5 and v6 init-script
  callback still fail. Trace native publication/settlement before claiming a fix for
  the still-unlocated intermittent timeout or deterministic callback failure.
- [x] For the retained duplicate-registration failure, use a fresh installed
  wheel to confirm `BrowserContext.expose_function` and `context-bindings` are
  present, then rerun the unchanged official case in an exact two-case pair.
  Installed v6 candidate and independent oracle both pass; the older wheel's
  immediate `AttributeError` is historical.
- [ ] Discriminate the init-script callback failure with a bounded event-stage
  probe: script execution/`typeof`/caught error, binding event seen, frame
  route found, dispatch admitted, and Python callback observed before the
  page-evaluation fence. Upstream waits for Page initialization before binding
  routing; current Rust may drop an early event, while publication timing is
  a separate possible cause. Repair only the demonstrated branch.
- [x] Implement the first opt-in Context `strict_selectors` policy slice in
  Rust/FFI and thin Python facade: a bounded per-Context default, explicit
  `strict` override, ordinary non-strict behavior, and Page/selected Frame
  selector paths. Release Rust policy tests pass 4/4; focused Python tests
  19/19, Ruff, rustfmt and Pyright pass. The default/v5 wheel does not include
  this feature; the v6 wheel and official paired rerun above promote only its
  exact Context test, not general strict-selector parity.
- [ ] Finish auditing and implementing the remaining Page/Frame selector
  entry points for the same strictness rule, then run adjacent official
  multi-match cases. Several implemented methods still ignore Context
  strictness and many upstream selector APIs are absent altogether.
- [x] Recollect the pinned Playwright Python v1.58.0 corpus in current v4
  candidate and independent oracle environments: 6,501 core + 25 examples IDs,
  with source-bound provenance. This is collection only and adds no pass
  records to either 6,526-row strict ledger.
- [x] Add two release Rust regressions that reproduce shared FFI Command
  count/byte starvation before binding acknowledgement or delivery enters
  the host future. These are admission tests, not liveness proofs.
- [ ] Execute the frozen wheel's full applicable Python corpus and the full
  official Core JS/TS corpus against independent oracle/candidate lanes;
  preserve exact source, browser isolation, failures, skips, and non-applicable
  classifications. The dedicated Chrome CDP became unreachable during the
  interrupted 39-case Context run and later recovered externally; the v4
  Python browser selections are not whole-corpus evidence.
- [ ] Resolve Context binding current-document partial-install rollback,
  dropped native registration Future and ambiguous CDP reply, and shared
  FFI/native-scheduler settlement resource cycles. Prove atomic same-name
  retry, exact cleanup, and acknowledgement/result/error progress under
  adversarial saturation before claiming complete compatibility.

Official cross-language conformance catch-up (2026-10-10):

- [x] Run the existing exact-source `BrowserContext.request` Chromium
  manifest against the independent official Python oracle: 51/51 async and
  47/47 sync pass, with complete schema-5 source-bound JUnit/provenance. Only
  those 98 previously `not-run` oracle keys change; the strict oracle ledger
  now has 934 passes and the candidate ledger remains at 1,279 passes.
- [x] Rerun the 51 async cases against the current isolated candidate wheel:
  51/51 pass with source-bound provenance. Existing candidate pass records get
  updated evidence, not 51 new outcome passes. A first attempt correctly
  refused other tasks' nonblank CDP pages before creating a report; they were
  not closed by this task.
- [x] Rerun the remaining 47 sync cases against the same current wheel in an
  empty-CDP window: 47/47 pass with source-bound provenance. Replace only
  their existing pass evidence references in the strict 6,526-row candidate
  successor; do not count them as new passes.
- [x] Pin an outcome-independent manifest for all 39 oracle-`not-run`
  Chromium rows of `tests/async/test_browsercontext.py`, including all 25
  candidate-`not-run` rows. Source verification and oracle collection pass
  39/39; collection grants no execution credit.
- [x] Run the same 39 BrowserContext IDs in independent oracle and current-wheel
  candidate lanes with task-only CDP inventory at run boundaries. The corrected
  oracle and explicit `chrome` channel produce 35 pass / four popup failures;
  candidate produces 25 pass / 14 failures. Both complete schema-5 sidecars
  verify: 22 both pass, 13 oracle-pass/candidate-fail, one both fail and three
  oracle-fail/candidate-pass. The earlier oracle 29/10 run had other-task CDP
  interference and stays diagnostic. The four persistent popup failures also
  remain diagnostic pending fixture investigation. The later 35-case verified
  subset, not either complete report, advances only the oracle-passing rows.
- [x] Correct the oracle CDP Browser adapter's public error-name prefix for
  async and sync while keeping fixture-owned Contexts hidden; 83 browserless
  harness checks pass. This changes runtime provenance for future reports,
  not the archived 39-case diagnostic or strict ledger.
- [x] Rerun all 39 exact BrowserContext oracle IDs with corrected adapter and
  recorded `chrome` channel: 35 pass / four popup failures, source-bound
  schema-5 verification passes, and CDP inventory is task-only at start/end.
  Keep failures explicit; paired candidate report and audit are complete. The
  four popup cases stay `not-run` in strict ledgers after the later 35-case
  source-bound selection.
- [x] Derive the exact 35 oracle-passing BrowserContext cases from the paired
  39-case schema-5 reports with a fixed-base source-bound receipt. The
  derivation tool passes 17/17 hermetic tests. Strict 6,526-row successors
  verify all selected rows and leave every other row untouched: oracle 969
  passed / four failed / ten skipped / 5,543 not run; candidate 1,287 passed /
  17 failed / 11 skipped / 5,211 not run. Thirteen candidate discrepancies
  are explicitly failed; the four popup cases remain `not-run` in both lanes.
- [ ] Officially verify the locally implemented, separately selectable
  Context `forced_colors` policy against the pinned upstream cases, including
  current/future Page and OOPIF initialization and Page-level overrides; the
  strict ledger still reflects the older wheel. Implement the separate
  `strict_selectors` Context policy: an explicit per-call choice wins,
  otherwise use the Context policy. Existing default `Page.text_content` and
  `query_selector` behavior needs tri-state, Context-isolation and lifecycle
  regression coverage. Accepting a keyword alone is not sufficient.
- [ ] Finish and officially verify the opt-in Rust-owned Context binding
  scope for the four `BrowserContext.expose_binding/expose_function` cases.
  Local Rust/Python Context and Page registration facades now exist with
  Browser-shared bounds, but the strict candidate ledger still reflects the
  older wheel. Current-document partial-install rollback, dropped native
  registration Future, ambiguous CDP reply and settlement liveness remain
  open; preserve the upstream duplicate-registration and lifecycle semantics
  when completing them. See the current checkpoint above.
- [x] Match the two pinned null-viewport conflict errors in Python async and
  sync preflight before Rust context admission; five local unit cases pass.
  Mirror the conflicts in the Node context-option boundary (three local
  adapter checks and one entrypoint integration check pass).
  Valid `no_viewport=True` still requires a separate capability, and these
  local checks give no official-test pass credit.
- [x] Implement an opt-in Rust-owned runtime `BrowserContext.set_offline`
  toggle, including current/future CDP sessions and thin Node/Python bindings.
  Focused Rust, Node, Python checks pass 7/7, 21/21, 3/3; the two selected
  official Python cases remain `not-run` pending paired browser execution.
- [x] Add bounded dirty-state reconciliation to the offline owner: defer policy
  commit, compensate an ambiguous failed CDP command and earlier sessions,
  serialize Context policy removal, and rebroadcast even an unchanged request
  after failed rollback or caller cancellation. Focused release tests pass
  18/18, but cancellation can leave a temporary native split until a later
  call; these local tests give no official pass credit.
- [ ] Test actual close/toggle interleaving, Python cancellation and allowed
  service-worker fetch behavior in browser-backed candidate/oracle lanes.
  Page/OOPIF-only local checks do not prove service-worker parity or immediate
  atomicity under caller cancellation.
- [x] Pin the complete official Core `tests/page/page-goto.spec.ts` file for
  `chromium-page` and collect all 64 public-API rows under the exact one-file
  manifest. All 64 were initially unrun in both strict ledgers.
- [x] Guard that Core full-file run and provenance replay with the pinned
  source hash and all 64 exact IDs; reject test/grep overrides. Provide an
  opt-in, Tailscale-bound second origin for the upstream fixture and verify
  its teardown without a browser. Distinct origin is not process-isolation
  proof.
- [x] Execute all 64 pinned Core `page-goto` rows against the independent
  oracle: 63 expected passes, one explicit MagicDNS-fixture failure, no skips;
  verify the source-bound sidecar. Rebase exactly these 64 oracle rows into
  the immutable `core456` ledger, with no candidate credit. A candidate
  attempt refused other tasks' reopened nonblank CDP pages before test
  execution.
- [x] Execute those same 64 Core rows in both lanes without filtering TLS,
  redirect, cross-origin or fixture-sensitive outcomes. The source-bound
  candidate report records 45 expected / 19 unexpected; the oracle records
  63 expected / one unexpected. Only the protocol-less MagicDNS URL failure
  is shared, leaving 18 candidate-only unexpected IDs. Different origins on
  the same host do not prove process isolation. Both immutable 48,477-row
  strict ledger successors and invocation audits preserve all 48,413
  unrelated rows unchanged.
- [x] Rerun the unchanged complete 64-case Core navigation file with the
  cumulative release artifact and source-bound provenance. Candidate improves
  from 45 expected / 19 unexpected to 57 expected / seven unexpected; exactly
  12 previously unexpected cases become expected. The new 48,477-row strict
  successor and invocation audit pass; 48,413 unrelated rows are unchanged.
  The independent oracle remains 63 expected / one shared MagicDNS fixture
  failure. Keep this as a full-file result, not 12 isolated pass claims.
- [ ] Resolve the remaining six candidate-only Core navigation discrepancies:
  three overlapping-navigation/cancellation ordering cases (source lines
  471, 543, 740), one upstream matcher-private bridge case (`toHaveURL` at
  492), the 20 simultaneous waiters (629), and missing public
  `framedetached` delivery in the iframe attach/detach case (859). The three
  overlap cases need timestamped CDP/server/Promise traces before assigning a
  root cause; `framedetached` delivery alone may not finish the iframe case.
  Keep the shared oracle fixture failure visible until a
  semantically faithful Tailscale-compatible fix exists. The earlier 18
  candidate-only failures are a historical baseline; the local top-level CDP
  referer policy and `data:` request suppression now have full-file browser
  proof within the new 57/7 result.
- [x] Locally suppress `data:` public Request and companion network events
  in the CDP leaf with finite resource-budgeted correlation; release leaf
  checks pass 16/16 default and 24/24 with request-frame/request-timing,
  plus 12/12 model checks. The cumulative native artifact includes this leaf,
  and the later unchanged Core full-file result provides official browser
  coverage for its selected case;
  extreme late-tail eviction and active HTTP ID reuse remain limitations.
- [ ] Design and prove a memory-bounded 20-waiter Page navigation profile
  before changing capacity constants. This exact test needs 20 Page slots
  and 20 Frame host operations, not 21; Page.goto is separate. The current
  32-owner × four-slot arena declaration is at least 66.78 MiB, while a
  naive 20-slot increase declares at least 333.91 MiB. Prefer a narrow opt-in
  burst profile or global lifetime-permitted 20-waiter pool, verify bounded
  rejection/reclamation and memory peak, then rerun the unchanged 64-case
  official file. Route/result/FFI ceilings need not rise for this test.
- [x] Locally repair the `networkidle0` alias, invalid `networkidle2` error,
  and omitted-navigation-timeout Node wiring through existing Rust policy.
  Five focused Rust release and 29 Node adapter tests pass after review;
  the selected release native artifact is rebuilt and ABI/profile smoke
  passes. The later complete 64-case official rerun verifies these selected
  cases under the cumulative artifact. Undefined Context navigation-timeout
  reset has 9/9 Rust release and 9/9 Node adapter tests, and three typed
  backend error call-log cases have 4/4 Node checks; the same full-file rerun
  includes their selected official assertions.
  The cumulative release addon passes ABI smoke 1/1, the browserless Node
  contract suite passes 563/563, and TypeScript passes. Fractional timeout
  values remain open.

Official Python BrowserContext lifecycle conformance checkpoint (2026-10-10):

- [x] Pin three exact-source async Chromium manifests for Context/Page close,
  enumeration, event cancellation, viewport/scale, multipage isolation,
  lifecycle events, and popup identity. Candidate and independent oracle pass
  all 11 unchanged selected official cases through the dedicated CDP adapter.
  Strictly promote only those 11 previously `not-run` IDs over successive
  6,526-row ledgers; current candidate/oracle passed counts are 1,279/836.
- [ ] Run remaining applicable BrowserContext cases and other engines with
  explicit fixture policy and source-bound provenance; these 11 cases do not
  establish full Context or full Python corpus parity.

WebSocket transport / official Python fake-peer tranche (2026-10-10):

- [x] Add a standalone opt-in protocol-neutral `pwright-transport-ws` Rust
  boundary with checked endpoints/headers, explicit per-connection limits,
  injectable fake peer, and parser-accepted Close-frame reason separate from
  EOF. The optional `live-tokio` adapter performs direct client I/O only; no
  browser, CDP, Playwright dispatcher, listener or background task is selected.
- [x] Canonicalize validated endpoints before the live handshake, reject
  unescaped control/whitespace bytes, and retain a parser-accepted Close
  receipt while driving Tungstenite's queued reply on explicit `close()`.
  Keep canceled/failed close attempts retryable with deterministic fake tests.
- [ ] Resolve Tungstenite 0.26's full-handshake TRACE logging of sensitive
  caller headers before any production composition selects `live-tokio` with
  credentials. The leaf does not install a global logger workaround.
- [ ] After isolated leaf tests and dependency proofs, thread selected headers
  and typed terminal reasons through the existing CDP connector and host, then
  thin Python/Node bindings, without changing default profiles.
- [ ] Build a separate Playwright remote-protocol adapter for
  `BrowserType.connect`; CDP attachment is not that protocol. Prove successful
  remote initialization before claiming a working remote Browser.
- [ ] Resolve the five unchanged official Python fake-WebSocket test bodies'
  literal `localhost`/`127.0.0.1` endpoints against the required Tailscale-only
  fixture bind. Do not promote adapted-endpoint diagnostics or this leaf's
  implementation tests into either 6,526-row strict ledger.

Current owned-launch implementation checkpoint (2026-10-10):

- [x] Complete the pinned browserless Python `BrowserType.executable_path`
  metadata tranche across Firefox and WebKit, separately per engine, with
  their exact official v1.58.0 revisions installed. Candidate and oracle each
  pass 4/4 full metadata cases per engine; all four schema-5 sidecars reverify
  against the exact archived capture sources, not the later formatted live
  tooling. Promote only the four previously `not-run` async/sync path keys
  over the Chromium-only strict successor.
  Both 6,526-row all-engine ledgers now record all six path keys passing:
  candidate 1,268 passed / 11 skipped / four failed / 5,243 not run
  (SHA-256 `e6ab49adfd8a2f64b9d2736e43328917faab17437d2c373076f0ceccb5d14e22`),
  oracle 825 passed / ten skipped / four failed / 5,687 not run
  (SHA-256 `cabcfc78429d12e962fefd8df8ddc3e4c9e77ff842e551b02dc4c4e814be92a1`).
  `docs/python-official-conformance.md` records exact reports, source-bound
  provenance and ledger paths. No browser was launched; positive Firefox/
  WebKit launch, `PLAYWRIGHT_BROWSERS_PATH=0` root parity and full-corpus
  compatibility remain open.
- [x] Run the unchanged pinned Python v1.58.0 async/sync Chromium
  `BrowserType.executable_path` and adjacent `name` cases against an isolated
  candidate wheel and independent oracle in exact-source, browserless owned
  mode. Both full reports pass 4/4 with capture-time-verified schema-5
  provenance binding the installed Chromium revision 1208 executable. Promote only the
  two previously `not-run` path rows, leaving the already-passing name rows
  untouched. Both 6,526-row strict ledgers change exactly two keys:
  candidate 1,264 passed / 11 skipped / four failed / 5,247 not run
  (SHA-256 `97693577b5a02e2c2358372fcef6db9b575af58e3fb67492ee01f83a50b184a2`),
  oracle 821 passed / ten skipped / four failed / 5,691 not run
  (SHA-256 `71bf7132c80a286de32064b461f27228e14653fc6fa105c3b2291f9f357e0108`).
  At this Chromium-only checkpoint four Firefox/WebKit path rows were still
  `not-run`; the all-engine successor above promotes them. No browser was
  launched or positive launch behavior proved. The installed-wheel registry
  provenance probe rejects upstream `.egg-info` shadowing; 59 focused tool
  tests pass.
- [x] Run the pinned Python v1.58.0 async invalid-numeric-proxy test across
  Chromium, Firefox and WebKit in exact-source, browserless `owned` mode.
  Oracle passed 3/3; candidate initially failed 3/3, then passed 3/3 after a
  narrow `proxy.server` type check before its existing unsupported gates.
  Both schema-5 green sidecars verify against live and archived sources;
  before/after process snapshots showed no new browser PID. Only these three
  public-API keys advance in the strict 6,526-row ledgers: candidate 1,262
  passed / 11 skipped / four failed / 5,249 not run, oracle 819 passed /
  ten skipped / four failed / 5,693 not run. Valid proxy launch and positive
  Firefox/WebKit providers remain unsupported.
- [x] Run the exact Core `library/browsertype-launch.spec.ts:64` diagnostic
  under an OS process-exec allowlist and outbound-network denial. The pinned
  dummy Node asset, not Chrome, exited once in the oracle 1/1 pass. Candidate
  failed before dummy spawn on inherited `tracesDir` after the narrow
  `handleSIGINT: false` Node fix; the guard rejected its missing process
  receipt and produced no candidate sidecar. Both strict keys stay `not-run`.
- [x] Add an explicit owned-launch startup-log composition: the provider drains
  stdout/stderr with fixed byte/line budgets, and only Node host's owned
  launch selects it. Synthetic browserless provider/composition/host release
  tests pass; the default provider and attach path remain unchanged. This is
  implementation evidence only, not an official candidate pass.
- [x] Gate startup-log capture behind independent provider/composition Cargo
  features; provider release tests pass 19/19 (off) and 24/24 (on), while
  composition passes 15/15 (off) and 16/16 (on), with strict Clippy in both
  modes. Add a focused CI gate for
  these selections; GitHub Actions has not run it yet.
- [x] Create an independent opt-in Rust Playwright-v8 trace leaf with bounded
  JSONL context/API event encoding and caller-owned trace-directory
  preparation. Release tests pass 11/11 and strict Clippy passes; the default
  Node host does not depend on it. This does not yet record Context events or
  produce `.network`/ZIP artifacts.
- [x] Add a separate browser-independent Rust v8 Context lifecycle leaf with
  injected output, clock, ID source and resource ledger. Its bounded
  Idle/BetweenChunks/Recording/Poisoned state, TraceBytes lease, first-chunk
  start, chunk replacement/discard, group auto-close, and fail-closed output
  cleanup pass 19/19 release tests, strict Clippy and dependency proof.
  Context close/disconnect/cancellation must call explicit abort, and a
  production output port still needs RAII temp-file cleanup. The CI gate also
  checks default-host tree exclusion but has not run on GitHub Actions. No
  host or Node/Python wiring, `.network`/ZIP sink or official trace credit
  exists yet.
- [x] Rebuild the opt-in owned-launch Node addon and pass a browserless public
  Node-to-N-API-to-Rust synthetic-process smoke test (1/1), checking bounded
  logs, temporary-profile removal and child reaping. Add a Node 20 CI build,
  native-load and smoke gate; the new GitHub Actions step has not run yet.
- [x] Add an explicit `playwright-trace` Node native profile that prepares an
  inherited, nonempty `tracesDir` in Rust before executable validation,
  temporary-profile allocation or process startup. Preserve the caller-owned
  directory after failed launch; keep default and legacy `owned-launch`
  unchanged. Browserless native tests pass 4/4; Node package tests pass
  533/533, with types, boundary and selected native-load checks green. CI
  has a selected-profile gate but GitHub Actions has not run it yet.
- [x] Rerun the exact pinned Core line-64 dummy-process diagnostic on oracle
  and the selected `playwright-trace` candidate under the process-exec
  allowlist and outbound-network denial. Both pass 1/1 with public-launch
  and exact dummy exit-1 receipts. This is diagnostic-only: it does not attest
  browser profile ownership or promote either strict-ledger key.
- [ ] Design and implement an opt-in Playwright-compatible tracing capability:
  the selected profile now validates/creates `tracesDir` before launch and
  retains its path in the host, but must still carry it into Context and
  implement tracing start/stop/chunk output with the expected named `.trace`,
  `.network` and ZIP artifacts. The existing `.pwtrace` sink has a different
  contract and is not a substitute. Other profiles must keep inherited
  `tracesDir` unsupported; do not filter fixture defaults or special-case the
  dummy executable.
- [x] Correct the official Core classification audit for direct private
  `APIRequestContext._tracing` access. Exactly 27 `not-run` invocations across
  three tests at lines 636, 662 and 699 become implementation-internal; the
  two requested tests account for 18. Both 48,477-row audits share SHA-256
  `3e260fd6dca8c2c20a3b16c39c196c2bc34c191e1f1dbd59567f89056547a458`.
  No outcome or public-parity credit changes.
- [ ] For that trace profile, connect the new bounded v8 JSONL writer and
  Context state/lease core to the host and a separate `.network`/ZIP streaming sink.
  Verify integrated inactive `stop()`, group auto-close, chunk replacement,
  named files and ZIP members before
  enabling snapshots, HAR bodies, screencast, source capture or remote
  Artifact delivery. The current `.pwtrace` sink writes PWRTRACE v1 only and
  the model's group/chunk semantics differ from upstream.
- [ ] Review, do not yet adopt, initial trace budgets: four active Contexts,
  4,096 entries/chunk, 2 MiB retained payload/Context plus structural memory,
  one export/Context and a 64 KiB I/O buffer. Admit disk quotas separately;
  reject overflow with typed backpressure and prove lease release. The
  proposed first official tranche is Python async/sync inactive/start-stop,
  then Core inactive stop and no-snapshot ZIP; named `tracesDir` snapshot/HAR
  tests follow only after those capabilities exist and browser policy permits.
- [x] Run the two remaining Firefox/WebKit Core Library invalid-numeric-proxy
  keys under the browserless exact-source Core454 manifest. Both oracle and
  candidate have two expected outcomes; four schema-2 sidecars verify against
  pinned source and the exact candidate source/addon archive. Strict
  48,477-row ledgers advance only those two rows: candidate 1,685 expected /
  seven skipped / ten unexpected / 46,775 not run; oracle 1,388 expected /
  six skipped / nine unexpected / 47,074 not run. Full classification audit
  retains its SHA-256; positive proxy/browser paths remain unverified.
- [x] Correct the pinned Core454 viewport source-classification false positive
  without changing either outcome ledger. The test at
  `tests/library/browsercontext-viewport.spec.ts:50` directly reads private
  `deviceDescriptors` imported at line 22; only its nine Library/Bidi
  invocations become `implementation-internal`, while all nine stay `not-run`
  with zero attempts. Paired 48,477-row v2 audits share SHA-256
  `8084c4d5c1b75660cd709936e23aeb9f6ab812bd5556a1a38ed4ce13b2d2f365`;
  Core454 candidate 1,685 expected / seven skipped / ten unexpected /
  46,775 not run and oracle 1,388 expected / six skipped / nine unexpected /
  47,074 not run are unchanged. The bounded scan found zero additional safe
  browserless public-API rows among source-only utility/parser, APIRequest,
  and launcher cases.
- [x] Complete the pinned direct-private-symbol classification audit without
  changing Core454 outcomes. Exact test-body use of private `outofprocess.start`
  at `library/browsertype-launch-selenium.spec.ts:181` (nine rows), private
  `globToRegexPattern`/`urlMatches` at `page/interception.spec.ts:73` (11),
  and private `asLocator` at `page/page-aria-snapshot-ai.spec.ts:72` (11)
  changes only those 31 `not-run` invocations from private-helper to
  `implementation-internal`. Sibling public tests keep their classification.
  Paired 48,477-row v3 audits share SHA-256
  `bfcb82ba51e95c9968daeb04d8c4a42f02eb9830ca64a265a02fb479b2f2be3a`;
  the Core454 outcome counts and SHA-256 remain unchanged.
- [x] Run a distinct implementation-internal, browserless source-only Core455
  tranche: pinned `browsercontext-viewport.spec.ts:50` and
  `page/interception.spec.ts:73` each expand to nine Library/Bidi invocations;
  Android/Electron are excluded. Oracle and frozen candidate each pass 18/18
  under a pre-module no-I/O preload; all eight schema-3 sidecars reverify.
  Strict successor ledgers change only those 18 `not-run` keys to `expected`,
  with `evidenceScope=upstream-private-source-only` and
  `candidateRuntimeRequired=false`: candidate 1,703 expected / seven skipped /
  ten unexpected / 46,757 not run (SHA-256
  `5cc090bd05c928255cc9da69d9e535879c59af467c4ecb89445a827257cef14e`),
  oracle 1,406 / six / nine / 47,056 (SHA-256
  `eb54dc153626706a97e54fad339f1c610e0d9f23232a7c614f74da59ef1bda6e`).
  Full classification SHA-256 stays v3. These 18 passes add zero public-API
  parity evidence; Pwright is not needed for either private test body. The
  strict rows retain immutable inventory `sourceClassification` metadata;
  their corrected per-invocation internal label is in the separate audit.
- [x] Add a pnpm-only CI replay for all eight Core455 schema-3 sidecars. A
  fresh pinned checkout built and replayed under Node 20 / pnpm 10 locally;
  both lanes passed 18/18 with no browser and no tracked upstream changes.
  GitHub Actions has not run the new workflow yet, and these private tests
  still contribute zero public-API parity evidence.
- [x] Run the two previously unrecorded pinned Python async
  persistent-Context page-argument engine keys under `owned` mode. WebKit
  oracle passed and candidate initially failed; after a narrow preflight
  ordering repair the same candidate case passes. Firefox remains a shared
  official skip. Four schema-5 sidecars and exact source archives verify;
  strict 6,526-row ledgers advance only those two keys to candidate 1,259
  passed / 11 skipped / four failed / 5,252 not run, oracle 816 passed /
  ten skipped / four failed / 5,696 not run. Python package units pass
  312/312 with Ruff/Pyright green; valid WebKit launch remains unsupported.
- [x] Preserve the historical manifest and add a separate exact-only Python
  browserless manifest/runner/verifier guard for the same WebKit pass and
  Firefox official skip. All four new schema-5 sidecars verify live and from
  the immutable archive; the 6,526-row successor ledgers refresh only those
  two evidence links, with unchanged statuses and classification SHA-256.
  Oracle starts its Node driver but no browser/CDP process.
- [x] Execute 18 pinned browserless Core persistent-Context preflight
  invocations across nine Library/Bidi projects. Core452 oracle records 17
  expected / one skip; candidate records 17 unexpected unsupported-boundary
  failures / the same skip. Preserve all verified sidecars and immutable
  48,477-row strict ledgers; classify nine page-argument rows as ordinary
  public API and nine certificate rows with the incidental private-helper
  import, without merging those categories.
- [x] Add a composable persistent-Context preflight leaf, preserve the
  unchanged official preload while conveying fixture default channel through
  an inherited non-enumerable setter, and rerun the same 18 invocations in
  core453. All 17 non-skipped candidate calls now match oracle, nine new
  source-bound sidecars verify, and the immutable 48,477-row candidate ledger
  has 1,683 expected / seven skipped / ten unexpected / 46,777 not run.
  All three Node profiles pass 511/511 package units; valid persistent Context
  launch remains unsupported until native provider and process/profile
  lifecycle implementation.
- [x] Run 12 pinned browserless Core Library/Bidi launch-argument invocations
  in core450: oracle 12/12, candidate 3/12, nine genuine public-API
  Firefox/WebKit profile/page-argument error mismatches. Preserve all 16
  source-bound sidecars and unchanged 48,477-row strict ledgers.
- [x] Archive the pre-fix candidate, apply narrow engine-specific Node
  preflight parity, and rerun the identical 12 official invocations in
  core451: candidate 12/12, matching the prior oracle rows; eight new
  source-bound candidate sidecars verify. The candidate strict ledger is
  1,666 expected / ten unexpected / six skipped / 46,795 not run. Five new
  AAA adapter units pass; the later browser-registry profile checkpoint has
  490/490 package units in each of three Node profiles.
- [x] Extract persistent-Context certificate-shape and engine-argument
  decisions into the zero-third-party-dependency
  `pwright-persistent-context-preflight` Rust leaf and wire a separate opt-in
  Node N-API profile. JS still finishes `Promise.all` file reads, Buffer
  truthiness, boxed-string normalization, fixture-default merging and public
  `Error` construction. Pure Rust release tests pass 28/28; default,
  owned-launch, browser-registry and new opt-in Node profiles pass 512/512
  package units each. Real selected addon tests pass 4/4, pinned guarded
  browserless public-method comparison, and exact dependency-closure
  proof adds only this leaf. No official corpus row is promoted by extraction.
- [x] Match pinned protocol unboxing for boxed-empty certificate `origin`
  and `passphrase` before JS/Rust truthiness checks. Both default and opt-in
  browserless public-method differentials now pass 14/14 after reproducing
  the pre-fix candidate failure. Make the native-test script self-select its
  profile so four skipped tests cannot be reported as a successful native gate.
- [ ] Decide how to move the still-active default JS persistent-Context
  preflight to Rust without regressing the default profile, then consolidate
  `BrowserType.launch`/`launchServer` argument decisions under an explicit
  composable contract. The opt-in addon retains the pre-existing host tree;
  it does not prove positive TLS/persistent launch. The default JS profile
  still lacks pinned classic Firefox `-juggler*` rejection.
- [x] In the opt-in Rust preflight only, match classic Firefox `-juggler*`
  rejection after profile misuse, excluding `moz-*` BiDi channels and truthy
  non-array `ignoreDefaultArgs`. The pinned, browserless public comparison
  covers 16 rejection and three accepted-preflight inputs; selected focused
  Node tests pass 23 with one intentional default-only skip. Default JS
  behavior is unchanged, so this does not close its separate parity gap or
  promote an official corpus row.
- [x] Accept boxed `String` channel/args in Node persistent-Context
  preflight, matching the pinned protocol's string conversion before
  default-argument decisions. Dedicated browserless tests pass 9/9,
  including seven pinned public-method comparisons; this is source-derived
  differential evidence, not official corpus promotion.
- [ ] Extend pre-spawn parity for client option ordering, protocol type
  validation, userDataDir types, TLS certificate parsing and proxy errors.
  Keep each newly proved edge in a separate bounded capability and do not
  treat the existing 18 official preflight rows as full persistent launch.
- [ ] Investigate the five remaining `unexpected:public-api-candidate` Core
  browser-backed rows in a policy-compliant dedicated Chrome/CDP task: one
  cached image response wait timeout, three missing legacy device
  orientation/motion initializers, and one upstream extra-header XPASS. All
  five are oracle-unexpected too; do not label them candidate-only or change
  their strict outcome statuses without new evidence.
- [x] Execute five pinned browserless Core Library BrowserType metadata
  invocations. Candidate/oracle each record two public `name()` passes and
  three `executablePath()` unexpected outcomes; six source-bound sidecars
  verify and core449 promotes only those five keys. The latter test accesses
  private `_playwright._defaultLaunchOptions` before the public method, so
  its failure does not prove a candidate public API mismatch. The corrected
  full invocation audit labels these three run and six unrun Bidi variants
  implementation-internal, without changing any outcome ledger.
- [ ] Design a narrow, explicit fixture adaptation if the upstream
  `executablePath()` test is to become a public-API comparison; retain the
  unchanged-source private-hook result separately, and resolve the oracle's
  missing v1.58.0 browser-cache executable before claiming parity.
- [x] Expose opt-in synchronous `BrowserType.executablePath(): string`
  through the existing Rust browser-registry leaf, without installing or
  launching browsers. Native 4/4, adapter 13/13, three Node profiles 490/490,
  types/native-load/boundary and direct pinned public-method comparison 3/3
  pass. The new feature adds no provider/install dependency beyond the
  addon's existing host base; it is not a host-free binary. Do not promote
  core449: its private precondition fails before this method.
- [x] Match upstream registry-directory environment snapshot timing in the
  opt-in profile. Import-time JS input capture feeds the Rust path policy;
  13/13 pinned fresh-process public-method comparisons, 9/9 Rust leaf tests,
  and 511/511 package units in all three Node profiles pass. The old
  two-argument addon fails closed. This is verified on the current Mac only.
- [ ] Separately design a public-only official `executablePath()` comparison
  without private test hooks or relying on an absent pinned browser cache.
- [x] Build an isolated ABI107 Python wheel from the final Rust host guard,
  import its native extension outside the source tree and pass 308/308
  browserless Python units. Archive SHA-256
  `77b485d083121a08f0dea4fe1b9183c65ebda2b9a7e4ff3aa74f4d8726718c3a`;
  do not replace the source/official-lane module or promote official rows.
- [x] Run 18 additional pinned Bidi browserless preflight invocations in
  candidate and oracle (18/18 each), with ten verified Core schema-2
  source-bound sidecars. Core448 advances only those 18 public-API keys:
  candidate 1,652 expected, oracle 1,355. Positive launch remains unproven.
- [x] Run 24 pinned browserless Bidi Library preflight invocations (four
  argument/error rows across six projects). Candidate and oracle each pass
  24/24 with verified source-bound provenance; core447 promotes exactly
  those keys to `expected` and all classify as public API candidates. No
  browser/CDP was used; Bidi positive launch remains unproven.
- [x] Run pinned Core Chromium `launchServer` remote-debugging-pipe
  pre-spawn rejection without a browser. Candidate/oracle each pass 1/1,
  source-bound provenance verifies, and core446 advances only that key:
  candidate 1,610 expected, oracle 1,313. This is not BrowserServer support.
- [x] Execute the pinned Core invalid-executable path test for Firefox and
  WebKit Library without spawning either browser. Candidate/oracle each pass
  1/1 per engine with four verified sidecars. The strict core445 ledgers
  change only those two keys; candidate 1,609 expected, oracle 1,312.
- [x] Archive the prior `owned-launch` Node addon and exact package source;
  all nine latest core447–449 candidate sidecars reverify against the archive
  after rebuilding the opt-in addon from the host guard. Its native-load
  smoke, owned-launch adapter, 477/477 Node units in both profiles, TypeScript
  and dependency-boundary checks pass. This does not prove live hidden-ID
  isolation or positive owned launch.
- [ ] Rebuild/verify the default Node addon and editable Python extension
  against the final fresh-owned host guard; the optional Node addon and
  isolated ABI107 Python wheel are verified. Rust all-features release check
  and 76/76 focused browserless release tests pass; async event publication
  and exhaustive entrypoint review remain open.
- [x] Archive the exact ABI109 `python/pwright` tree and provenance-bound
  harness/tooling before replacing the editable Python extension. Seven prior
  candidate sidecars reverified against that archive; the wheel's ABI107
  extension and two bundled OpenSSL dylibs were installed, source import
  resolved to ABI107, and 308/308 browserless units passed. The old strict
  ledger is unchanged.
- [x] Extend Python official provenance to schema 5: candidate records the
  exact adjacent `shuozeli_pwright.dylibs` members with absolute paths and
  SHA-256, oracle records an empty list, and `--package-root` verification
  requires a matching sibling directory. Focused tests pass 57/57; historical
  schema-1 through schema-4 sidecars remain unchanged.
- [x] Capture and verify three fresh schema-5 candidate-owned official glob
  results against the editable ABI107 extension and both sibling dylibs;
  reverify each against the exact package/tooling/harness archive. Refresh
  only those three already-passing keys in a new 6,526-row strict candidate
  ledger. They remain implementation-internal, not public-API parity proof.
- [ ] Validate the source patches for two hidden-ID publication gaps in
  rebuilt optional Node/Python artifacts: fresh-owned Worker and
  Worker-console notice filtering by owning published Page, and rejection of
  collection-hidden storage-state scratch Page/Frame from public admission.
  An exact task-local permit keeps private scratch operations and cleanup;
  unpublished indexed origins fall back to scratch. Focused Rust release
  tests pass, but FFI and real-browser checks are pending. Also decide/
  test whether the hidden default Page consumes a public Page slot.
- [ ] Decide whether Rust-only `HostSession::page_snapshot` should require
  published Page membership like the Node FFI projection; verify the
  new-Page publication timing window before changing legacy Rust semantics.
- [x] Execute pinned Python browserless `test_should_work_with_glob` in
  candidate/oracle across all three engines. Six sidecars verify; the ABI109
  strict ledgers add two candidate and three oracle passes. Formal
  applicability is `implementation-internal`/`non-applicable` because of
  upstream private imports, so this advances corpus execution only.
- [x] Promote three more pinned Core Chromium Library browserless argument/
  proxy rejections. Candidate and oracle each pass 3/3 with source-bound
  provenance, and the strict core444 ledgers advance exactly those keys to
  `expected`: candidate 1,607, oracle 1,310. These do not start a browser.
- [x] Run all eight remaining pinned Python Firefox/WebKit preflight rows
  separately by engine without launching a browser. Candidate and oracle each
  record six passes and two upstream skips, with verified source-bound
  sidecars. Exactly eight more strict ledger keys advance; the Python totals
  are candidate 1,256 passed and oracle 812 passed. Combined-engine fixture
  ordering failed diagnostically and was not promoted.
- [x] Promote pinned Core browserless invalid-executable launch preflight:
  candidate and oracle each pass 1/1 with verified provenance. The core443
  48,477-row ledgers change only this key from `not-run` to `expected`;
  candidate is 1,604 expected, oracle 1,307 expected. This validates the
  public Node error path, not Rust-owned positive launch.
- [x] Expose opt-in public Node and Python Chromium `BrowserType.launch`
  through the Rust-owned process handoff, explicit executable path and unique
  temporary profile. Python source/native is ABI107; package units pass
  308/308, Node units 472/472, with corresponding earlier release/type/boundary
  checks green. Default CDP attach remains available.
- [x] Collect the 12 expanded pinned Python async/sync launch-preflight rows
  in separate candidate/oracle `owned` runs under a stable interpreter;
  schema-4 sidecars verify. Collection-only JUnit is not execution evidence.
- [x] Run the four Chromium invalid-page-argument/nonexistent-executable
  rows against unchanged official tests in both candidate and oracle. Each
  lane passes 4/4, both sidecars verify, and exactly four `not-run` strict
  ledger keys advance to `passed`. These error paths start no Chrome process.
- [x] Run candidate-only genuine Chromium owned launch/close twice against
  the two pinned official Python async/sync tests. Both pass with a verified
  source-bound sidecar and no matching temporary-profile Chrome process in a
  post-test snapshot. Do not promote: oracle remains unrun under the global
  Pwright-only browser rule, and process/profile identity is not yet bound.
- [ ] Prove positive public owned launch, Page/Context behavior and close
  against paired pinned official tests; execute the distinct Core owned-launch
  lane without the historical `launch -> connectOverCDP` adapter.
- [x] Prepare that distinct Core one-spec lane for pinned Chromium
  `should be callable twice`: exact-source manifest, real public-launch
  wrapper, release owned-launch addon and browserless 1/1 official collection.
  Runner checks pass 10/10 plus 31/31 existing; no browser/spec was run.
  Any result remains diagnostic until process/profile attestation and a
  narrow exception to the mandated `chrome-cdp` workflow are settled.
- [x] Repair fresh-owned Python Browser/Context close-event ordering in the
  façade, preserving attached-CDP behavior. Async/sync/CDP-guard units pass
  and the package suite is 308/308; freeze the two official close-event cases
  in an exact-source manifest. Official execution remains pending.
- [x] Make owned-browser host shutdown one-flight: concurrent callers wait
  for the same retained provider report, and cancellation of a waiter cannot
  abandon an in-progress shutdown. Two fake-future release tests pass.
- [ ] Expose an opt-in private diagnostic receipt binding executable,
  temporary profile, direct-child PID and final shutdown report; prove
  descendant cleanup and report profile-removal failures. Current host
  retention is not yet visible to Node/Python or official provenance.
- [ ] Make the hidden native default Context/Page/Frame completely
  unaddressable through all direct and generic host entrypoints; prove
  process-tree cleanup and reaping on shutdown and failure.
- [x] Execute or account for the remaining eight selected Firefox/WebKit
  Python preflight rows without claiming positive engine support. The
  Firefox page-argument pair is upstream-skipped in both lanes.

Earlier owned-launch preparation checkpoint (2026-10-10):

- [x] Arm the optional Chromium composition's successful provider lease before
  binding; fake-process release tests cover abandoned takeover and cancelled
  cleanup without launching Chrome. The provider's raw lease semantics remain
  unchanged. Process-tree supervision and confirmed reaping are still open.
- [x] Add a Rust-host Fresh Chromium entrypoint sharing the persistent owned
  launch handoff, with an empty caller-owned profile and collection-level
  hiding of the native default Context. It is a Rust-only slice, not a public
  Node/Python `BrowserType.launch`; guessed native IDs still bypass some host
  visibility checks, so strict unaddressability remains open.
- [x] Separate Python official `cdp-adapter` and `owned` harness modes. Owned
  mode disables both launch monkeypatches and fails closed before pytest when
  the real public launch API is missing. New schema-4 provenance and JUnit
  launch-mode labels prevent accidental mixing; 74 browserless tests pass.
  Six historical ABI106 sidecars verify with archived pre-change sources.
- [x] Freeze the first four unchanged Python async/sync launch preflight
  functions in an exact-source manifest; the pinned checker verifies their
  upstream source/fixtures. The selection expands over three engines, but no
  collection report or executable outcome has yet been captured.
- [x] Wire public Rust-backed Python/Node launch and run the first four paired
  owned-mode Chromium error-path cases. Remaining native-ID admission,
  process-tree cleanup and positive owned-browser conformance are open above.

Browserless cross-engine conformance and evidence boundary (2026-10-10):

- [x] Execute the exact 15 previously unrun Core BrowserType Library/BiDi
  invocations against candidate and oracle, preserve the candidate's eight
  Firefox/WebKit failures, then repair only thin BrowserType metadata/error
  exports; fresh schema-2 candidate and oracle reports agree on 11 expected
  and four upstream skips without launching a browser.
- [x] Promote exactly four paired-green Firefox/WebKit async/sync Python
  `BrowserType.name` rows; keep all six `executable_path` rows diagnostic until
  the oracle's pinned browser revisions and candidate's optional metadata
  implementation are both verified under a recorded environment.
- [x] Add an opt-in, browserless Rust registry for pinned v1.58.0
  Chromium/Firefox/WebKit executable-path metadata and Python ABI106. In
  isolated six-pair official diagnostics, candidate and oracle return the same
  per-engine path strings, but the unchanged upstream existence assertions
  fail in both lanes because those exact binaries are absent. Keep all six
  path rows red and unpromoted; this is metadata parity, not browser support.
- [x] Reproduce the pinned Core invalid-channel `BrowserType.launchServer`
  preflight error without spawning any browser/server or loading native code;
  the exact official candidate/oracle row is paired-green and Node 450/450
  units, types and boundary checks pass. The same 15-key BrowserType matrix
  was rerun on this exact candidate source: 11 expected/four upstream skips in
  both lanes, with four fresh source-bound sidecars verified.
- [x] Expand the pinned browserless BrowserType preflight selection to all
  three Library engines (12 exact official rows). Candidate and oracle pass
  12/12 with six verified sidecars; 11 rows advance from `not-run`. Rerun the
  15-key matrix against this final Node source and verify four more sidecars:
  11 expected/four upstream skips in both lanes. Node 458/458 units, types and
  boundaries pass; valid launch/server/connect capabilities remain absent.
- [ ] Design a real composable BrowserServer/launchServer capability for valid
  requests; the preflight facade returns `PWRIGHT_UNSUPPORTED` by design.
- [ ] Finish the 12-row async/sync launch-preflight selection in the genuine
  owned-launch lane. Four Chromium rows now have paired-green provenance and
  strict-ledger promotion; eight Firefox/WebKit rows remain unexecuted. The
  exact upstream selection uses Tailscale-bound autouse HTTP/HTTPS fixtures.
  Do not treat negative preflight as positive engine-launch parity.
- [x] Bind new Core/Python official capture sidecars to fixed local
  runner/helper source hashes, retain schema-1 reports as `legacy-unbound`,
  and fail closed on source drift or symlink/path escape.
- [x] Give all 6,148 Core product-infrastructure rows explicit source/role
  reasons without silently marking them non-applicable.
- [x] Implement opt-in schema-3 provenance for future pinned Python
  `executable_path` captures: bind child resolver environment, official
  `browsers.json` revisions, resolved cache/path and existence before/after;
  preserve historical schema-1/2 sidecars through an exact-source archive.
- [ ] Rerun pinned Python `executable_path` diagnostics under schema 3 after
  resolving browser-install/policy scope; do not promote the six current red
  rows merely because the path strings match. Schema-3 verification is
  intentionally sensitive to later env/install changes, not a portable
  archival attestation.
- [ ] Decide formal applicability for each product-infrastructure subsystem;
  a classification tag alone does not justify exclusion.
- [ ] Run the staged secure Core dispatch fixture only after the shared CDP
  Chrome tab is available, and resolve the separate retained-lease warning.
- [ ] Execute and account for every remaining applicable official Core and
  Python invocation; selected-file parity is not full-corpus parity.

[Core matrix evidence](upstream-playwright-conformance.md) and
[Python metadata evidence](python-official-conformance.md#cross-engine-browsertype-metadata-checkpoint-2026-10-10).

Launcher ownership and honest-oracle design checkpoint (2026-10-10):

- [x] Audit pinned Playwright v1.58.0 `launch`, persistent launch and
  `launchServer` against the existing Rust, Node and Python surfaces. A
  fresh, nonpersistent launch returns `Browser` without a default Context;
  persistent launch returns its Context. `launchServer` has a separate
  process-owning WebSocket lifecycle. These are three contracts, not one
  CDP-attach adapter.
- [x] Fail closed on Python `launch_persistent_context("")`: the previous
  `Path("").resolve()` mapped it to the current working directory, whereas
  upstream uses an owned temporary profile. The current explicit-profile
  implementation now rejects that unsupported case before native code;
  sync/async regression tests and the 299-test unit suite pass. This is a
  safety repair, not upstream empty-profile parity.
- [ ] Add a separate opt-in Rust Chromium **ephemeral** launch owner, with an
  explicit validated executable or installed-browser receipt, unique
  temporary profile, no default Context, immediate kill-on-drop lease
  takeover, child-tree supervision, cleanup after process exit, and
  cancellation/listener-failure tests. Do not implicitly download a browser.
- [ ] Expose that owner through a typed Node/Python `BrowserType.launch`
  feature, returning `Browser`; preserve the existing persistent-Context
  feature separately. Valid Firefox/WebKit launch remains unsupported until
  real engine providers exist. Share early option validation across engines
  only as preflight coverage, not as browser-support evidence.
- [ ] Build an independent `BrowserServer` owner and a versioned Playwright
  WebSocket protocol adapter before exposing valid `launchServer` or
  `wsEndpoint()`. The current Pwright server wire and CDP endpoint are not
  interchangeable with that protocol. Specify authentication, Tailscale-only
  binding, client Context isolation, disconnect cleanup and process authority.
- [ ] Resolve the literal upstream `launch server > should work with host`
  case before running it. Its source passes `host: '0.0.0.0'` and asserts that
  address appears in `wsEndpoint()`, conflicting with this Mac's Tailscale-only
  server-binding rule. The inventory retains three default-engine Library
  rows plus six non-default BiDi rows that upstream marks for runtime skip;
  all nine remain `not-run` in the strict ledger. Do not start an all-interface
  listener, rewrite the assertion silently, or classify these rows N/A from
  static inspection alone. Decide a safe isolated execution or a formal
  security-policy exception with explicit per-row accounting.
- [ ] Add an official **owned-launch** conformance lane that disables both
  candidate and oracle `BrowserType.launch -> connect_over_cdp` monkeypatches;
  record exact browser revision, executable, profile/fixture ownership and
  process-tree RSS. Keep shared-CDP adapter reports historically labeled.
  Test negative preflight separately from successful launch. The current
  16-session FFI cap and 512-MiB event budget do not bound Chrome child RSS.

Proposed, not yet adopted, product-scope policy: keep classification separate
from a per-invocation applicability decision. The 6,148 Core product rows
partition exactly into stress 3,333, Playwright Test 1,720, MCP 900,
installation 59, Electron 35, Android 34 and examples 67. At least 3,303
stress rows directly exercise public lifecycle/Page/Locator/launch behavior;
they cannot be excluded as a group and require an adapted Rust-backed harness
before any result is promoted. The 300 browser-cycle rows additionally depend
on unresolved launcher ownership. Mixed private stress assertions need
portable replacement contracts, while upstream object-count internals need
explicit reasoning plus Pwright-owned RSS/lease/Handle budget tests, since
memory use is a central goal. Playwright Test contains public connection and
APIRequest obligations even if its runner is out of product scope. MCP,
package installer, Electron, Android and application examples each require an
explicit product-scope decision and extraction of any shared public-library
behavior. A blocked
browser environment or unimplemented launcher is a gap, not a non-applicable
result. No product row has been removed from either strict ledger.

Dispatch-event source/artifact and selected-file comparison checkpoint
(2026-10-10): The unchanged Playwright Core v1.58.0
`tests/page/page-dispatchevent.spec.ts` source (SHA-256
`0af6f0562a67f391939881030426b6c0c4c0b4d77ba7d88bd64160b75231e36e`)
is pinned as one complete 17-case Chromium Page file by
`tests/official/playwright-core-page-dispatch-event-manifest.json` (SHA-256
`4bb9714f113992367330d0578f2862b0ced17d766c69310faaca655310b06137`).
The separate Python v1.58.0 async source
`tests/async/test_dispatch_event.py` (SHA-256
`a4905396220267094de57d7fc721c8d733d4f0e453f37295882338c9ebe31c19`)
has 12 exact Chromium cases in
`packages/pwright-python/tests/official/page-dispatch-event-manifest.json`
(SHA-256 `acbe71f2b9922e5040b17c2bcbcb3946b9096964f314de0f63ddc909353d0128`).
The pinned Python tag has no `tests/sync/test_dispatch_event.py`; its sync
Locator dispatch case resides in `tests/sync/test_locators.py` and is not a
case from this 12-case file.

Node `Page.dispatchEvent()` now selects the first matching Locator unless
`strict: true`; `ElementHandle.dispatchEvent()` uses the retained element owner.
Rust owns selector resolution, static DOM event construction (including
DeviceOrientationEvent and DeviceMotionEvent fields), and the same-world
owned-JSHandle/DataTransfer initializer route. Plain serializable initializers
retain the atomic Locator evaluation path. The distinct opt-in
`official-conformance-page-dispatch-event` `.node` artifact hashes to
`ff8e5db65ab83dca3e7f53fb22b493c81167ece9f7afcdb40e1424d2327ef34d`;
it uses the existing official Page feature selection with an explicit native
ElementHandle dispatch port. Node package units pass 442/442, TypeScript and
dependency-boundary gates pass, native ABI/profile/port smoke passes 1/1, and
strict all-feature release Clippy passes. A separate task-owned Chrome smoke
carried one live DataTransfer JSHandle through Page and ElementHandle dispatch
(2/2), with zero active leases at that attempt's explicit close; it is not an
official case run or a general lifecycle proof. A later scoped registered-
selector atomic-dispatch probe reached its event assertion (1/1) but reported
17 active leases and 8,844 retained event bytes after explicit close;
cleanup is unresolved under the shared Chrome state. A scoped
`deviceorientation` probe on `about:blank` failed at
`initDeviceOrientationEvent` identically in the candidate and installed
Playwright 1.58.2. The pinned v1.58.0 source contains that fallback; this
diagnostic proves neither the pinned official case nor a candidate-only bug.

Python async/sync Page, Locator and ElementHandle dispatch facades, including
the nested live-JSHandle owned route, are implemented. Stable-source ABI105
`abi3-py39` wheels v2/v3 hash to
`229a2fd6c2bc024c3b06943d53538217d0c70168e0e01675b52f25302fd13165` /
`3892b19253d7ba4b050c14c437e26c2508eb8d9294d9e1a6a7debb3d3a9a0d4d`.
Their normalized payloads reproduce except generated CycloneDX
`serialNumber`/timestamp and the dependent SBOM hash in `RECORD`. A fresh v3
installation exposes ABI105 and both owned dispatch methods and passes 287/287
Python units; Ruff, strict Pyright and release addon check pass. The pinned
Core oracle v1 and candidate v2 each record 14 expected / three unexpected
outcomes on the 17-case file, with the same three device orientation/motion
failures on the HTTP fixture. Oracle report/provenance SHA-256:
`e5cc8d623a30203ebf336790c98fe3d19ca241de4d4196a6c21959c612fc627f` /
`c1b0486ccefc1a170d252678a1ee9306bb3495f015d4666807aede7253e7e95d`;
candidate v2:
`c62ed0843bc3e0cbe225bd2277d550eac6bb7d706959605af783023d2414e1b9` /
`88ce1497a6ec6d6d53c743778b0d2baf0e02bafc805c188641ec3dff8afda478`.
The exact 17-key HTTP v2 strict Core ledgers have 48,477 rows: candidate 1,579
expected / two skipped / seven unexpected / 46,889 not run (SHA-256
`c75186cd78e28259a48247e553b566a514944f9470bc103e98d57b8de6411f4b`);
oracle 1,282 expected / one skipped / six unexpected / 47,188 not run
(SHA-256 `65ec860c18b4b5e3e82e96f87167eb0fd36f170ca7f0c1e1e2f51b6886298f39`).
Candidate v2 is a verified historical source snapshot. After the JS
request-cache retirement patch, candidate v3 again recorded 14 expected /
three unexpected with the same per-spec outcomes as the pinned HTTP oracle;
its report/provenance SHA-256 is
`693db5b080912f67831f382c472bb17e7387d340ab33489307c5e23273b69522` /
`aca608efc75caa1497e07b6635f7b95f6aec968a6f06f82d3bf942d071df3b03`.
The verified-at-capture 48,477-row candidate ledger SHA-256 is
`cc21a501b7a12e575c870ff1f74eb0a3e3db89b9b8b0a0aa438bf129bc2f4560`.
The Core runner changed again for the secure-fixture lane after this capture;
do not claim that v3 provenance re-verifies against the present runner tree.
The Core and Python runners now reject pre-existing result/sidecar destinations
before launching a fresh test process (Python's explicit isolated-resume mode
remains separate), preventing a failed launch from re-stamping a stale report.
The secure Core validator also requires the exact 17 unique pinned spec IDs,
not merely a 17-test count. Historical sidecars did not bind runner/helper
source hashes, so these guards do not retroactively establish a fresh run.
An independent browserless Core `chromium-library` slice pins two unchanged
v1.58.0 BrowserType rows in
`tests/official/playwright-core-browserless-browsertype-validation-manifest.json`
(SHA-256 `6845bcc03f0e3e6e7753b07edce877338c6c3f56c7de95dee3e5f400e1862594`).
`browserType.name should work` is expected in candidate and oracle;
`launch server > should validate options` is expected in the oracle but
unexpected in the candidate because `launchServer` is absent. The exact
two-row strict merge advances candidate to 1,580 expected / two skipped /
eight unexpected / 46,887 not run (SHA-256
`556eafa00527dbf41cd7d494dcb2f2a9423272581af614ab6ea1a7f09f93faa7`)
and oracle to 1,284 expected / one skipped / six unexpected / 47,186 not run
(SHA-256 `1b08b44e3351da8442652333694aef39a0ac7ab07ff866dea7d3e5b5f10fdb3c`).
The `launchServer` discrepancy is a server-protocol design task, not just a
missing JS method: Rust has an opt-in owned Chromium launcher, but the Node
addon exposes only CDP attach and Pwright's current server wire is not the
Playwright JSON/WebSocket dispatcher consumed by `connect()`. Design an
opt-in Rust-owned server, protocol adapter, authenticated Tailscale-only
listener, and multi-client close/kill lifecycle before advertising a
`BrowserServer`-compatible API. The pinned upstream launch-server file has
108 inventory rows across nine projects; related connect/reuse/multiclient
tests expand the dependency surface, but are not automatically applicable
or passing. The upstream `0.0.0.0` host test conflicts with this Mac's
Tailscale-only binding policy and needs an explicit policy disposition.

The official Python async oracle and candidate each pass the selected 12/12
cases. Oracle XML/provenance SHA-256:
`45356b260b29ee850e5d32b38d0db94c07aea8332ddb27bc99c94490a33bcb1f` /
`3e8cfaa4e381ea3b8d5febeab9b5d65852ad123ea2a839af1c85c9ca25c469fd`;
candidate:
`71c1be307ec418904fadb30bb93edff99200ea86cb144936aaed5a35da233b36` /
`b742d93ac50ab46c1c93a0190719f6563c737aff2cfeeadff7ea86efbe8ac83f`.
After the async dispatch merge, the strict 6,526-row Python ledgers were
candidate 1,238 passed / eight skipped / four failed / 5,276 not run (SHA-256
`c5a31bf3be6d848b804a910a1de6409b09ded8fc8c361db8b1a12ad60c9039de`)
and oracle 792 passed / seven skipped / four failed / 5,723 not run
(SHA-256 `bcc3d23b593cc99fc1d753a6350e89f16b06dea3047afd495ee55fedffafcdef`).
This is selected-file parity, not full-corpus parity. A separate formal
browserless Python sync-lifecycle manifest (SHA-256
`dbad4ede06a7e3ae07d994a75e02a8868bc82191145e6c3746a880eb2fa2081f`)
runs the pinned six rows inside a same-thread asyncio loop; its wrapper SHA-256
is `7a089d2a16cbeec46926374571e96cf72cd8d3a513c76b0281c988be470e63c7`.
Candidate ABI105-v3 wheel and independent oracle each pass 6/6, with
candidate JUnit/provenance SHA-256
`6b08f1df918824cc60e95d9231ba4c6c84f2b0eb5bd81639e9890e88d4c85880` /
`085841dbbf0993c31a4b4c9942770e243115d03ecbe975b6c3b6cc46503c2e1d`
and oracle
`c1f3a1c464d78c225cca5cd10568017d1b6c0a27d9d10cfc7dd0810fbfddb2cd` /
`0c191f41c487998880d90608060c5fd3c8fc68f5e84612dab45e6cd956a01326`.
The old ordinary-pytest 3-pass/3-fail candidate/oracle baseline remains
preserved. Independent sidecar verification allowed exactly the six formal
loop-mode keys, and no others, to be promoted into new 6,526-row ledgers:
candidate 1,240 passed / eight skipped / four failed / 5,274 not run
(SHA-256 `e7cd534dbd40309ed7d258431face26a1b40d0f2c14c27834b493ec3fff5695e`);
oracle 796 passed / seven skipped / four failed / 5,719 not run
(SHA-256 `07cf927aa0a241b2075d9ec0f26c88e58daff0bf50e3d6cd646588ba056fc1d8`).
The next browserless Python `BrowserType.name` tranche selects the exact
async/sync `test_launcher.py` pair with manifest SHA-256
`e34e466352e5bb94304ba5dfa21655255877f6330b5ba236313c99dc97e0dac8`.
Candidate ABI105-v3 and independent oracle both pass 2/2; candidate
XML/provenance SHA-256 is
`de0479b59b4650342c82006b13406470241390cc25ebc34cab6665e0d1d142f2` /
`0703d2ff4b7ec441f1eec22553a10ac1de1dd5e43cd8fd8edf0a92bd8159a9e2`,
oracle `9ed208f5c431d5b689e11eecdf7516695e961db965d1ae1545920ee4a6047611` /
`3bb3b3a9cfd6cd02f4c91b7f6a754e6ba94682c98777511dbdfde79df37df21d`;
both sidecars verify. Its source checker allows the exact pinned two-case
fixture closure only when the source/fixture hashes match; other browserless
cases with fixture arguments remain denied. Only Tailscale-bound HTTP/HTTPS
fixture servers ran, not Chrome/CDP. Exactly two keys advance the 6,526-row
strict ledgers: candidate 1,242 passed / eight skipped / four failed /
5,272 not run (SHA-256
`ed31f1074ab675e75b8828e27a141a858144e8f85e408b9955d36fd1f2c6e4e8`),
oracle 798 passed / seven skipped / four failed / 5,717 not run (SHA-256
`2c02dffe0d102fe0d2dd660b30318097e455018c074140f73db5707067511852`).

Core fixture caveat: its Tailscale HTTP adaptation assigns the same MagicDNS
host and port to `PREFIX` and `CROSS_PROCESS_PREFIX`. Therefore the Core case
titled cross-origin navigation passes in both lanes without testing distinct
origins. HTTP MagicDNS is also not a secure context; the three shared sensor
constructor failures cannot establish secure-context parity. A revised
task-owned trusted-Tailscale-TLS dual-port fixture lane is staged with secure
manifest/preload SHA-256
`efc2f9f131c245baf7ddca262cd08ca3b5cee3d9f8d3d7a4248cbe55a5a03de2` /
`f3243c9f73c33a07a8eb62428505bd612f6248384f02ded14ffe2d42d219b8c2`.
It has browser-free checks only, **no official browser result**: a preceding
trailing-dot TLS attempt failed in preflight before tests, and the revised
dual-port lane has not run while shared Chrome belongs to another task.
Different ports would separate origins, not necessarily sites or processes.
Lifecycle cleanup remains unresolved:
the later registered-selector probe retained 17 leases / 8,844 event bytes,
and no cause or fix is proven by the earlier zero-lease DataTransfer probe.

NODE-434 (Page check/uncheck, paired Python and Core tranches): The unchanged
Playwright Python v1.58.0 async/sync Chromium `test_check.py` files contribute
18 exact cases in
`packages/pwright-python/tests/official/page-check-manifest.json` (SHA-256
`7e9b34f4ed0b0c193e213375ad2fc8dad6fbd13b622cda3f77c5d2ecf51809ae`).
The preserved candidate v1 is 0/18, failing at missing `Page.check()` /
`Page.uncheck()` methods; independent oracle v1 passes 18/18. Candidate v1
report/provenance SHA-256 values are
`e7bd53be1ebb5598ad2c32b81b48815ed45938c109d457436f8b76fcae46774c`
and
`c7f5a90298b02b16bed002db9765289a53a41f7d0542d686ec3cdbaf37b4028d`;
oracle values are
`ed02b29dc95adc83c2f08d3584fa8b4a041dbfd2b0385061bca8504e10ece72e`
and
`f12a1290f16071659a873fabe7937162e41d26c386802569c5be3d593f74c0ac`.
Both sidecars verify. The red candidate 6,526-row ledger is 1,208 passed /
eight skipped / 22 failed / 5,288 not run, SHA-256
`7181ca1d518c24c69f9db6a0c9a85d85fd6384feca748139b9287216b9790e70`.
Before its 18-case merge, the old unprofiled oracle ledger was rebased by
exactly 25 pristine TodoMVC example IDs to the profiled 6,526-ID inventory
(SHA-256 `a937e7f5400ed2a0a565673893f2c132aaf7f9be29ff0916078a6d1740bdb045`).
The oracle merge is 780 passed / seven skipped / four failed / 5,735 not run,
SHA-256 `1bc6a8b0dbaa1a79471e60d4c7576e0512043c737e2f70dea80db56fe13cea47`.

Thin Python async/sync Page methods now delegate to the existing Rust-owned
Locator checked-state action. The repaired macOS `abi3-py39` wheel SHA-256 is
`85913b81bcc9d9c65533554b4b6f7f9ca423216ea08f43d682bc1244b2d5d46b`;
installed-wheel package units pass 271/271, with Ruff and strict Pyright
green. Candidate v2 passes the same 18/18 as oracle. Its report SHA-256 is
`2bc0177b7a6cc8f1ca7c1d4a6e20acfd728dbf819224ee88e69a7f07ad633e14`
and verified sidecar SHA-256 is
`8c374ce922b748f613d82980ba3aa6be3eb68cdcde76071d19af7e36c8a44573`.
Exact 18-key replacement yields candidate 1,226 passed / eight skipped /
four failed / 5,288 not run across 6,526 rows, ledger SHA-256
`82c8161800ec3acd18938d0f9d0ba80edf76ce66b0918d563a6923b93ef6ee74`.

At the 2026-10-10T05:01Z source snapshot, independent final-source
`abi3-py39` wheel builds A and B hash to
`11d0a7f348145e8c4e2150a78dc3afc5638571cc271e938558f62093090a4adf`
and
`0086c99813ee9f8dad7804c32235f272473d5de3d483fa24f36a2b681a793474`.
Their normalized payloads reproduce; only generated CycloneDX `serialNumber`
and timestamp fields, plus the resulting SBOM hash in `RECORD`, differ. A
fresh installation passes ABI104 and 271/271 package units. The selected
official 18/18 result above used the earlier `85913b81...` wheel, not either
05:01Z build. A browser rerun on these builds was deferred because a foreign
nonblank tab occupied shared CDP Chrome; no final-source official result is
claimed from this packaging checkpoint.

The unchanged Playwright Core v1.58.0 `chromium-page` file has 17 exact
cases in `tests/official/playwright-core-page-check-manifest.json` (SHA-256
`41787c922fd4690cc96cbf778e918132af57c778321afd7c470acec2ff8432b5`).
The preserved candidate v1 is 0/17 with 17 unexpected failures: 16 missing Page
methods and one missing `ElementHandle.boundingBox()`. Its report/verified
sidecar SHA-256 values are
`8ea21fc35c11196cabf5ee8a51ae2faac34ac97ce7ede4def10cc82a43bd31c1`
and
`854f70c28119023ab35abef4ab165a0351cb1ffd63896613e5b136a71533955c`.
The original baseline `.node` remains
`f229cbbc36691afaee8b8ba65d6dfffad2b3a8f2576647a1e186d5e558215384`.
The strict baseline 48,477-row candidate ledger was 1,548 expected / two
skipped / 21 unexpected / 46,906 not run, SHA-256
`c9990d41d41b1f622d5ce99e8dbb0796e30d6b0c8ba6cd2678e463fb1e0524d1`.
Independent oracle v1 passes 17/17; report/verified sidecar SHA-256 values
are `16b0b069164778d38e91b8979a0b7104aa1f8f2736f11abf32c1f69209689547`
and
`f8dde0aa3d09f2132e794f8e86e5f1e00b40c201777409afe29f1f97107701d3`.
Its strict ledger is 1,268 expected / one skipped / three unexpected /
47,205 not run, SHA-256
`d4a2878876e885d77297e47fe3f6bdaa91e72a4bc735448305c0cb205f8ea0a2`.

The opt-in `official-conformance-page-check` profile builds a new `.node`
artifact, SHA-256
`a991b225d0a3dece24d0a457b05898e6aacb8f6217eb9d79f5d710c33ea30531`.
Thin Node Page methods delegate to the selected Locator/Rust action. The
ElementHandle geometry read uses the original retained Handle and existing
geometry program. Final candidate v2 passes 17/17, matching oracle; report
SHA-256 is `58a5de733c8d052f64919a5ad6a738672ca059f72e787ee0a54a14b321d19375`
and verified sidecar SHA-256 is
`72895a0bdd0432f5d2f80877a98a98f47550d0caf0fbd56b06344f5b617d6a41`.
Exact 17-key replacement yields candidate 1,565 expected / two skipped /
four unexpected / 46,906 not run across 48,477 rows, ledger SHA-256
`f3176185ef59fc45cc49228c3bec688dcea6c935ab2451f21e138962156cdad8`.
Both final Core classification audits retain SHA-256
`32520af75e4ac14352a9060e6f7156900bb52c4a3d99fe927311ba7490328af0`.
Node package units pass 424/424, strict TypeScript and dependency-boundary
checks pass, and the selected host release set has 617 passes and 14
explicit ignores; strict addon Clippy also passes. The geometry path uses
`getBoundingClientRect()`, not Chromium's exact `DOM.getBoxModel`; equivalence
beyond the selected case is unproven. Both selected files now have paired
outcome parity, but complete Python and Core corpora remain open.

Python BrowserContext storage-state (nine-case red baseline): The pinned
Playwright Python v1.58.0 source at
`47a5d35ef4f815a2021349f86ae391f7c20c02d6` has nine exact Chromium
cases across the unchanged async (five) and sync (four) storage-state files.
`packages/pwright-python/tests/official/browsercontext-storage-state-manifest.json`
pins their source hashes and IDs (SHA-256
`5c18d9019844547ae881d988a81bc3d1f515cc87bc6fccc1a773e9e8ea14cc12`).
The candidate v1 fails all nine: seven call an unimplemented
`BrowserContext.storage_state()`, while the two restore cases reject
`Browser.new_context(storage_state=...)` as unsupported. The independent
oracle v1 passes seven and fails the two IndexedDB serialization assertions
solely because the unchanged upstream tests expect a hardcoded `localhost`
origin while the Tailscale-bound fixture correctly reports its full MagicDNS
origin. These two oracle failures are fixture-address discrepancies, not
evidence of a candidate-only IndexedDB implementation defect. Both result
provenance sidecars verify.

Candidate report
`.work/official-conformance/playwright-python-candidate-browsercontext-storage-state-v1.xml`
has SHA-256 `9a5bc5f317e00edb179f4f82ff337edcd709564a222319a5fbb95fb1aaf0546a`;
its provenance sidecar has SHA-256
`155d5002b95e94a021563afcefb40b4d050727c44f78dfdee30a9f6d9db1c1f0`.
Oracle report
`.work/official-conformance/playwright-python-oracle-browsercontext-storage-state-v1.xml`
has SHA-256 `2d214557d9f16484aaeefd15d95408a323bc46bd5eb19f68ad85a53f1196a135`;
its provenance sidecar has SHA-256
`1e1078a54df0c29c2dcfef26a247b603cbb8c70a75a63d92bb127c1135dc68cc`.
Exact nine-key merges into the complete 6,526-row inventories yield candidate
1,201 passed / eight skipped / nine failed / 5,308 not run in
`.work/official-conformance/outcomes/playwright-python-candidate-browsercontext-storage-state-v1.jsonl`
(SHA-256 `dc2bd099dd1ef158539f5e149b5e014a90d980b986a5c070751bd8fc6a3985a9`)
and oracle 745 passed / seven skipped / two failed / 5,772 not run in
`.work/official-conformance/outcomes/playwright-python-oracle-browsercontext-storage-state-v1.jsonl`
(SHA-256 `d27e9c0bfc0c8e889805119f2171d10bf8953f4cb5c6311f60c76672320e7218`).
This is the preserved red baseline, not the current storage-state result.

Python BrowserContext storage-state (opt-in implementation and paired rerun):
`context-storage-state` now owns bounded cookies, localStorage and simple-JSON
IndexedDB capture/restore in Rust. Python's async/sync facades only validate,
marshal and write the returned state to an optional file. Temporary origin
Pages are hidden from public Page/network notices and have cancellation
cleanup. The repaired macOS `abi3-py39` wheel includes its OpenSSL dylibs;
SHA-256 is `8f5e3234cf2d46cc9ff4c2cd9179031e59d75b53bd276c23744317d713493cb1`.
The second repaired build has the same payload after normalizing only
CycloneDX serial number/timestamp and its dependent RECORD hash. The installed
wheel loads outside the source tree, exposes the opt-in feature, and passes
all 11 focused facade tests. Python's 263 package unit tests,
strict Pyright, Ruff check/format, and 75 official harness/provenance tests
pass. The selected Rust host release union passes 839 tests with 22 explicit
ignores; strict host/Node addon Clippy and `cargo fmt --check` pass.

The same unchanged nine upstream cases on the same Chrome now produce seven
passes and the same two hardcoded-`localhost` IndexedDB assertion failures in
both candidate and independent oracle. Candidate rerun report
`.work/official-conformance/playwright-python-candidate-browsercontext-storage-state-v3-repeat.xml`
has SHA-256 `a43d954ff5a14ea64aa3a86de989d350f3aefe4059b7a5ca5eef82d366d2b5ad`;
its verified provenance sidecar has SHA-256
`1565dbc19591b3b757b96b8c0c2bb947d5bab28ab0cb34786e8a094f157178e6`.
Oracle v3 report
`.work/official-conformance/playwright-python-oracle-browsercontext-storage-state-v3.xml`
has SHA-256 `ed29bcc8e528ef77aa621891aa60bb346fd90b7c9d5ab214f9bb1311ae5252b6`;
its verified sidecar has SHA-256
`efd85286116c94b981f7185c87e5df36d6fe0df0f375f14582f06d92e58f18ab`.
Exact nine-key merges preserve 6,526 inventory rows: candidate 1,208 passed /
eight skipped / two failed / 5,308 not run, ledger SHA-256
`d1ecfa2db3e440bae951c09c55e2da862f45f964579b7e69970d59ded510f5ed`;
oracle 745 passed / seven skipped / two failed / 5,772 not run, ledger
SHA-256 `3161add7ea5243cefa071dfc2c9845ddf6e18a5dee43bc9dc7c450d7735f6a20`.

Preserve the first repaired-wheel run as a separate anomalous artifact:
`.work/official-conformance/playwright-python-candidate-browsercontext-storage-state-v3.xml`
timed out on the first async case after a just-closed task Chrome tab and then
produced navigation/setup/teardown failures. Its verified report SHA-256 is
`1e863732092719b145c1cfce082fa8c019d67d4b2061f388849f7107f300bd1b`;
strict ledger merging correctly rejects duplicate teardown test IDs. A
one-case control and full rerun of the *same* repaired wheel passed as above,
as did the earlier unrepaired v2 wheel. The transient cause is unproven; do
not classify the first run as a deterministic packaging defect or erase it.
Further parity remains open for encoded IndexedDB keys/values, browser-side
bounds on very large stores and lifecycle stress. Known-ID pending Contexts
are now hidden exactly; before the ID returns, the unidentified-creation
guard can still broadly hide unrelated newly observed Contexts. Concurrent
visibility parity for that interval and the full Python/Core corpora remain
open.

Python BrowserContext add-cookies (whole-file paired re-execution): The
existing pinned manifest selects all 19 Chromium cases in unchanged upstream
`tests/async/test_browsercontext_add_cookies.py`; manifest SHA-256 is
`7c2cdf04985d4c7b869dc5ad79557a8383bcd499f2946388d4fa83f9a4109ca0`.
The same repaired candidate wheel and independent oracle each pass 17 and
fail the same two assertions that hardcode `localhost` against the required
MagicDNS fixture. Candidate report SHA-256 is
`8c2d0cb78770560ef5b67c41b33a06ea1e2f4b7ddec6e6311e27f938309e86cc`
and verified sidecar SHA-256 is
`cc56001104c23d193f8bdcc27507919a740225e261f667b8e4a705fc7c10cda9`;
oracle report SHA-256 is
`5b859b0384ef1402b07d1abb943a8ff548b48f7857ae5115908a73f2da551fd4`
and verified sidecar SHA-256 is
`896521026e5fec97c04f51b599446a80b5442b02a6a0b00d7c42a231a712883b`.
Strict 6,526-row merges now record candidate 1,208 passed / eight skipped /
four failed / 5,306 not run (ledger SHA-256
`1876c267051f872f5cfb424c6865204861aa76c6a8b98ece4a227f108719b318`)
and oracle 762 passed / seven skipped / four failed / 5,753 not run (ledger
SHA-256 `c1d661e878a21d33d3fd1956d90dfc544749fea4c6787c01d8c7fe9b45e141ba`).
Both shared fixture-address failures remain recorded as failures, not
candidate successes.

NODE-433 (Core Page network-event design): Prove the complete unchanged seven-
case Chromium Page `page-event-network.spec.ts` file as one lifecycle tranche,
without treating individually working Request and Response events as proof that
their identities, terminal state or redirect graph compose correctly.

- [x] Inspect all seven exact `chromium-page` inventory rows, the unchanged
  upstream file and the official client/server Request lifecycle. Six rows are
  ordinary public behavior. The seventh is dynamically `fixme` for Chromium
  and must remain an explicit collected outcome rather than be removed with a
  title filter.
- [x] Define the observable contract before implementation: Request URL,
  method, resource type, Frame and eventual Response; Response URL, status,
  success and Request; failed-request null Response, typed failure text and
  Frame; request-finished success state; request/response/finished ordering;
  and both directions of a redirect chain with stable Request identity.
- [x] Add a fixed one-suite manifest selecting the entire file with no grep or
  title filter. Collect exactly seven tests from one source file and keep BiDi,
  Firefox and WebKit expansions separate and unclaimed. Oracle collect-only
  proves 7 tests in 1 file; manifest SHA-256 is `7b71c2a...2cf2`. Candidate
  collect-only is intentionally rejected because it cannot prove the runtime
  candidate was loaded, so it is not recorded as a candidate outcome.
- [x] Preserve complete candidate and independent Playwright 1.58.0 oracle
  baselines before changing runtime code. Inspect every attempt, including the
  dynamic Chromium fixme, instead of inferring missing methods from totals.
  Candidate v1 has one expected pass, five unexpected failures and one skipped
  case; oracle v1 has six expected passes and the same one skipped case.
- [x] Design one bounded Rust-owned Request lifecycle identity that can back
  `request`, `response`, `requestfinished` and `requestfailed` publication.
  Language wrappers may preserve object identity, but must not become the
  authoritative response promise, failure state or redirect graph. Define
  retirement after terminal publication without invalidating retained public
  wrappers.
- [x] Keep optional concerns independently selectable. Base Request events may
  expose immutable method/resource/Frame metadata; eventual Response,
  failure, redirects and terminal state require only their declared lifecycle
  owner. Do not pull Page routes, response bodies, request sizes or unrelated
  request history into the minimal profile.
- [x] Design ordered composition instead of trusting two asynchronous language
  watchers. Request and Response notices currently occupy separate bounded
  queues, so their independent Promise loops cannot prove browser event order.
  Add one optional merged-dequeue port over those existing queues, using a
  shared monotonic Rust sequence and the lower front sequence. It adds no third
  event copy or queue; standalone request-only and response-only profiles keep
  their present ports, while the composed profile starts only the merged
  watcher.
- [ ] Add AAA tests for same-object publication, terminal success/failure,
  Response cross-links, redirect predecessor/successor identity, event order,
  Frame lookup, bounded owners, overflow and close cleanup. Focused host
  network tests pass 28/28, including a three-hop retained predecessor chain,
  eviction/lease release and the 256-owner cap. Node cache-loss and retained-
  wrapper tests pass. Exhaustive coverage of every named case is still open.
- [x] Run all seven unchanged cases in paired candidate/oracle lanes on the
  same clean dedicated Chrome, verify provenance, merge exactly those seven
  inventory keys and regenerate both 48,477-row audits. Candidate v3 and
  oracle v3 each record six expected passes and the same upstream Chromium
  `fixme` skip, with no unexpected outcome.
- [ ] Finish the selected release host union, package, type, native-load,
  standalone profile, dependency-boundary, Clippy, formatting, diff and memory
  gates on the v3 source. Node 414/414, types, native-load, boundary, affected
  host 28/28, selected host release union 839 passed/22 ignored, four
  standalone profile build/load checks, strict selected Clippy and both
  official lanes pass; final memory/diff checks remain.
- [ ] Continue the complete Python and Core corpora. The pinned nine-case
  Python BrowserContext storage-state baseline and repaired paired rerun are
  recorded above; the full corpora remain open.

The lifecycle design extends the existing bounded `RequestHistoryOwner` rather
than adding a second Request registry. Its response state and terminal state
must be independent: a Response can arrive before a later successful or failed
terminal event. The owner therefore exposes an awaited optional Response plus
a synchronous terminal/failure snapshot. Redirect metadata remains Rust-
derived in both directions; the language cache only canonicalizes wrappers by
the portable Request ID and links the already declared predecessor/successor.
After linking, the public wrappers retain each other, so the Page cache can
retire terminal entries without breaking user-retained identity. To survive
GC before the next hop is linked, each bounded Rust owner now retains a one-way
Arc to its predecessor. The synchronous one-hop FFI reconstructs a collected
JS predecessor on demand while reusing a still-live wrapper. N-API carries the
owner as an opaque `External<Arc<_>>`, not as a new behavior-owning class.

Feature boundaries remain explicit. `request-events` stays the immutable
notice leaf. The implementation reuses the existing opt-in `request-history`
owner selected by `response-request` in the official profile for eventual
Response, terminal/failure and redirect metadata; it has not released a new,
independently named Request-lifecycle profile. Ordered network-event
composition requires Request and Response event leaves but reuses their
storage. `response-finished` remains independent; the minimal request-events
profile does not select response bodies, Page routes or request sizes. The v1
baseline below was captured before runtime edits.

The immutable NODE-433 v1 baselines now cover all seven `chromium-page` cases
on the same dedicated Chrome 155.0.8059.39, with no title filter. Candidate
v1 records one expected pass (`Response @smoke`), five unexpected failures,
and the upstream Chromium `fixme` as one skipped case. Its failures are two
missing `Request.response()` calls, no observed failed-request event in
`RequestFailed @smoke`, and two `Response.finished()` unsupported-operation
errors; these are symptoms, not yet a verified common cause. The independent
Playwright 1.58.0 oracle records six expected passes and the same skip. The
candidate result is
`.work/official-conformance/playwright-core-candidate-page-event-network-core433-v1.json`
(SHA-256 `eb56bbbae48d90f68349df37b8c6c2b5b23e8fa8b4fa3cf741b0d396b803d32b`),
with provenance sidecar
`.work/official-conformance/playwright-core-candidate-page-event-network-core433-v1.provenance.json`
(SHA-256 `27e8fdd4977675997c7dad7c7a18617d69965b70b9f11efbddf24f985a3619ec`).
The oracle result is
`.work/official-conformance/playwright-core-oracle-page-event-network-core433-v1.json`
(SHA-256 `8aa08739fe71c9c4df1963ecd0813a9c2056c83bc813eff101a813f7fce87977`),
with provenance sidecar
`.work/official-conformance/playwright-core-oracle-page-event-network-core433-v1.provenance.json`
(SHA-256 `2e5fb0b9f97abedf644dd9ebe3c822e62a18994394e49ff738496b95cbc0dcc2`).
Both provenance sidecars verify. This v1 result remains the preserved red
baseline, not the final candidate result.

The repaired candidate v2 and independent oracle v2 each pass all six
non-skipped cases in the unchanged seven-case file, with the same upstream
Chromium `fixme` skip and zero unexpected outcomes. Candidate report
`.work/official-conformance/playwright-core-candidate-page-event-network-core433-v2.json`
has SHA-256 `6c8190143688489e013120b4c17bbc0c324f17ed712ece068a3d868f5653eab4`;
its verified provenance sidecar
`.work/official-conformance/playwright-core-candidate-page-event-network-core433-v2.provenance.json`
has SHA-256 `600e676fc57869b7b3df9313aa0209984762833460122dfa54a6a9d150dcee7a`.
Oracle report
`.work/official-conformance/playwright-core-oracle-page-event-network-core433-v2.json`
has SHA-256 `5353a6e754bdb2de30ca058300aec340899868adedc0c6dfcd1559aaede8afe9`;
its verified provenance sidecar
`.work/official-conformance/playwright-core-oracle-page-event-network-core433-v2.provenance.json`
has SHA-256 `4f8eddf2c5473c139ae7b5c7ac32a2c1b763c52f94218240c40d9c98837aa359`.
Exact seven-key merges yield candidate 1,548 expected / two skipped / four
unexpected / 46,923 not run, ledger SHA-256
`183e410d1119033a6524048b45d3dde56d5ed88a54eb345e4cea977f607af7a0`;
and oracle 1,251 expected / one skipped / three unexpected / 47,222 not run,
ledger SHA-256
`f093e0e0c3d2d408fc881c6b3e13d8e53ab478c1237bde9b306102a5b82b811a`.
Both 48,477-row classification audits retain SHA-256
`32520af75e4ac14352a9060e6f7156900bb52c4a3d99fe927311ba7490328af0`.
The exact selected host union passes 831 release `--lib` tests with 22
explicit ignores; affected narrow host network tests pass 25/25. Node package
tests pass 412/412, strict TypeScript types (including Request lifecycle
types) and official native load pass. Standalone `request-events`,
`response-events`, `response-finished` and `response-request` profiles each
release-build and native-load; dependency-boundary checks, strict addon
Clippy, `cargo fmt --check` and `git diff --check` pass. The candidate v2
provenance was reverified after formatting/profile builds and binds artifact
SHA-256 `03c2ce054d0c36959374e4e2dca7a0c6d3c2f6394052364d8c0edde9a88ab137`
and source SHA-256 `5ed5f37dc878f97267238cfd755145f9fb30dffb18c639364698051883f533da`.
At the memory checkpoint, 67% was free and swap use was zero; shared app RSS
cannot be attributed to this chat. Exhaustive AAA coverage, including GC
under a live browser between redirect hops, and the complete Core and Python
corpora remain open.
The pinned nine-case Python storage-state red baseline and repaired paired
rerun are recorded above; an earlier attempt safely refused an unrelated game
tab, but it is no longer the status of that tranche.

The post-GC candidate v3 and independent oracle v3 rerun the same unchanged
seven cases on the same dedicated Chrome: six pass and the same upstream
Chromium `fixme` is skipped in each lane. Candidate report
`.work/official-conformance/playwright-core-candidate-page-event-network-core433-v3.json`
has SHA-256 `ab638a95df1368c01f9f04818fad394cb0848b75802df08ff38846e0ecf255a6`;
its verified provenance sidecar has SHA-256
`3030484167f94cc300459b60f51e9043afb1dbd29f1568c9d448f6716644fa09`.
Oracle report
`.work/official-conformance/playwright-core-oracle-page-event-network-core433-v3.json`
has SHA-256 `611f034080a758aed4e681d9713777286590929da7ad04c527f4f254701d4863`;
its verified provenance sidecar has SHA-256
`2b7222f836cd285cc41e9cb3c41635c8dfc6ef4b24246291d0cfa3ea8cd003a6`.
The candidate artifact SHA-256 is
`f229cbbc36691afaee8b8ba65d6dfffad2b3a8f2576647a1e186d5e558215384`, and
its Node package source SHA-256 is
`47b354a5e3b4ef3390f4a99629978fd7275e799c0ee4e43ff8f53a4ae152a8c3`.
Exact seven-key complete-ledger merges retain the above outcome counts:
candidate v3 SHA-256
`9d6b99f4eb4049ab1d67dbb62f86c128f212d96b857ee55a7fadd5298f32f6e6`,
oracle v3 SHA-256
`85cbbf632ad1fb9110cf5cac29dfafd7597d30b12301b660151554d52c0c79d3`.
Both regenerated 48,477-row classification audits retain SHA-256
`32520af75e4ac14352a9060e6f7156900bb52c4a3d99fe927311ba7490328af0`.
The new focused Rust network tests pass 28/28; the exact selected host release
union passes 834 tests with 22 explicit ignores. Node package tests 414/414,
strict TypeScript types, native-load and dependency-boundary checks pass.
These are v3 gates; the v2 standalone profiles and Clippy results above remain
historical until repeated after this change.

NODE-432 (Core Page response-wait design): Complete the unchanged Chromium
Page `page-wait-for-response.spec.ts` file by composing public Response events
over the existing bounded Rust network lifecycle, without deriving responses
from request events or adding another CDP observer.

- [x] Inspect the complete pinned file, its nine exact `chromium-page`
  inventory rows and the official Page/Waiter/Response client paths. All nine
  are ordinarily passing `public-api-candidate` rows; Firefox/WebKit/BiDi and
  Android expansions remain separate and unclaimed.
- [x] Define the required public behavior before implementation: synchronous
  string/RegExp/predicate waiting; generic `waitForEvent('response')`; explicit
  and Page-default timeouts; caller stacks and matcher logs; async predicates
  that await `response.text()`; one call to a winning sync predicate;
  `timeout: 0`; and cached-image response publication alongside the already
  supported request event.
- [x] Add a fixed one-suite manifest with no title filter, collect exactly nine
  tests from one source file, and preserve complete candidate/oracle baselines
  before changing runtime code. Inspect every attempt rather than inferring a
  shared cause from aggregate counts. Collection proves nine tests in one file.
  Candidate v1 passes 1/9: the explicit generic-event timeout passes, four
  `waitForResponse` calls fail immediately as missing, and three generic
  response waits plus the mixed cached-image wait expose absent Response event
  publication. Oracle v1 and a clean-Chrome v2 both pass 8/9; Chrome 154 never
  emits the second cached-image request, so that shared upstream/current-browser
  discrepancy remains explicit rather than being rewritten as candidate red.
- [x] Reuse the existing Rust `response-events` installation and bounded
  response-notice queue. Add a narrow independently selectable N-API watch
  port and immutable Response snapshot conversion; compose the existing
  `response-body`, `response-request` and request-event leaves only when their
  public methods are selected. JavaScript must not subscribe to CDP, infer a
  response from request completion or retain a second response registry.
- [x] Share the NODE-431 Page event waiter for timeout, close, call-stack and
  diagnostic behavior. Add only Response publication and URL/predicate
  matching in the language facade, while Rust retains subscription activation,
  queue admission, lifecycle reconciliation, body ownership and URL-glob
  compilation. Cover the minimal response-events profile and full official
  composition with AAA, native-load and dependency-boundary tests.

The implementation adds only a bounded `nextResponseNotice` N-API watch and
immutable metadata conversion over Rust's existing 256-item response queue.
The standalone `response-events` artifact release-builds and native-loads;
dependency proofs exclude Page routes, response body and request history from
that minimal profile. Candidate v2 passed 7/9 and isolated one candidate-only
failure: an async predicate called `response.text()` before CDP terminal body
readiness and received `No resource with given identifier found`. A separately
selected bounded `response-finished` owner now gates both the public completion
Promise and the optional body leaf without widening the minimal event profile.
- [x] Run the complete nine-case file through paired candidate/oracle lanes on
  the same clean dedicated Chrome, verify provenance, merge exactly the nine
  inventory keys into both 48,477-row ledgers, regenerate audits, and rerun
  package, type, ABI, dependency, Rust, Clippy, formatting, diff and memory
  gates. Keep the complete Python and Core corpora open.

Final candidate v3 and paired oracle v3 have zero normalized outcome diff:
eight expected outcomes and the same one unexpected cached-image case. Chrome
154 never emits the second request awaited by that upstream case, so it remains
an explicit shared browser divergence rather than a candidate success. Report
hashes are `1184ec4...229d` / `3ae728c...ba59`; provenance hashes are
`98c3ef4...9dd2` / `562462e...fd7`; manifest `e68c644...596`, artifact
`30bb029...853` and source `07c4c8e...ae` are provenance-bound. Exact merges
reach candidate 1,542 expected / one skipped / four unexpected / 46,930 not
run (`7bfaf60...de7a`) and oracle 1,245 expected / three unexpected / 47,229
not run (`ae0fc98...0322`). Both audits retain `32520af...8af0`. Package
407/407, exact official host 850 passed plus 29 explicit ignores, strict types,
native loads, boundary proofs, standalone release checks, strict addon Clippy,
formatting and diff checks pass. The complete Core and Python corpora remain
open.

NODE-431 (Core Page request-wait design): Prove the complete unchanged
Chromium Page `page-wait-for-request.spec.ts` file against the request-event
boundary completed by NODE-430 before expanding network coverage.

- [x] Inspect the complete pinned file, its eight exact `chromium-page`
  inventory rows and the official Page/Waiter client implementation. Every row
  is an ordinarily passing `public-api-candidate`; no annotation or title
  filter may reduce the tranche.
- [x] Define the required behaviors before implementation: synchronous
  `Promise.all` arming; string, predicate and ordinary/cross-VM RegExp matching;
  explicit and Page-default timeouts; `timeout: 0`; Playwright `TimeoutError`
  identity; quoted event diagnostics and long-URL log truncation; and a stack
  whose first user frame identifies the API call.
- [x] Add a fixed one-suite manifest with no title filter, collect exactly eight
  tests from one source file, and preserve complete candidate/oracle baselines
  before repairing a discrepancy. Collection proves eight tests in one file;
  oracle v1 passes 8/8, while candidate v1 passes 4/8. Its complete attempt
  records expose explicit-timeout wording, ignored Page-default timeout, absent
  long-URL log context, and a separate `window.builtins` fixture loss after the
  worker timeout. No aggregate failure is being counted as an API diagnosis.
- [x] Keep browser observation, bounded request queues, URL-glob compilation
  and default-timeout state in the existing selected Rust leaves. JavaScript
  may compose EventEmitter waiting and format Playwright-compatible client
  diagnostics, but it must not add a second request observer or own browser
  matching behavior.
- [x] Repair only the proven client-composition differences. Share one Page
  event waiter path between `waitForEvent('request', ...)` and
  `waitForRequest(...)`, preserve synchronous listener installation and
  predicate errors, reject on Page close/crash where observable, and retire
  listeners/timers on every terminal path. Add AAA tests for each repaired
  contract, including cross-realm RegExp and captured call-site stacks.
- [x] Run the complete eight-case file through paired candidate/oracle lanes on
  the same clean dedicated Chrome, verify provenance, merge exactly the eight
  inventory keys into both 48,477-row ledgers, regenerate audits, and rerun
  affected package, type, ABI, dependency, Rust, Clippy, formatting, diff and
  memory gates. Keep the complete Python and Core corpora open.

Final candidate v3 and paired oracle v2 each pass all eight unchanged cases.
Candidate v2's 7/8 intermediate isolated an off-by-one matcher-log preview;
the final cap includes the ellipsis in 50 Unicode code points. Report hashes
are `e2d90b8...e5d1` / `b7a837a...5df0`; artifact `fc00727...54790`, source
`76aa0e9...d0b8c` and manifest `b5936ea...cdcd` are provenance-bound. Exact
eight-row merges reach candidate 1,534 expected / one skipped / three
unexpected / 46,939 not run (`c19a233...7abc`) and oracle 1,237 expected / two
unexpected / 47,238 not run (`4030917...5241`). Both audits remain
`32520af...8af0`. Package 393/393, exact official host 843/843 executed passes
plus 29 explicit browser/diagnostic ignores, types, native loads, boundaries,
minimal feature checks, strict Clippy, formatting and diff checks pass. The
complete Python and Core corpora remain open.

NODE-430 (Core Request post-data design): Complete the unchanged Chromium Page
`network-post-data.spec.ts` file without treating its two upstream
expected-failure annotations as automatic candidate success.

- [x] Inspect the complete pinned file, its six exact Chromium Page inventory
  rows and the official client/server Request implementations. Four cases are
  ordinarily expected; Chromium marks the file/blob and `sendBeacon` cases as
  expected failures.
- [x] Reuse the existing bounded Rust `request-events` notice queue rather than
  add a second request registry. The host activates subscriptions before an
  attached/new Context becomes public; Node runs one weak Browser watcher and
  `page.waitForRequest()` installs its EventEmitter listener synchronously
  before returning the Promise. This preserves Playwright's `Promise.all`
  arming order without racing an asynchronous native arm call.
- [x] Keep JSON/form decoding in the existing optional Rust
  `request-post-data-json` leaf. N-API may materialize the already parsed value,
  and JavaScript may wrap the snapshot, but JavaScript must not become the
  authoritative post-data parser.
- [x] Add a fixed one-suite manifest with no title filter and collect exactly
  six `chromium-page` invocations from one source file. Preserve a complete
  candidate/oracle baseline before repairing any discrepancy. Oracle v2 is
  6/6 expected after restarting the dedicated Chrome; candidate v1 records two
  annotated expected outcomes plus four unexpected outcomes, but all six
  attempts fail at the same missing `page.waitForRequest` call.
- [x] Add independently selected Node `request-events` and
  `request-post-data-json` boundaries. The former exposes one bounded
  `nextRequestNotice` watch port and the shared Rust URL-glob compiler; the
  latter extends Request snapshots only with Rust's parsed value or official
  error. Neither may select Page routes, response bodies or request history.
- [x] Implement Playwright-shaped `page.waitForRequest()` matcher and timeout
  handling over that notice stream, reusing the shared Request facade and URL
  glob compiler. Cover synchronous listener installation, skipped nonmatches,
  predicate errors, timeout, terminal cleanup and FFI-copy accounting with AAA
  tests. Build and native-load both standalone profiles: this caught and fixed
  two N-API object-shape feature-combination bugs that the full official
  profile had masked, and CI now compiles both minimal unions explicitly.
- [x] Run the complete unchanged six-case file in paired candidate/oracle lanes
  on the same dedicated Chrome. Inspect attempt errors for both annotated
  cases, verify provenance, merge only honest outcomes, regenerate both
  48,477-row audits and rerun package, type, ABI, dependency, Rust, Clippy,
  formatting, diff and memory gates. Keep the complete Python and Core corpora
  open.

Final candidate v3 and oracle v3 each record six expected outcomes. The four
ordinary cases pass, while the file/blob and `sendBeacon` cases fail at their
real post-data assertions with the same `null` values in both lanes, satisfying
the pinned Chromium annotations honestly. Candidate v2's 5/6 intermediate
isolated an omitted N-API optional field; an explicit parser-selected boolean
now distinguishes parser absence, parsed null and parse error. Verified report
hashes are `3349ede...67955` / `7452f56...8e2c`; artifact
`fc00727...54790`, source `afb44a6...6757` and manifest
`b652990...3794` are recorded by provenance. Exact six-row merges reach
candidate 1,526 expected / one skipped / three unexpected / 46,947 not run
(`8e8e6b7...be91`) and oracle 1,229 expected / two unexpected / 47,246 not
run (`55be77a...57aa`). Both audits remain `32520af...8af0`. The complete
Python and Core corpora remain open.

NODE-429 (Core extra HTTP headers design): Prove the existing composable header
leaf against both complete Chromium public-API files before changing runtime
code.

- [x] Inspect the complete unchanged upstream files and exact inventory rows.
  Chromium contributes two `browsercontext-set-extra-http-headers.spec.ts`
  invocations and five `page-set-extra-http-headers.spec.ts` invocations; the
  Firefox/WebKit rows remain separate and unclaimed.
- [x] Keep header behavior in the existing optional `extra-headers` Rust leaf.
  Context creation, Context mutation and Page mutation already share its
  bounded state/command boundary; Node may validate public record shapes and
  map typed errors but must not merge, lowercase or dispatch headers itself.
- [x] Add a fixed two-suite manifest with no title filter: `chromium-library`
  for the two Context-file cases and `chromium-page` for all five Page-file
  cases. Collect-only must prove 2 + 5 exact invocations and one file per suite.
- [x] Preserve paired candidate/oracle baselines before any repair. Required
  behaviors are case-insensitive Page-over-Context precedence, retaining
  unrelated Context headers, redirects, Context mutation, exact non-string
  diagnostics and Chromium's explicitly expected-failure Referer case.
  Context is candidate/oracle 2/2. Page is candidate 5/5 versus current-Chrome
  oracle 4/5: the oracle's Referer assertion now passes and is therefore an
  unexpected pass against upstream's pinned `test.fail` annotation.
- [x] Repair the masked candidate failure before interpreting that divergence.
  Candidate currently satisfies the expected-failure marker for the wrong
  reason: `response.request()` is absent. Add an independently selected
  `response-request` boundary that resolves the navigation's native request ID
  through the existing bounded Rust request-history owner and copies one
  immutable Request snapshot through N-API. Reuse the shared Request facade;
  do not reconstruct sent headers from configured Page/Context settings.
- [x] If another discrepancy remains, repair the narrow Rust/facade boundary;
  do not weaken assertions, filter the Referer case, or move header merging or
  redirect behavior into the harness. The repaired candidate reaches the real
  Referer assertion and passes it, exactly like the current-Chrome oracle; no
  second runtime discrepancy remains.
- [x] Verify provenance and merge complete-file outcomes. Call only identical
  green rows paired conformance; retain the Chrome-154 expected-failure drift
  as an explicit oracle unexpected result if it remains after the candidate
  reaches the real assertion. Regenerate both 48,477-row audits and rerun
  affected package, ABI, dependency, release, Clippy, formatting, diff and
  memory gates. Keep the complete Python and Core corpora open.

Final candidate and oracle Context files each record two expected outcomes.
Their Page files each record four expected outcomes plus the same one
unexpected pass: Chrome 154 satisfies the Referer assertion although pinned
Playwright 1.58.0 still marks that case with `test.fail`. All four provenance
sidecars verify. The candidate merge refreshes exactly seven prior keys and
corrects the formerly masked Referer row from expected-with-`TypeError` to
unexpected-without-error; no other candidate row changes. The oracle merge
refreshes the two prior Context keys and executes five Page keys. Final ledgers
are candidate 1,520 expected / one skipped / three unexpected / 46,953 not run
(`366378d...5da9`) and oracle 1,223 expected / two unexpected / 47,252 not run
(`7642925...2ac`). Both 48,477-row audits remain `32520af...8af0`. The complete
Python and Core corpora remain open.

NODE-428 (Core Touchscreen/tap design): Split coordinate touch input from
selector-driven tap actions before implementing the official `tap.spec.ts`
tranche.

- [x] Inspect the complete unchanged upstream file and the pinned Playwright
  server path. The Chromium Library tranche has nine invocations spanning raw
  `Touchscreen.tap`, Page/Locator/ElementHandle actions, event ordering,
  cancelled touch events, temporary modifiers, trial behavior, navigation and
  visibility waiting.
- [x] Keep two independently selectable boundaries. `touchscreen-input` owns a
  Page-scoped coordinate primitive and real `touchStart` / `touchEnd` dispatch;
  `tap-action` composes selector/Handle resolution, actionability, hit-target
  interception, trial semantics and temporary modifiers over that primitive.
- [x] Add a typed Rust Touchscreen capability/backend/composition leaf with
  finite coordinates, bounded concurrent ownership, Page input-state sharing,
  cancellation-safe native retirement and exact teardown metrics. It must not
  require Locator, Handle or Mouse capability crates.
- [x] Expose raw `page.touchscreen.tap(x, y)` through a feature-selected N-API
  port and a validation-only Node facade. Context `hasTouch` remains a separate
  prerequisite owned by the existing `device-emulation` leaf.
- [x] Add the Rust tap-action composition once the raw leaf is proven. Reuse
  the existing actionability and input coordinator rather than resolving
  selectors or simulating browser events in JavaScript. Cover Page, Locator and
  ElementHandle without duplicating browser behavior.
- [x] Prove Playwright-compatible event order, cancelled `touchstart` /
  `touchend`, current and requested modifiers, well-formed touch points,
  navigation non-waiting, delayed visibility and trial-only pointer boundary.
  The preserved v3-v5 candidate reports pass the other eight cases and isolate
  navigation non-waiting as the only remaining browser proof. Final-source v6
  closes that case and passes the full file 9/9.
- [x] Run the complete unchanged nine-case Chromium Library file in isolated
  oracle and candidate lanes only when the shared dedicated Chrome has no
  foreign active page. Then update the cumulative 48,477-row ledgers by exact
  inventory key and rerun all package, type, ABI, dependency, Rust, Clippy,
  formatting, provenance and memory gates.
- [x] Do not claim official Touchscreen or tap compatibility from the legacy
  gRPC helper, mouse-backed historical fixture, direct diagnostic, filtered
  title, or an implementation checkpoint that lacks the full-file paired gate.

Final candidate and independent oracle both pass 9/9 with no title filter.
Verified report/provenance SHA-256 values are `f165c1c...71086` /
`63a1407...ce35` and `e3eb0bd...e664e` / `e886e57...6e07b`; artifact
`5fb122e...b91` is selected explicitly. Strict nine-row merges reach candidate
1,521 expected / one skipped / two unexpected / 46,953 not run
(`d084416...8bac`) and oracle 1,219 expected / one unexpected / 47,257 not run
(`1589687...8b0`), with zero inventory drift. Both 48,477-row audits retain
`32520af...8af0`. Full-corpus execution remains open.

NODE-427 (Core Touch events): Materialize TouchEvent dictionaries and expose
Context touch emulation through an independent Node feature.

- [x] Pin and execute the complete unchanged Chromium Library
  `locator-dispatchevent-touch.spec.ts` file with no title filter: independent
  oracle 1/1 and final candidate 1/1.
- [x] Reproduce the two candidate discrepancies before the official run:
  Chromium rejects plain initializer records as `Touch` values, then the
  repaired event program reaches the explicit `hasTouch=true` capability
  boundary. Do not mislabel these direct diagnostics as an official red report.
- [x] Make the Rust-owned static dispatch program construct real browser
  `Touch` objects for `touches`, `targetTouches` and `changedTouches`, defaulting
  each missing target to the resolved Locator element.
- [x] Add independently selectable addon `device-emulation` identity and a
  narrow `newContextWithHasTouch` port over the existing bounded Rust Context
  registry; keep validation/selection in the Node facade and emulation,
  lifecycle and teardown in Rust.
- [x] Rebuild artifact `69a91ea...de1`; pass candidate/oracle 1/1 with reports
  `9b5a550...918b` / `2ed7cd4...be5e` and verified provenance
  `e3907f9...cb73` / `51acd91...bd3`.
- [x] Strictly promote the same single inventory row in each 48,477-row ledger.
  Candidate reaches 1,512 expected / one skipped / two unexpected / 46,962 not
  run (`7d0d844...1260`); oracle reaches 1,210 expected / one unexpected /
  47,266 not run (`fb3aef0...06ed`). Both audits remain
  `32520af...8af0`.
- [x] Pass package 368/368, strict TypeScript, native load, dependency
  boundaries, the exact official-profile host release suite (796 passed / 22
  explicit browser ignores in the library target; 818 / 30 across all selected
  targets), formatting, diff check and strict host/addon Clippy.
- [ ] Continue every remaining applicable Python and Core JS/TS invocation;
  complete-corpus conformance remains open.

[Architecture and evidence](node-ffi-compatibility-design.md#touch-event-materialization-and-context-emulation-node-427-2026-10-06).

NODE-426 (Core Locator): Complete the final official Locator query file.

- [x] Pin and execute all 24 unchanged Chromium Page invocations in
  `locator-query.spec.ts` with no title filter: independent oracle 24/24 and
  preserved candidate red baseline 12/24.
- [x] Add typed `RegExp` `hasText` / `hasNotText` filters through the existing
  Rust selector recipe, expose the existing Rust ordered-text expectation for
  the file's exact-string arrays, and retain an explicit unsupported boundary
  for mixed regular-expression arrays until their own official tranche.
- [x] Compare lazy FrameLocator recipes structurally and defer invalid
  cross-frame `and` / `or` diagnostics until the async query, matching the
  official construction and failure timing without moving frame ownership to
  Node.
- [x] Implement the private `__testHookBeforeQuery` count rendezvous as a
  bounded Rust host operation: capture the current execution world, await the
  Node hook through one thread-safe promise, then query the same world so
  navigation produces Playwright's context-loss result.
- [x] Preserve a 23/24 intermediate result that isolated the final discrepancy
  to composite-FrameLocator error timing; rebuild artifact `c12153d...d1bc`
  and pass final candidate/oracle 24/24 with reports `54e5e20...d913` /
  `60508d7...5f8c` and verified provenance `deecead...ad7` /
  `fb1b7c8...2413`.
- [x] Strictly promote exactly the same 24 not-run rows in each 48,477-row
  ledger with zero inventory changes. Candidate reaches 1,511 expected / one
  skipped / two unexpected / 46,963 not run (`083773b...7d1`); oracle reaches
  1,209 expected / one unexpected / 47,267 not run (`6961c30...eaf`).
- [x] Regenerate both invocation audits (`32520af...8af0`) and pass package
  363/363, strict TypeScript, native load, dependency boundaries, the exact
  official-profile host release suite (795 passed / 22 explicit browser
  ignores in the library target; 817 / 30 across all selected targets),
  formatting, diff check and exact feature-union Clippy.
- [ ] Continue every remaining applicable Python and Core JS/TS invocation;
  the complete 123-case Locator Chromium tranche is now accounted for.

[Architecture and evidence](node-ffi-compatibility-design.md#locator-query-composition-node-426-2026-10-06).

NODE-425: Complete the second official Locator miscellaneous file.

- [x] Pin and execute all 16 unchanged Chromium Page invocations in
  `locator-misc-2.spec.ts` with no title filter: independent oracle 16/16 and
  preserved candidate red baseline 12/16.
- [x] Expose the existing Rust `page-viewport` and `locator-screenshot`
  capabilities plus `selectText` on the already-selected
  `locator-dom-actions` leaf through thin Node facades; keep viewport policy,
  strict resolution, scrolling, geometry, capture and actionability in Rust.
- [x] Preserve a 15/16 intermediate run that proved those three browser paths
  and isolated the remaining discrepancy to the unknown-key diagnostic; route
  the typed keyboard error through one mapper in both ordinary and drag-aware
  host selections.
- [x] Rebuild artifact `3edcb5c...71db`; pass final candidate/oracle 16/16
  with report hashes `fe9ba30...482c` and `f7010a8...5173` and verified
  provenance `c53a522...bc90` / `969c169...06fd`.
- [x] Strictly promote exactly the same 16 not-run rows in each 48,477-row
  ledger with zero inventory changes. Candidate reaches 1,487 expected / one
  skipped / two unexpected / 46,987 not run (`8c78583...c841`); oracle reaches
  1,185 expected / one unexpected / 47,291 not run (`7d75835...4ef9`).
- [x] Regenerate both invocation audits (`32520af...8af0`) and pass package
  360/360, strict TypeScript, native load, dependency boundaries, the exact
  official-profile host release suite (793 passed / 22 explicit browser
  ignores in the library target; 815 / 29 across all selected targets),
  formatting, diff check and exact feature-union Clippy.
- [ ] Execute the remaining 24 Locator Chromium Page invocations, then every
  remaining applicable Python and Core JS/TS invocation.

[Architecture and evidence](node-ffi-compatibility-design.md#locator-miscellaneous-viewport-capture-and-selection-node-425-2026-10-06).

NODE-424: Complete the first official Locator miscellaneous file.

- [x] Pin and execute all 13 unchanged Chromium Page invocations in
  `locator-misc-1.spec.ts` with no title filter: corrected independent oracle
  13/13 and preserved candidate red baseline 11/13.
- [x] Expose the existing independently selectable Rust
  `locator-dom-actions` leaf through a thin Node `Locator.dispatchEvent()`
  facade; event source, strict resolution, initializer conversion and timeout
  remain Rust-owned.
- [x] Compose the pre-existing `file-input` leaf into only the explicit
  `official-conformance-page-bindings` profile; do not duplicate upload logic
  or widen default artifacts.
- [x] Rebuild artifact `356f84b...c39e`; pass final candidate/oracle 13/13
  with report hashes `566e6fd...6a85` and `28b9a9a...568b` and verified
  provenance `59fbc53...5ca0` / `26886bc...51ed`.
- [x] Strictly promote exactly the same 13 not-run rows in each 48,477-row
  ledger with zero inventory changes. Candidate reaches 1,471 expected / one
  skipped / two unexpected / 47,003 not run (`1afb6c7...ecaa`); oracle reaches
  1,169 expected / one unexpected / 47,307 not run (`1a73f91...ccb9`).
- [x] Regenerate both invocation audits (`32520af...8af0`) and pass package
  355/355, strict TypeScript, native load, dependency boundaries, the expanded
  official-profile host release suite (790 passed / 22 explicit browser
  ignores in the library target), formatting, diff check and exact
  feature-union Clippy.
- [ ] Execute the remaining 40 Locator Chromium Page invocations, then every
  remaining applicable Python and Core JS/TS invocation.

[Architecture and evidence](node-ffi-compatibility-design.md#locator-miscellaneous-actions-node-424-2026-10-06).

NODE-423: Complete the official Locator FrameLocator file.

- [x] Pin and execute all 22 unchanged Chromium Page invocations in
  `locator-frame.spec.ts` with no title filter: corrected independent oracle
  22/22 and preserved candidate red baseline 3/22.
- [x] Add thin composable `Locator.contentFrame()`,
  `Locator.frameLocator()`, nested/indexed/semantic `FrameLocator` methods and
  `FrameLocator.owner()` while retaining one lazy Rust-owned recipe.
- [x] Implement the independently required `Route.fulfill()` settlement leaf;
  the official frame fixture can now serve its routed iframe document without
  moving interception state or body serialization into Node.
- [x] Keep frame replacement, detach/navigation retry, original deadlines,
  missing-frame state reads, strict ambiguity and non-frame-owner diagnostics
  in Rust; add typed terminal CSS `:has-text()` composition.
- [x] Rebuild artifact `fbc48ff...1382b`; pass final candidate/oracle 22/22
  with report hashes `076f817...fb71a` and `0135d60...e6e55` and verified
  provenance `ded1730...0502` / `27230d7...54f4a`.
- [x] Strictly promote exactly the same 22 not-run rows in each 48,477-row
  ledger with zero inventory changes. Candidate reaches 1,458 expected / one
  skipped / two unexpected / 47,016 not run (`176cc90...a3b`); oracle reaches
  1,156 expected / one unexpected / 47,320 not run (`6106eae...546`).
- [x] Regenerate both invocation audits (`32520af...8af0`) and pass package
  352/352, strict TypeScript, native load, dependency boundaries, the exact
  official-profile host release suite (788 passed / 14 explicit browser
  ignores), formatting, diff check and exact feature-union Clippy.
- [ ] Execute the remaining 53 Locator Chromium Page invocations, then every
  remaining applicable Python and Core JS/TS invocation.

[Architecture and evidence](node-ffi-compatibility-design.md#locator-framelocator-composition-node-423-2026-10-06).

NODE-422: Complete the official Locator convenience file.

- [x] Pin and execute all 22 unchanged Chromium Page invocations with no title
  filter: oracle 22/22 and preserved candidate red baseline 13/22.
- [x] Add thin public `Locator.evaluateHandle`, `describe`, `description` and
  `toString` compatibility while retaining strict resolution, conversion,
  result-Handle ownership and cleanup in Rust.
- [x] Keep deferred Locator descriptions immutable across resolution, reset a
  custom description after every derivation and add Playwright-shaped timeout
  call-log presentation without moving retry policy into Node.
- [x] Bound conversion state before resolving an Element; publish only the
  final result Handle and release the unpublished temporary Element on every
  success, conversion failure, browser failure and cancellation path.
- [x] Preserve a 21/22 intermediate regression that caught missing rich text
  Node preview, then rebuild artifact `23f0bf4...390f05` and pass final
  candidate/oracle 22/22 with verified report hashes `b659794...146ae` and
  `86d5d28...458ed`.
- [x] Strictly promote exactly the same 22 not-run rows in each 48,477-row
  ledger with zero inventory changes. Candidate reaches 1,436 expected / one
  skipped / two unexpected / 47,038 not run (`6db265c...b8df`); oracle reaches
  1,134 expected / one unexpected / 47,342 not run (`61f077a...862d`).
- [x] Pass package 349/349, strict TypeScript, native load, dependency
  boundary, 807 non-ignored host release tests with 22 explicit live-browser
  ignores, formatting, diff check and full feature-union Clippy.
- [ ] Execute the remaining 75 Locator Chromium Page invocations, then every
  remaining applicable Python and Core JS/TS invocation.

[Architecture and evidence](node-ffi-compatibility-design.md#locator-convenience-composition-node-422-2026-10-06).

NODE-421: Reverify the already-supported Locator files end to end.

- [x] Pin five complete Chromium Page files in two fixed manifests: evaluate
  6, list 1, highlight 1, click 5 and is-visible 8.
- [x] Collect exactly 21 tests with no title filters, removed assertions or
  project overrides.
- [x] Pass independent oracle and Rust-backed candidate 21/21 against the same
  dedicated Chrome and verify all four provenance sidecars.
- [x] Keep the final NODE-420 artifact unchanged at `8e704cc...6f00a`.
- [x] Strictly refresh exactly 21 already-expected candidate rows, preserving
  1,414 expected / one skipped / two unexpected / 47,060 not run
  (`06e0444...63546`).
- [x] Strictly promote the same 21 oracle rows from not-run to expected,
  reaching 1,112 expected / one unexpected / 47,364 not run
  (`a813017...58324`).
- [x] Inventory all 11 Locator files as 123 Chromium Page tests and pin the
  remaining convenience/frame/misc/query files with verified collect counts.
- [x] Execute the complete 22-case Locator convenience file and repair only
  its observed discrepancies in NODE-422.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[Evidence](node-ffi-compatibility-design.md#supported-locator-file-reverification-node-421-2026-10-06).

NODE-420: Complete the official Locator ElementHandle file.

- [x] Pin all five Chromium Page invocations from the unchanged
  `locator-element-handle.spec.ts` file in one fixed manifest.
- [x] Preserve candidate 0/5 versus oracle 5/5 red evidence: the candidate
  lacked public `Locator.elementHandle(s)` and rejected typed-parent plus XPath
  descendant composition.
- [x] Add only thin public Node aliases; keep waiting, strictness, selector
  compilation, Handle publication and collection snapshots in Rust.
- [x] Represent typed and browser selector segments as one bounded immutable
  Rust program. Support only descendant (`within`) composition; keep
  intersection, union and filter composition explicit unsupported boundaries.
- [x] Poll browser-program `elementHandle()` under the original timeout,
  return null only internally while absent and preserve strict-mode rejection
  for multiple matches.
- [x] Rebuild final artifact `8e704cc...6f00a`; pass candidate/oracle 5/5
  with verified report hashes `7dfc27e...71a49` and `0f1655d...45ae0`.
- [x] Strictly promote only the same five `library/chromium-page` rows. The
  candidate ledger reaches 1,414 expected / one skipped / two unexpected /
  47,060 not run (`6fa68d3...9db39`); oracle reaches 1,091 expected / one
  unexpected / 47,385 not run (`6da3adc...83d6e`).
- [x] Pass host release 615 non-ignored units with 14 explicit live-browser
  ignores, package 346/346, strict TypeScript, native load, dependency
  boundary, formatting, diff check and full feature-union Clippy.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[Architecture and evidence](node-ffi-compatibility-design.md#locator-elementhandle-composition-node-420-2026-10-06).

NODE-419: Repair the non-preview ElementHandle convenience discrepancies.

- [x] Keep exact retained-Handle DOM reads in the optional Rust
  `element-handle-dom` capability; expose one bounded addon read port.
- [x] Reuse the shared versioned selector runtime for input retargeting and
  enabled/editable/checked state rather than duplicating DOM logic in Node.
- [x] Add thin `ElementHandle`, `Page` and `Frame` facades for attribute, HTML,
  input value, visibility/state, fill and select-option operations.
- [x] Rebuild artifact `6290125...978218` and preserve the complete unchanged
  12/15 candidate result (`98fa228...e87a9`): two rich previews and one
  `nth=0` chain remain red.
- [x] Model Playwright `nth=<i32>` in both typed and registered Rust selector
  chains, with positive, negative and invalid/initial coverage.
- [x] Implement rich Handle preview as an asynchronous, bounded Rust read over
  the exact retained Node; Node only caches the returned preview and isolates
  refresh failures.
- [x] Pass selector 38/38 and registry 11/11, package 346/346, strict
  TypeScript, release check and complete feature-union Clippy.
- [x] Rebuild final artifact `0247543...db47f` after the unrelated Page exits;
  pass the complete unchanged candidate file 15/15 and verify the v3
  provenance sidecar (`eee2d88...214b68`).
- [x] Strictly replace only the same 15 library invocations in the 48,477-row
  ledger: five stay expected and ten move unexpected to expected, reaching
  1,409 expected / one skipped / two unexpected / 47,065 not run
  (`b5532ff...c5940`).
- [x] Pass host release 609/609 non-ignored units (14 explicit live-browser
  ignores), native load and every dependency-boundary closure.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[Architecture and red baseline](node-ffi-compatibility-design.md#elementhandle-dom-reads-and-official-red-baseline-node-418419-2026-10-06).

NODE-418: Record complete ElementHandle query, evaluation and convenience files.

- [x] Pin fixed manifests for the complete 7-row query-selector, 6-row
  eval-on-selector and 15-row convenience Chromium Page files.
- [x] Pass candidate/oracle query-selector 7/7 and eval-on-selector 6/6 with
  current artifact provenance.
- [x] Preserve the complete convenience discrepancy at candidate 5/15 versus
  oracle 15/15; do not cherry-pick passing titles or weaken assertions.
- [x] Merge all 28 invocations into the 48,477-row candidate and oracle ledgers,
  keeping ten candidate failures explicit.
- [x] Complete NODE-419 browser verification and address rich preview as an
  independent retained-Handle lifecycle/caching capability.

NODE-417: Complete the official Page/Frame query-selector tranche.

- [x] Add Page/Frame `$$` as one Rust-owned ordered ElementHandle batch with
  temporary carrier settlement and no page-overridable `Array.from`.
- [x] Retain full XPath as a separately tagged, bounded Rust browser plan
  instead of mislabeling it as CSS.
- [x] Compile explicit `xpath=`, automatic `//`, `..`, parenthesized XPath and
  relative XPath in mixed `>>` chains through the versioned backend runtime.
- [x] Preserve the initial candidate 10/17 result and its seven XPath-only
  discrepancies; pass repaired candidate and oracle 17/17.
- [x] Verify both provenance sidecars and strictly promote only the 17
  Chromium library rows, reaching 1,396 expected / one skipped / two
  unexpected / 47,078 not run.
- [x] Pass 335/335 package units, strict TypeScript, native load, boundary,
  release check, full-profile Clippy, formatting and diff checks.
- [ ] Extend browser-program composition only as later official operations
  require it; unsupported Locator operations must stay explicit.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[Composable query-selector evidence](node-ffi-compatibility-design.md#composable-page-and-frame-query-selectors-node-417-2026-10-06).

NODE-416: Make Page/Frame `$` atomic and close the remaining registered-selector rows.

- [x] Replace the JavaScript `count()` plus Handle sequence with one Rust-owned
  optional ElementHandle program while keeping Locator Handle semantics strict.
- [x] Treat exact null as absence without Handle publication, adopt utility
  Nodes and compensate non-Node results and route reservations.
- [x] Preserve the initial official `.first()` incompatibility and remove that
  redundant facade filter after Rust assumes non-strict first-match ownership.
- [x] Pass the remaining five Chromium selector-registration rows 5/5 in both
  candidate and oracle lanes with all ten provenance sidecars verified.
- [x] Strictly promote only those five rows, reaching 1,379 expected / one
  skipped / two unexpected / 47,095 not run.
- [x] Execute the complete 17-row query-selector tranche through NODE-417.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[Atomic query evidence](node-ffi-compatibility-design.md#atomic-optional-page-query-node-416-2026-10-06).

NODE-414: Execute the first browser-backed registered-selector row.

- [x] Preserve the initial candidate failure at `tag=DIV` and the independent
  Playwright 1.58.0 oracle 1/1 pass.
- [x] Expose Rust `evaluate_first_with` through the Node addon and route public
  `$eval`/`$$eval` through Rust-owned registered selector execution.
- [x] Parse `page.$` through a Rust Locator count before the ordinary Handle
  producer, preserving zero-match `null` and case-sensitive engine errors.
- [x] Pass repaired candidate/oracle 1/1 with verified provenance and promote
  exactly one row to 1,374 expected / one skipped / two unexpected / 47,100
  not run.
- [x] Pass 319/319 package units, strict TypeScript, package boundary, release
  native load and full composed-profile Clippy.
- [ ] Add positive registered-engine Handle publication for `page.$` and run
  the remaining path, content-script, bad-return and composition rows.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[Browser-backed selector evidence](node-ffi-compatibility-design.md#browser-backed-selector-execution-node-414-2026-10-06).

NODE-413: Add composable Node selector registration and close its first Core
public-API discrepancy.

- [x] Preserve the initial candidate 0/9 versus oracle 9/9 browserless
  library/BiDi evidence for `selectors-register.spec.ts:148`.
- [x] Export CJS, ESM and typed `selectors.register()` plus
  `selectors.setTestIdAttribute()` with only source/path normalization and
  Node-specific error casing in JavaScript.
- [x] Add the independent `selector-registration` addon/profile marker and
  keep registry state in the lazily shared `NativeEnvironment`.
- [x] Route session, Locator, FrameLocator, semantic test-id and strict-action
  paths through the existing bounded Rust registry when selected.
- [x] Prove no added Pwright package dependency; pass default/selected release
  checks, selected clippy, package boundary, native load, 317/317 Node units,
  strict TypeScript and eight Rust registry units.
- [x] Pass repaired candidate 9/9 and oracle 9/9 with verified provenance;
  promote exactly nine rows to reach 1,373 expected / one skipped / two
  unexpected / 47,101 not run with no other ledger mutation.
- [ ] Execute the remaining browser-backed registered-engine behavior and
  error rows beyond the paired-green line-29 Chromium row.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[Node selector-registration evidence](node-ffi-compatibility-design.md#node-selector-registration-capability-node-413-2026-10-06).

NODE-411: Complete browserless launcher metadata across BiDi projects.

- [x] Add one exact manifest slice for the two pinned launcher metadata
  assertions under the official BiDi configuration.
- [x] Execute all six BiDi/Firefox library projects without starting or
  connecting a browser.
- [x] Pass candidate 12/12 and Playwright 1.58.0 oracle 12/12 with verified
  provenance.
- [x] Promote exactly 12 rows, reaching 1,364 expected / one skipped / two
  unexpected / 47,110 not run with no other ledger mutation.
- [x] Preserve the NODE-408 static registry implementation, package tests,
  Rust ABI, native artifact and resource boundaries unchanged.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[BiDi launcher metadata evidence](upstream-playwright-conformance.md).

NODE-410: Close sync BrowserContext page-event facade parity.

- [x] Run nine sync Chromium BrowserContext event rows and retain the initial
  candidate 7/9 versus oracle 9/9 discrepancy evidence.
- [x] Add async and sync `BrowserContext.expect_page()` facades over the
  existing bounded Context lifecycle waiter; preserve canonical Page identity
  in sync predicates and results.
- [x] Cover both facades with AAA tests; pass 252/252 installed-wheel units,
  Ruff and strict Pyright.
- [x] Build two portable wheels with normalized-identical payloads while
  preserving ABI104/90 and the native artifact SHA-256.
- [x] Pass repaired candidate 9/9 and oracle 9/9 with verified provenance and
  identical case IDs.
- [x] Promote exactly nine rows, reaching 1,201 passed / eight skipped / 5,317
  not run with no other ledger mutation.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[BrowserContext event evidence](python-official-conformance.md#sync-browsercontext-event-checkpoint-2026-10-06).

NODE-409: Promote only paired sync request-timing evidence.

- [x] Execute all four unchanged sync Chromium resource-timing rows from the
  installed ABI104/90 wheel; candidate passes 4/4.
- [x] Preserve the independent oracle 2/4 diagnostic where Chrome 154 returns
  a zero DNS timing for ordinary navigation and redirect.
- [x] Rerun only SSL and subresource as a clean common 2/2 candidate/oracle
  selection with matching case IDs and verified provenance.
- [x] Promote exactly those two rows, reaching 1,192 passed / eight skipped /
  5,326 not run with no other ledger mutation.
- [x] Keep the two candidate-green/oracle-red rows unpromoted and preserve
  ABI104/90, the artifact and all resource/dependency boundaries.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[Sync request-timing evidence](python-official-conformance.md#sync-request-timing-checkpoint-2026-10-06).

NODE-408: Close the complete Playwright launcher-metadata discrepancy.

- [x] Add a fixed browserless manifest slice for both upstream launcher
  metadata assertions across the three library projects.
- [x] Preserve the initial candidate 3/6 versus oracle 6/6 evidence showing
  that only `devices` was missing.
- [x] Vendor the complete pinned 143-entry registry with exact SHA-256,
  provenance and Apache-2.0 license; do not add a test-only single device.
- [x] Export one static registry through CJS, ESM and TypeScript without loading
  native code or starting a browser.
- [x] Pass repaired candidate 6/6 and oracle 6/6 with verified provenance.
- [x] Promote exactly six Core rows, reaching 1,352 expected / one skipped /
  two unexpected / 47,122 not run with no other ledger mutation.
- [x] Pass 312/312 Node tests and strict TypeScript without changing a Rust ABI,
  native feature, browser ownership or resource boundary.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[Launcher metadata evidence](upstream-playwright-conformance.md).

NODE-407: Complete sync service-worker policy under a valid secure context.

- [x] Run default-allow and explicit-block through the trusted Tailscale HTTPS
  fixture instead of reusing an invalid ordinary-HTTP diagnostic.
- [x] Pass the two unchanged sync Chromium rows 2/2 in candidate and
  Playwright 1.58.0 oracle with verified secure-fixture provenance.
- [x] Complete both async and sync service-worker policy Chromium files at
  2/2.
- [x] Promote exactly two rows, reaching 1,190 passed / eight skipped / 5,328
  not run with no other ledger mutation.
- [x] Preserve ABI104/90, the wheel and all capability/resource boundaries.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[Sync service-worker evidence](python-official-conformance.md#sync-service-worker-policy-checkpoint-2026-10-06).

NODE-406: Promote Keyboard-plus parity and preserve shared environment reds.

- [x] Pass the remaining environment-independent Chromium
  `Keyboard.press("+")` row 1/1 in candidate and Playwright 1.58.0 oracle.
- [x] Verify both provenance sidecars and the shared case-ID digest.
- [x] Promote exactly one row, reaching 1,188 passed / eight skipped / 5,330
  not run with no other ledger mutation.
- [x] Preserve the popup-dialog Page-timing and Tailscale-cookie-domain paired
  failures as shared-red diagnostics, not Pwright regressions or false passes.
- [x] Keep ABI104/90, the wheel and every capability/resource boundary
  unchanged.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[Keyboard-plus and shared environment evidence](python-official-conformance.md#keyboard-plus-and-shared-environment-checkpoint-2026-10-06).

NODE-403: Preserve sync caller provenance and promote only environment-valid
Page evidence.

- [x] Diagnose all nine remaining sync Page Chromium rows from the installed
  wheel; retain the 7-pass/2-fail report as diagnostic evidence.
- [x] Attach the calling Python thread to Playwright errors only on the sync
  failure path while retaining the native/async stack suffix.
- [x] Pass the exact same clean seven rows 7/7 in the wheel-backed candidate
  and Playwright 1.58.0 oracle with verified provenance.
- [x] Keep both headed `Page.pause`/headless-fixture mismatches explicitly
  `not-run` instead of fabricating Inspector support or promoting failures.
- [x] Promote exactly seven rows, reaching 1,187 passed / eight skipped / 5,331
  not run with no other ledger mutation.
- [x] Preserve ABI104/90 and the native artifact; pass 250/250 installed units,
  normalized two-wheel reproducibility and all dependency/strict gates.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[Sync Page evidence](python-official-conformance.md#sync-page-and-caller-stack-checkpoint-2026-10-06).

NODE-402: Verify sync request-fulfill parity without adding another owner.

- [x] Run the two remaining sync Chromium rows from the NODE-401 portable wheel.
- [x] Match them 2/2 against the independent Playwright 1.58.0 oracle with the
  same isolated case-ID digest and verified provenance.
- [x] Complete both async and sync request-fulfill Chromium files at 3/3.
- [x] Promote exactly two `not-run` rows, reaching 1,180 passed / eight skipped
  / 5,338 not run with no other ledger mutation.
- [x] Preserve ABI104/90 and all native dependency/resource boundaries.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[Sync request-fulfill evidence](python-official-conformance.md#sync-request-fulfill-parity-checkpoint-2026-10-06).

NODE-401: Execute and promote the complete official TodoMVC profile.

- [x] Preserve all 25 upstream `[chromium]` cases and run each in an isolated
  process from the repaired portable wheel.
- [x] Fix top-level JavaScript function classification in Rust so nested arrows
  remain valid expression source; cover the boundary with three AAA tests.
- [x] Provide the stable inactive `BrowserContext.tracing.stop()` teardown
  shape without pretending that trace recording or path output is installed.
- [x] Pass the exact same 25/25 cases in the wheel-backed candidate and
  Playwright 1.58.0 oracle with verified provenance.
- [x] Promote exactly 25 `not-run` rows and prove no other ledger mutation,
  reaching 1,178 passed / eight skipped / 5,340 not run.
- [x] Keep ABI104/90, pass 249/249 installed units, reproduce normalized wheel
  payloads and pass dependency, release, formatting, Ruff, strict Pyright and
  Python 3.9 gates.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[TodoMVC execution evidence](python-official-conformance.md#complete-todomvc-execution-checkpoint-2026-10-06).

NODE-400: Compose typed string expectations and promote only paired evidence.

- [x] Add one Rust-owned exact/regex matcher for value and class expectations;
  support mixed ordered class arrays through thin Python and Node adapters.
- [x] Preserve label/control value semantics, SVG-safe class ordering, bounded
  retries and pre-browser regex validation in Rust.
- [x] Pass all four unchanged async/sync Chromium class/value invocations in
  isolated candidate and Playwright 1.58.0 oracle lanes.
- [x] Promote only the four green candidate rows, advancing Python to 1,153
  passed / eight skipped / 5,365 not run across all 6,526 rows.
- [x] Produce a portable ABI104/90-feature wheel, prove normalized rebuild
  equality, pass 246/246 units and repeat the four official candidate cases
  from a fresh `site-packages` installation.
- [x] Pass 309/309 Node tests, dependency and capability-boundary proofs,
  strict types, Python 3.9 runtime audit, release checks and formatting gates.
- [ ] Execute and account for every remaining applicable Python and Core JS/TS
  invocation.

[Typed string matcher evidence](python-official-conformance.md#typed-string-expectation-checkpoint-2026-10-06).

NODE-398: Resolve definitions in the corrected official profiles.

- [x] Resolve all 6,526 fixture closures to 95 content-addressed definitions
  across both frozen environments with no unresolved provider.
- [x] Preserve definition origins, scopes, dependencies, override chains,
  selected definitions and reviewed semantic roles.
- [x] Classify all 25 TodoMVC rows as plugin-backed Page creation with a
  21-name closure rather than an infrastructure failure.
- [x] Join exact applicability and outcome ledgers and partition all 5,360
  applicable `not-run` rows without changing an outcome or ABI.
- [x] Prove byte-identical binding and definition ledgers on an independent
  two-profile recollection; pass 15 AAA tests and strict Python gates.
- [ ] Combine fixture bands with body-level launch analysis and execute every
  remaining applicable Python row.

[Profile-aware definition evidence](python-official-conformance.md#profile-aware-fixture-definition-checkpoint-2026-10-06).

NODE-397: Collect fixture closures in each official Python CI profile.

- [x] Collect the 6,501 core rows in the core oracle and the 25 TodoMVC rows
  in the isolated `pytest-playwright` examples oracle.
- [x] Verify canonical one-to-one order and exact direct-input agreement
  without executing any fixture or test body.
- [x] Preserve core-only autouse server/asyncio fixtures and all plugin-only
  Page, Context, launch, URL and artifact fixtures.
- [x] Pass seven AAA tests plus Ruff, Python 3.9 runtime-type and Pyright gates.

[Profile-aware fixture evidence](python-official-conformance.md#profile-aware-pytest-fixture-closure-checkpoint-2026-10-06).

NODE-396: Reconstruct the official Python collection profiles.

- [x] Model upstream's separate core and TodoMVC CI jobs in the manifest.
- [x] Freeze the examples job as Python 3.10, Playwright 1.58.0, pytest 8.4.1
  and `pytest-playwright==0.7.2` in a hash-locked isolated environment.
- [x] Replace exactly 25 obsolete unsuffixed TodoMVC IDs with official
  `[chromium]` IDs while keeping the other 6,501 rows byte-equivalent.
- [x] Rebase only pristine `not-run`, zero-attempt outcomes and prove all
  1,149 passes, eight skips and 5,369 not-run outcomes are preserved.
- [x] Make the profiled inventory the default for collection, audit, runner,
  fixture and result-summary tooling.
- [x] Pass four inventory and five rebase AAA tests plus strict Python gates.

[Official profile evidence](python-official-conformance.md#official-ci-profile-reconstruction-checkpoint-2026-10-06).

NODE-395: Historical root-only fixture-definition diagnostic (superseded).

- [x] Resolve all 6,526 fixture closures to 77 content-addressed definitions,
  preserving every override chain and selected definition per invocation.
- [x] Record source kind/path, line, scope, base ID, dependencies, autouse and
  parameter status plus one fail-closed semantic role for every definition.
- [x] Join exact applicability and current outcome ledgers, partitioning all
  5,360 applicable `not-run` rows by fixture-induced runtime band.
- [x] Keep the 25 TodoMVC rows with an absent `page` provider in an explicit
  unresolved band rather than silently skipping or claiming Page creation.
- [x] Re-run collection and prove byte-identical binding and definition
  ledgers; pass 14 AAA tests and all strict Python gates.
- [x] Resolve the missing TodoMVC provider by reconstructing upstream's
  separate examples profile in NODE-396 through NODE-398.
- [ ] Combine fixture bands with body-level launch analysis and execute the
  remaining applicable Python rows without treating a fixture band as proof
  that the source is browserless.

[Historical fixture-definition evidence](python-official-conformance.md#superseded-root-only-fixture-definition-diagnostic-2026-10-06).

NODE-394: Historical root-only pytest fixture diagnostic (superseded).

- [x] Collect direct, initial and transitive/autouse fixture names for all
  6,526 canonical invocations with `pytest --collect-only`.
- [x] Execute no fixture or test body and record the exact oracle interpreter,
  pytest version, command, checkout, inventory and collector hashes.
- [x] Verify canonical order and one-to-one coverage, public/internal pytest
  closure agreement and exact ordered equality with NODE-393 direct inputs.
- [x] Prove `event_loop_policy` reaches all rows and upstream `start_server`
  reaches all 6,501 non-example rows.
- [x] Pass six AAA tests plus Ruff, Python 3.9 runtime-type and Pyright gates.
- [x] Classify every resolvable fixture definition location, scope and semantic
  role in NODE-395 while isolating the one unresolved provider explicitly.

[Historical fixture-closure evidence](python-official-conformance.md#superseded-root-only-pytest-fixture-closure-diagnostic-2026-10-06).

NODE-393: Inventory source-declared pytest inputs per Python invocation.

- [x] Preserve the exact declared order on all 6,526 expanded rows and exclude
  only class-method `self`/`cls` receivers.
- [x] Cross-tabulate all 36 direct inputs against applicability and outcomes,
  including a separate parameterless queue.
- [x] Keep the direct-input audit distinct from a transitive/autouse fixture
  closure and preserve it as a separate evidence layer.
- [x] Pass 12 AAA tests, Ruff, Python 3.9 runtime-type checks and Pyright.
- [x] Collect and verify the complete pytest fixture-name closure for every
  expanded invocation in NODE-394.
- [x] Record and classify every fixture definition origin, scope and role;
  NODE-396 through NODE-398 reconstruct and resolve the TodoMVC provider.

[Direct-input evidence](python-official-conformance.md#direct-pytest-input-inventory-checkpoint-2026-10-06).

NODE-392: Add a guarded browserless official-Python evidence lane.

- [x] Require an exact manifest execution policy and a browserless capability
  declaration on every selected case.
- [x] Statically reject browser fixtures, launch, CDP connection, Context/Page
  creation and navigation in the selected function/helper closure.
- [x] Make the runner and independent provenance verifier derive the same
  browser and fixture-server requirements.
- [x] Reject `requiresFixtureServers: false` because upstream's session-wide
  autouse fixture starts both servers; prove Tailscale-only binding in both
  candidate and oracle output.
- [x] Preserve the candidate/oracle 3/3 shared asyncio-topology failures and
  merge only the matching 3/3 green stop-lifecycle rows.
- [x] Advance the Python ledger by exactly two new rows to 1,149 passed / eight
  skipped / 5,369 not run without changing ABI 103 or runtime capabilities.
- [x] Preserve those ordinary-pytest 3-pass/3-fail reports, then add a separate
  source-pinned same-thread asyncio-loop mode. Verify candidate and oracle
  6/6, and merge only its six exact keys into the current 6,526-row ledgers.
- [ ] Execute the remaining applicable Python and Core JS/TS rows; 5,360
  applicable Python `not-run` rows was the earlier NODE-392 checkpoint, not
  the current post-loop-merge ledger count.

[Browserless evidence](python-official-conformance.md#browserless-sync-lifecycle-checkpoint-2026-10-06).

NODE-391: Separate private test helpers from implementation-internal behavior.

- [x] Introduce an applicable `public-api-with-private-test-helper` Core queue
  consistent with the Python applicability policy.
- [x] Keep exact local implementation sources internal and keep non-library
  product sources in product infrastructure.
- [x] Restore 6,279 public behaviors with helper dependencies to the required
  execution queue; retain their 684 expected and 5,595 not-run outcomes.
- [x] Regenerate all 48,477 rows and pass 13 AAA tests, Ruff, Python 3.9
  compatibility, Pyright and patch checks.
- [ ] Execute all 5,595 remaining helper-dependent public rows and every other
  applicable Core JS/TS and Python row.

[Full upstream ledger](upstream-playwright-conformance.md).

NODE-390: Recompute private client access per Core invocation.

- [x] Identify the 1,951 invocations in 13 sources whose only source-level
  internal reason was private client access somewhere in the file.
- [x] Recompute direct access inside each bounded test segment and keep the
  shared channel-scope and crash fixtures explicitly internal.
- [x] Restore 1,334 ordinary public candidates, 27 public-with-private-hook
  rows and six product-infrastructure rows without changing outcomes.
- [x] Preserve all 48,477 rows, regenerate both hashes and pass 11 AAA tests,
  Ruff, Python 3.9 compatibility, Pyright and patch checks.
- [ ] Execute the newly visible public queue and continue reviewing every
  remaining applicability class.

[Full upstream ledger](upstream-playwright-conformance.md).

NODE-389: Separate example-application validation from public-API conformance.

- [x] Prove the 67 invocations across 48 `examples/**` files all carry the
  inventory's explicit `official-example` role.
- [x] Preserve every invocation and outcome while classifying those five
  example suites as product infrastructure with one exact role reason.
- [x] Retain the source inventory role in every generated audit row.
- [x] Regenerate and hash all 48,477 rows against the unchanged NODE-372
  outcome ledger; pass seven AAA tests and strict Python checks.
- [ ] Continue reviewing applicability and executing every applicable Core
  JS/TS and Python invocation.

[Full upstream ledger](upstream-playwright-conformance.md).

NODE-388: Correct the Core EventEmitter applicability accounting without
changing outcomes.

- [x] Verify all 19 `tests/library/events/*.spec.ts` files import the pinned
  upstream repository's local `./utils` EventEmitter implementation.
- [x] Mark all 477 expanded invocations with one explicit local-implementation
  reason; preserve the 36 already-internal rows and reclassify the 441 false
  public candidates as implementation-internal.
- [x] Regenerate and hash a complete 48,477-row audit against the unchanged
  NODE-372 outcome ledger.
- [x] Add a focused AAA regression test and pass all six classifier tests plus
  formatting and lint checks.
- [ ] Continue reviewing broad applicability classes and executing every
  applicable Core JS/TS and Python invocation.

[Full upstream ledger](upstream-playwright-conformance.md).

NODE-387: Compose Playwright-compatible ElementHandle keyboard behavior and
close the deterministic async Chromium keyboard discrepancies.

- [x] Pin the complete 29-case async Chromium keyboard manifest and preserve
  the ABI 102 candidate baseline plus independent Playwright 1.58.0 oracle.
- [x] Keep exact ElementHandle focus and Page keyboard input inside one
  admitted Rust command; expose only thin async/sync `press` and `type` facades.
- [x] Add Page/Frame typing, Frame query/selector evaluation and official
  Keyboard error shaping without adding a Python protocol or queue.
- [x] Pass 26 rows and one upstream skip from a fresh ABI 103 installed wheel;
  preserve both shared-environment failures and promote only green evidence.
- [x] Advance the candidate ledger to 1,147 passes / eight skips / 5,371 not
  run, with no recorded failure and verified complete-report provenance.
- [x] Pass 239/239 installed units, release Rust tests, strict Rust/Python
  gates, the 108-package proof, normalized wheel reproducibility and fresh
  6,526/6,526 collection.
- [ ] Continue every remaining Python and Core JS/TS invocation.

[Full upstream ledger](upstream-playwright-conformance.md).

NODE-386: Compose Playwright-compatible Python Page Keyboard and close the
recorded focus discrepancy.

- [x] Preserve the exact five-case async Chromium focus manifest, its 3/5
  installed candidate baseline and the independent 5/5 oracle.
- [x] Add an independently selectable Python-addon `keyboard` feature without
  moving Page identity, key mapping, delay, admission or cancellation from
  the existing Rust host.
- [x] Expose all five public operations through thin async and sync facades.
- [x] Pass 5/5 from a fresh ABI 102 / 89-feature installed wheel and advance
  the candidate ledger to 1,121 passes with no recorded failure.
- [x] Pass 232/232 installed units, strict Rust/Python gates, the 108-package
  proof and final-source normalized wheel reproducibility.
- [ ] Continue every remaining Python and Core JS/TS invocation.

[Full upstream ledger](upstream-playwright-conformance.md).

NODE-385: Compose Playwright-compatible Request timing and close its official
Chromium tranche.

- [x] Pin all four async Chromium rows from upstream
  `test_resource_timing.py` in a fixed source-verified manifest.
- [x] Preserve the installed ABI 101 baseline at 2/4 and the independent
  official Playwright oracle at 4/4 without weakening assertions.
- [x] Add the optional Rust timing sidecar, cached/redirect normalization and
  terminal snapshot sharing through model, codec, backend and composition.
- [x] Replace the navigation notice race with checked per-stream sequences and
  an exact bounded publication fence; keep Python as the thin dispatch owner.
- [x] Pass 4/4 from a fresh ABI 101 / 88-feature installed wheel, verify both
  provenance sidecars and advance candidate to 1,119 passes and oracle to 738.
- [x] Pass 226/226 installed units, strict Rust/Python gates, the 108-package
  proof and final-source normalized wheel reproducibility.
- [x] Implement the missing Python `Page.keyboard` facade for the two recorded
  focus failures.
- [ ] Continue every remaining Python and Core invocation.

[Full upstream ledger](upstream-playwright-conformance.md).

NODE-384: Close Chromium coverage for upstream `test_request_fulfill.py`.

- [x] Pin the two remaining rows and upstream file hash in an exact manifest.
- [x] Run the fresh ABI 100 installed candidate and independent official
  Playwright oracle in isolated per-case processes; both pass 2/2.
- [x] Verify both provenance sidecars and advance candidate to 1,110 passes
  and oracle to 727.
- [x] Record that the upstream file is now 3/3 for Chromium without claiming a
  new runtime capability or ABI.
- [ ] Continue every remaining Python and Core JS/TS invocation.

[Full upstream ledger](upstream-playwright-conformance.md).

NODE-383: Close the remaining async Chromium JSHandle differential without
moving native ownership into Python.

- [x] Preserve the exact 15-case manifest and its 6/15 candidate baseline plus
  independent 15/15 official Playwright oracle.
- [x] Encode nested and repeated JSHandle arguments as bounded ordered Rust
  owner slots while leaving plain-value evaluation on its original route.
- [x] Publish `get_properties()` through atomic bounded Rust batch creation and
  independent result disposal.
- [x] Match Python Date decoding and Window/primitive/Promise public previews.
- [x] Pass 15/15 from a fresh ABI 100 installed wheel and advance candidate to
  1,108 passes and oracle to 725 with verified provenance.
- [x] Pass 221/221 installed units, strict Rust/Python gates, normalized wheel
  reproducibility and the 108-package dependency proof.
- [ ] Continue every remaining Python and Core JS/TS invocation; this tranche
  is not full-corpus completion.

[Full upstream ledger](upstream-playwright-conformance.md).

NODE-382: Compose native directory input without widening colocated file-path
trust.

- [x] Add a separately selectable `file-input-native-paths` leaf over the
  existing bounded file-input and retained-preflight owners.
- [x] Validate exact input directory/multiple flags before mutation and retain
  the prepared source, target identity and cleanup reservation through
  settlement or cancellation.
- [x] Pass every directory and `webkitdirectory` row in the fixed 18-case
  official async input tranche from a fresh ABI 99 release wheel.
- [x] Preserve the remote-CDP 50 MiB guard for ordinary payload paths and keep
  both environment-limited large-file cases explicit.
- [x] Advance the candidate ledger to 1,093 passes and the oracle ledger to
  710, with independently verified provenance.
- [x] Pass 217/217 installed units, strict Python/Rust gates, normalized wheel
  reproducibility and the 108-package dependency proof.
- [ ] Continue every remaining Python and Core JS/TS invocation; do not treat
  this tranche as full-corpus completion.

[Full upstream ledger](upstream-playwright-conformance.md).

NODE-362: Compose target-origin TLS over existing HTTP CONNECT and SOCKS5
tunnels and verify the two previously unrun client-certificate rows.

- [x] Add a transport-level tunnel-TLS port that consumes an existing
  connection and cannot open a second socket.
- [x] Implement the port in the existing Rustls owner, including SNI,
  request-scoped verification and exact-origin client-certificate selection.
- [x] Inject the port into both proxy leaves without adding a concrete TLS
  dependency to either leaf; keep missing composition an explicit error.
- [x] Add two Node profiles whose dependency closures are exactly certificate
  plus HTTP or SOCKS feature unions; build and load-verify both release
  artifacts.
- [x] Pass the unchanged official HTTP-proxy+mTLS and SOCKS5+mTLS rows 3/3 per
  proxy in Library and 6/6 per proxy in Bidi, in both candidate and oracle.
- [x] Advance Core to candidate 1,309 expected / one skipped / 31 unexpected /
  47,136 not-run and oracle 992 expected / 47,485 not-run; pass 113 Rust
  request tests and 282 Node tests.
- [ ] Continue authenticated SOCKS, proxy bypass, BrowserContext proxy/
  certificate composition and every remaining Core and Python invocation.

[Full upstream ledger](upstream-playwright-conformance.md).

NODE-361: Execute every already-supported global-request client-certificate
row through the official Bidi project matrix.

- [x] Add exact browserless Bidi selections for the unchanged validation row
  and the same seven direct-request rows already proven in Library.
- [x] Keep the still-unsupported HTTPS-over-HTTP/SOCKS proxy certificate rows
  explicit and unselected; do not weaken or silently skip them.
- [x] Rebuild and load-verify the selected release artifact before candidate
  execution.
- [x] Pass validation 6/6 and direct requests 42/42 in both candidate and
  independent oracle lanes; candidate logs confirm the local Pwright adapter.
- [x] Advance the Core ledgers to candidate 1,291 expected / one skipped / 31
  unexpected / 47,154 not-run and oracle 974 expected / 47,503 not-run.
- [ ] Implement HTTPS target tunneling through HTTP CONNECT and SOCKS5, then
  execute the two proxy-plus-certificate rows and remaining corpora.

[Full upstream ledger](upstream-playwright-conformance.md).

NODE-360: Execute every already-supported global-request proxy row through the
official Bidi project matrix.

- [x] Add exact browserless Bidi selections for upstream
  `tests/library/fetch-proxy.spec.ts:47`, `:143` and `:133` without changing
  assertions or the pinned checkout.
- [x] Reuse the same source-only Tailscale fixture adapter in both lanes and
  keep every selection browser/CDP independent.
- [x] Pass authenticated HTTP CONNECT 6/6, HTTPS proxy-server ALPN 6/6 and
  SOCKS5 6/6 in both candidate and oracle lanes; candidate logs confirm the
  local Pwright adapter in all 18 invocations.
- [x] Advance the Core ledgers to candidate 1,243 expected / one skipped / 31
  unexpected / 47,202 not-run and oracle 926 expected / 47,551 not-run.
- [ ] Continue the context-request proxy rows and the complete remaining Core
  and Python corpora; this evidence-only checkpoint does not complete them.

[Full upstream ledger](upstream-playwright-conformance.md).

NODE-359: Add an independently selectable Rust SOCKS5 request connector and
establish exact official candidate/oracle evidence.

- [x] Add `pwright-api-request-connector-socks5` behind the exact
  `api-request-socks5-proxy = api-request + SOCKS5 leaf` profile; keep the base
  request and HTTP CONNECT profiles free of the leaf.
- [x] Bound the proxy URL and handshake, advertise only no-auth, encode remote
  domain names plus IPv4/IPv6 targets, parse every reply form and preserve
  tunneled application bytes by reading only the exact reply length.
- [x] Keep credentials and HTTPS request origins explicitly unsupported in
  this first leaf rather than silently degrading behavior.
- [x] Add exact official coverage for
  `tests/library/fetch-proxy.spec.ts:133`; apply the same source-only Tailscale
  fixture rewrite to candidate and oracle without modifying upstream.
- [x] Pass the unchanged official row 3/3 candidate and 3/3 oracle across the
  Library projects, with report SHA-256 values
  `19af5d94d8ccc7196d8200f229b7eafec4f0954c687dddf9c9309ef5d2428438`
  and `7926ab68bdb6f9614781b8365b047d59176203adb913d5fa168a98edcbfb87e7`.
- [x] Pass 110 release tests across eleven request crates, strict selected
  Clippy, exact one-leaf dependency proof, native build/load and a real
  Tailscale-bound remote-DNS smoke; pass Node 279/279 and strict TypeScript.
- [x] Advance the Core ledgers to candidate 1,225 expected / one skipped / 31
  unexpected / 47,220 not-run and oracle 908 expected / 47,569 not-run.
- [ ] Add HTTPS target tunneling, authenticated SOCKS and bypass, then execute
  the remaining proxy rows and complete Core/Python corpora.

[Canonical design and evidence](api-request-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-358: Compose HTTPS proxy-server transport and establish exact official
ALPN evidence without broadening the base request profile.

- [x] Accept only `http:` and `https:` proxy servers in the selected CONNECT
  leaf while keeping request origins limited to plain HTTP.
- [x] Delegate outer TLS, certificate-ignore policy and HTTP/1.1 ALPN to the
  injected Rustls connector instead of adding a second TLS implementation.
- [x] Add an exact fixed manifest row for upstream
  `tests/library/fetch-proxy.spec.ts:143` and use the same source-only
  Tailscale address adaptation in candidate and oracle lanes.
- [x] Pass the unchanged official row 3/3 candidate and 3/3 oracle across the
  Library projects; candidate/oracle report SHA-256 values are
  `6dba12195eed6bf3f7ba71b3bc2713d97acf83449db4ed28e350aef5b821d74d`
  and `429f9fa3ffec495127cc0f7310815d6eca336a488b74d197f74f97f11a5621c8`.
- [x] Pass 104 release tests across ten request crates, selected native
  build/load, strict Clippy, Node 276/276 and strict TypeScript.
- [x] Advance the Core ledgers to candidate 1,222 expected / one skipped / 31
  unexpected / 47,223 not-run and oracle 905 expected / 47,572 not-run.
- [ ] Add HTTPS target tunneling, SOCKS and bypass, then execute the remaining
  proxy rows and complete Core/Python corpora.

[Canonical design and evidence](api-request-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-357: Add an independently selectable Rust HTTP CONNECT proxy leaf and
establish official Node candidate/oracle evidence.

- [x] Add `pwright-api-request-connector-http-proxy` behind the exact
  `api-request-http-proxy = api-request + proxy leaf` profile; keep the base
  request profile free of proxy code.
- [x] Bound proxy URL, username, password and response-head bytes; redact
  credentials from diagnostics and retain any bytes read after CONNECT.
- [x] Keep Node thin: validate the public proxy object, reject unsupported
  bypass/profile selection and pass values once to the native context.
- [x] Bind official proxy/destination fixtures to Tailscale through one common
  source-only adapter without modifying the pinned upstream checkout.
- [x] Pass the unchanged official authenticated global-request proxy row 3/3
  candidate and 3/3 independent oracle across Library projects.
- [x] Pass 103 release tests across ten request crates, Node 276/276, strict
  types, exact one-leaf dependency proof, selected native load, strict Clippy
  and a real Tailscale-bound proxy smoke 2/2.
- [x] Advance the Core ledgers to candidate 1,219 expected / one skipped / 31
  unexpected / 47,226 not-run and oracle 902 expected / 47,575 not-run.
- [ ] Add separately selectable HTTPS-over-proxy, SOCKS and bypass behavior,
  then execute the remaining proxy rows and complete Core/Python corpora.

[Canonical design and evidence](api-request-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-356: Complete official async/sync request-fallback behavior while keeping
Python at ABI 96.

- [x] Pin all 55 Chromium rows from the four Page/BrowserContext async/sync
  request-fallback files in one exact source manifest, SHA-256
  `126f1bf54d8189c344eedd70a4e5d271d2bed14d7a831c8bb0a3dc88c61a5b00`.
- [x] Keep routing logic in Rust; normalize the published Request header map,
  add thin async/sync `all_headers()` and `header_value()` facades, and retain
  only effective public identity in a bounded 64-entry cache until terminal
  request publication.
- [x] Run sync route handlers off the bridge loop so nested sync Page calls can
  complete, while route commands still settle through the same bounded loop.
- [x] Add four AAA regressions and pass all 207 source and installed-wheel unit
  tests, Ruff and strict Pyright.
- [x] Pass source candidate and independent oracle 55/55: async 26/26 and sync
  29/29. Candidate report hashes are
  `fcd1e06378b3b834c4d48aa32238d3df42865ced2895b9916033ac24e8f4062d`
  and
  `2fea136fd2b975e40ed40b068be28e85967ae129af13f2402a34051529ff89c5`.
- [x] Build two ABI 96 / 83-feature release wheels and reproduce their payloads
  after CycloneDX normalization. SHA-256 values are
  `5d204e51bc55abe004c02972651719f1f8994e533a7e1faf0a076edc42b1ba4b`
  and `049af7ea076ceaf93ddf8c11ee6b11c798b6492c637aae5bce0711ca91455a72`.
- [x] Generate the provisional cumulative ledger: 1,070 passed, zero failed,
  seven skipped and 5,449 not run, SHA-256
  `14f2e04ad35fbfaff95109c326ff6064d955d737f192bee21eea60c062623256`.
- [ ] Rerun the fixed manifest from the installed release wheel and promote the
  provisional ledger after the shared dedicated Chrome CDP endpoint recovers.
  It became unresponsive during another concurrent task; no restart was made
  because unknown tabs must be preserved.
- [ ] Continue all 5,449 explicitly not-run Python rows and the complete Core
  JS/TS corpus; this checkpoint does not complete the global conformance goal.

NODE-355: Advance Python to ABI 96 with BrowserContext request and
`Route.fetch()` composition.

- [x] Add `context-api-request` as the exact Rust-owned composition of the
  existing API request client, Context cookies, device/user-agent settings,
  HTTP credentials and TLS-error policy; keep Python free of protocol and HTTP
  wire ownership.
- [x] Add `route-fetch` as the exact union of `context-api-request`,
  `page-routes` and `route-fulfill-api-response`, including typed JSON
  fulfillment and callback-safe sync `APIResponse` access.
- [x] Preserve Context/Page request identity, live cookie sharing, base URL,
  browser user agent, credentials, TLS policy, typed params and body forms,
  close disposal/reason, redirect limits, timeout and connection-reset retry.
- [x] Pin Playwright Python v1.58.0 commit
  `47a5d35ef4f815a2021349f86ae391f7c20c02d6` in two fixed manifests whose
  SHA-256 values are
  `94067ae8c5817c7c21e7e18186065cb34501a56938fb302eee6b721983958f86`
  and `5c2a3ca541d9eb221a8440be377d06d6c1f35e457730999e0d4103e82722fbfd`.
- [x] Pass final installed candidate and independent official oracle 108/108:
  Context async 51/51, Context sync 47/47, route async 6/6 and route sync 4/4.
- [x] Pass 203 installed-wheel units, 1,038 all-feature host tests with 26
  declared ignores, two addon tests, strict Rust/Python gates and exact
  108-package feature-union proofs.
- [x] Reproduce final wheel payloads after normalizing only CycloneDX identity;
  wheel SHA-256 values are
  `6662eeca620742ff25d49d2aa168f7c7a165cd30f15256180f315476c3fae43a`
  and `ca86fdf2db491f38be544131f0cb8a0d542769f0105f81d482ccd937de4f9f1e`.
- [x] Advance exactly 108 rows to 1,015 passed, zero failed, seven skipped and
  5,504 not run, ledger SHA-256
  `a0c33f08557de51ee7665720a393469fe3538c9c8f7aa421cc3f5b684e57e178`.
- [ ] Continue all 5,504 explicitly not-run Python rows and the complete Core
  JS/TS corpus; this checkpoint does not complete the global conformance goal.

NODE-346: Advance Python to ABI 93 with Rust-owned route redirects.

- [x] Pin both unchanged Chromium redirect-chain rows and record an exact ABI
  92 red lane at 0/2 plus an independent official oracle at 2/2. Manifest,
  red and oracle SHA-256 values are
  `0d745e50edccc9db198df552fd0eae83947c53e2a4a6706326ae7d873f6eef14`,
  `5395181269a7dace0d878dfd9f91b7ff88fb5183175b08f212b5ecf2f707edd8`
  and
  `a96f71c0e5b63e71e37c02df982895951ba63711f68c47f2bf910a04a08153cf`.
- [x] Add separately selectable `route-redirects` as the exact union of
  `page-routes` and `request-history`; keep request and redirect-chain
  ownership out of the binding.
- [x] Auto-continue redirect pauses in Rust without another user callback or
  finite-use decrement. Reapply explicit headers after removing `Cookie`,
  bound state to 64 live chains and retire it on terminal network/target/Page
  transitions with explicit capacity errors.
- [x] Reuse the exact Rust request-history owner and keep Python thin: link
  `redirected_from` / `redirected_to`, preserve Fetch's complete header
  snapshot and share Request identity across async and sync route adapters.
- [x] Pass final installed candidate 2/2, SHA-256
  `dcd31ae1b4590970cb7c46d5aed5df923902ecbefe6983e1dd460a759e9d2339`.
- [x] Rerun all 44 async Chromium Page-route rows: 43 pass and 1 fail, report
  SHA-256
  `3f3ef3d3f0f8b3823451e4ce8f76129e9a43d324ff482a8fa2ab3f813822cac9`.
- [x] Pass 195 installed-wheel units, 1,030 all-feature host tests with 26
  declared ignores, two addon tests, focused redirect/capacity/exact-request
  tests, strict Rust/Python gates, source validation and the exact dependency-
  union proof. Fresh final-wheel collection is 6,526/6,526.
- [x] Reproduce both repaired wheel payloads after normalizing only CycloneDX
  identity. Their SHA-256 values are
  `95512f3ea04a9c8147fe6441acae1b95705d00b99597cf77186a4c62bb2bd4bc`
  and
  `9833b1fa82f1659adb555f627fce2a8f302627006a51526670c93b728da8fcf2`.
- [x] Advance both selected rows: 853 passed, zero failed, seven skipped and
  5,666 not run, ledger SHA-256
  `f4dca295f673c6b680020e02c4f99d8a7a8cd2dd261696dcdf7638a32ce9a362`.
- [ ] Repair the sole remaining Page-route discrepancy, service-worker
  interception; then continue the remaining Python and complete Core JS/TS
  corpora.

NODE-345: Advance Python to ABI 92 with Rust-owned
`Route.fulfill(response=APIResponse)` composition.

- [x] Pin the unchanged global-fetch Chromium row and record an exact ABI 91
  red lane at 0/1 plus an independent official oracle at 1/1. Manifest, red
  and oracle SHA-256 values are
  `1a1ec11a564aee020317fdcb62e220fbe5b6f950e1687622e2f428b8d737f42c`,
  `6e224bf6ebca1f40440f79062103e8c32dc13c357078570e5e1506889219f300`
  and
  `ee4517f65b628199cc353ae00aa31837280ab452d201e2121cd44d69aa47eed8`.
- [x] Add separately selectable `route-fulfill-api-response` as the exact
  union of `page-routes` and `api-request`; do not add a new package or move
  API-response ownership into the binding.
- [x] In Rust, inherit status, headers and content type and copy the retained
  response body only after `body_bounded()` proves it fits the existing 1 MiB
  route limit. Preserve the response on an over-limit rejection.
- [x] Keep Python thin: pass native `APIResponse` identity for the ordinary
  path, unwrap the same identity in sync mode and retain the existing explicit
  body/path override paths. Add `Response.json()` only as JSON decoding over
  the Rust-owned response body.
- [x] Pass final installed candidate 1/1, SHA-256
  `1f03ff84db73ce99fe0b3c60122d087b1682a7431121318889771336f7354a2a`.
- [x] Rerun all 44 async Chromium Page-route rows: 41 pass and 3 fail, report
  SHA-256
  `8362386e58f0fcf6641ef5fdb80410aa840f516a7335be84da3e973dd7f8c76e`.
- [x] Pass 194 installed-wheel units, 1,026 all-feature host tests with 26
  declared ignores, two addon tests, the focused bounded-body Rust test,
  strict Rust/Python gates, source validation and the exact dependency-union
  proof. Fresh final-wheel collection is 6,526/6,526.
- [x] Reproduce both repaired wheel payloads after normalizing only CycloneDX
  identity. Their SHA-256 values are
  `20cddd19a6d96ab92b9c93d4d24b27c473cffb67682ecbef6f98e3a6b343c041`
  and
  `a171ac67d754fe24767c4a9d65dba550fb42bb62d687db885b1cd4c48edf5b3c`.
- [x] Advance the selected row: 851 passed, zero failed, seven skipped and
  5,668 not run, ledger SHA-256
  `09089b9c798ccf3a4e3f93645839257783b16a39c03b8c737ade9f34b0d40d48`.
- [ ] Repair the three remaining Page-route discrepancies: service-worker
  interception, navigation redirect interception and subresource redirect
  chains; then continue the remaining Python and complete Core JS/TS corpora.

NODE-344: Keep Python at ABI 91 and align browser-generated CORS preflights.

- [x] Pin the two unchanged POST/method Chromium rows and record an exact ABI
  91 red lane at 0/2 plus an independent official oracle at 2/2. Manifest,
  red and oracle SHA-256 values are
  `8c17ca7e9acffb0fdad0f3523a4f83eadf2f3649371a3c42d889f75e921c01d2`,
  `87541e590afec054f4d0d6fbc49ddd80104de19c6d55e119f1f8726ca3081694`
  and
  `f596e5b7e0c4b44df14a7f2c15e86396e81a0c3e87661a6d2a31cdbc03dd2991`.
- [x] Recognize only bounded `OPTIONS` requests that carry
  `Access-Control-Request-Method`, and only when a registered user route
  matches the URL; preserve ordinary `OPTIONS` routing.
- [x] Fulfill the preflight directly in Rust with Playwright's 204 response,
  echoed origin/method/requested headers and credentials, without publishing
  a Python `RouteCall` or consuming its finite-use budget.
- [x] Cover header echoing, ordinary `OPTIONS` and route-budget preservation
  with three Arrange-Act-Assert Rust tests.
- [x] Pass final installed candidate 2/2, SHA-256
  `61ae975f214727ccd92c77ba760d8e2b6b9936d1a249c32091c07fcb99d8ee4e`.
- [x] Rerun all 44 async Chromium Page-route rows: 40 pass and 4 fail, report
  SHA-256
  `91158df512ed9ba3b86c511b037758ffb6e82c193380604849622cd11ac8da5d`.
- [x] Pass 190 installed-wheel units, 1,026 all-feature host tests with 26
  declared ignores, two addon tests, focused preflight tests, strict
  Rust/Python gates, source validation, the 108-package dependency proof,
  normalized wheel reproducibility and fresh 6,526/6,526 collection.
- [x] Advance both selected rows: 850 passed, zero failed, seven skipped and
  5,669 not run, ledger SHA-256
  `5a0465a5fdeef895cae08b30b2babca1b794b28474a5cf32118962e1556ab0d4`.
- [x] Hand `Route.fulfill(response=...)` to NODE-345.
- [ ] Repair the other three Page-route discrepancies: service-worker
  interception, navigation redirect interception and subresource redirect
  chains; then continue the remaining Python and complete Core JS/TS corpora.

NODE-343: Advance Python to ABI 91 with exact routed
`Request.response()` correlation.

- [x] Pin the unchanged encoded-URL Chromium row and record an exact ABI 90
  red lane at 0/1 plus an independent official oracle at 1/1. Manifest, red
  and oracle SHA-256 values are
  `bb1d45e9ed7cce9eb55e00f1b90d07c96cb6f8d6fcf3b772f6844a2229afe83f`,
  `e49f5cc30cbdcbe4a5f17c52b354d5719eaf859917602d160790887f1e752492`
  and
  `be3df59d21eac14f3587b90a52d1df8291b5ac7664a8794091c3c29b49bb4d3f`.
- [x] Carry the bounded Chromium `Fetch.networkId` through the Rust route call
  and fallback path; do not infer identity from URL, method or event timing.
- [x] Reuse the bounded Rust request-history owner and its settled Response
  owner. Wake exact-ID waiters on record/reconciliation/close, observe
  Page/session termination and preserve the found owner across eviction.
- [x] Keep Python thin: `Request.response()` delegates the native identity to
  Rust and returns a `Response` that references the same public `Request`;
  Python retains no parallel network registry.
- [x] Pass final installed candidate 1/1, SHA-256
  `1b35b052b10603a7a11d2976e8b7dbb71d23781919e99f3023317170c1ab3142`.
- [x] Rerun all 44 async Chromium Page-route rows: 38 pass and 6 fail, report
  SHA-256
  `876cbfb7d06cbdff10d5485425f3d0ddbfed8f3c175fdf784c1a5862a68ea108`.
- [x] Pass 190 installed-wheel units, 1,023 all-feature host tests with 26
  declared ignores, two addon tests, focused exact-identity tests, strict
  Rust/Python gates, source validation, the 108-package dependency proof,
  normalized wheel reproducibility and fresh 6,526/6,526 collection.
- [x] Advance the selected row: 848 passed, zero failed, seven skipped and
  5,671 not run, ledger SHA-256
  `58c32ca19ac776657dcfd4e823ca58d9d57acf519b5c47d371f3c2deb12520d3`.
- [x] Hand the two CORS POST/method discrepancies to NODE-344.
- [ ] Repair the four remaining Page-route discrepancies: service-worker
  interception, navigation redirect interception, subresource redirect chains
  and `Route.fulfill(response=...)`; then continue the remaining Python and
  complete Core JS/TS corpora.

NODE-342: Advance Python to ABI 90 with composable route matching and
Rust-owned fulfill-path I/O.

- [x] Preserve Python regex/predicate and handler object identity in a thin
  binding matcher; register one bounded Rust catch-all and delegate nonmatches
  to Rust-owned LIFO fallback instead of implementing a second route stack.
- [x] Count `times` only on matcher hits, wait for actual route settlement and
  defer native unroute until the final matching handler completes.
- [x] Add opt-in `route-fulfill-path`: Rust validates a UTF-8 path of at most
  4 KiB, requires a regular file, reads at most 1 MiB asynchronously and
  replaces caller `Content-Type` with extension-derived MIME.
- [x] Pin the two unchanged official Chromium rows and pass final installed
  candidate plus independent oracle 2/2. Candidate and oracle report hashes
  are
  `35a123a706cf2c645b7db4afab7d67b50f8253aaa76b359a13b508fb2289d451`
  and
  `0ba256ac00477dd62dee767a5e7b513e2e49fc60e889629e5382a4b318304d35`.
- [x] Rerun all 44 async Chromium Page-route rows: 37 pass and 7 fail, report
  SHA-256
  `489b44b15c599c0e7bf7ae83c5a6d5057fded99c8ee8f23b30e57424451c3dfa`.
- [x] Pass 189 installed-wheel units, 1,022 all-feature host tests with 26
  declared ignores, two addon tests, focused feature tests, strict
  Rust/Python gates, source validation, the 108-package dependency proof,
  normalized wheel reproducibility and fresh 6,526/6,526 collection.
- [x] Advance both selected rows: 847 passed, zero failed, seven skipped and
  5,672 not run, ledger SHA-256
  `e423606ad57b9f2e0bdb95f4e4a74e71d65ea9582890b864e86ba76837817028`.
- [x] Hand the encoded-route `Request.response()` discrepancy to NODE-343.
- [ ] Repair the six remaining Page-route discrepancies: service-worker
  interception, navigation redirect interception, CORS POST/method behavior,
  subresource redirect chains and `Route.fulfill(response=...)`; then
  continue the remaining Python and complete Core JS/TS corpora.

NODE-340: Advance Python to ABI 89 with exact composable Request Frame identity.

- [x] Pin six unchanged async Chromium request-frame rows plus the independent
  Page-route request-frame row; preserve the popup-first unavailable boundary.
- [x] Resolve routed `Fetch.frameId` through the existing topology and carry
  one optional portable Frame ID in the bounded Rust route call.
- [x] Keep ordinary request Frame descriptors and cursor-relative availability
  in the existing bounded Rust network adapter. Seed only Frames that predate
  an explicitly created Page cursor; do not import future popup topology into
  an already-buffered first request.
- [x] Reuse the Page Frame registry for async/sync `Request.frame`, derive
  `Response.frame` from the exact Request and expose
  `ElementHandle.content_frame()` through the existing Rust Handle owner.
- [x] Pass final installed candidate and independent oracle 6/6 plus 1/1. The
  candidate report hashes are
  `0ce2544dc09b85e6e8d7574fda6dbaa88194f2a06b1321527fc8747fbb4249d7`
  and
  `12b08216247c408f5023fb11fb1caf8df9f0219c74502a9f62ce032f8a1cc503`.
- [x] Rerun all 44 async Chromium Page-route rows: 35 pass and 9 fail, report
  SHA-256
  `122d5db740b95603aecee9d70713d465df19be09c60daac55d1898510cb32e4d`.
- [x] Pass 185 installed-wheel units, 1,043 all-feature host tests with 36
  declared ignores, two addon tests, the 14-case CDP network leaf, the exact
  composition regression, strict Rust/Python gates, source validation, the
  108-package dependency proof, normalized wheel reproducibility and fresh
  6,526/6,526 collection.
- [x] Advance six previously not-run rows while replacing the already-passed
  popup-first evidence: 845 passed, zero failed, seven skipped and 5,674 not
  run, ledger SHA-256
  `673973f03e77d0291596cfbc971579c70d98831f634ab74606a43b37282375c7`.
- [ ] Repair the nine remaining Page-route discrepancies, then continue the
  remaining Python and complete Core JS/TS corpora.

NODE-339: Advance Python to ABI 87 with one composable request-lifecycle event
stream.

- [x] Pin the two unchanged async Chromium Page-route abort rows; preserve the
  intermediate 1/2 diagnostic that exposed missing exact
  `Response.request` correlation.
- [x] Extend the existing Rust `request-events` owner to publish request,
  request-finished and request-failed notices through one bounded FIFO, with
  terminal failure text charged to the same retained-byte budget.
- [x] Preserve one portable public request ID while retaining the Chromium
  native ID only inside Rust history records for exact navigation response
  correlation; do not guess by URL or add a second Python registry.
- [x] Pass final installed candidate and independent oracle 2/2 plus five
  candidate repeats 10/10. The final candidate and oracle report SHA-256
  values are
  `daa8311542664a25242b7c0877c98b03302b5b7542f13c0b05c4ca6cad050811`
  and
  `a8f35888e8ba83c2b8eca51df9ef6b975c860084a77ebdf571a502d86c1b281c`.
- [x] Rerun all 44 async Chromium Page-route rows: 34 pass and 10 fail with
  exactly the two selected discrepancies fixed, report SHA-256
  `046feb19615e7e5aee2c37271108f4f0c3705dad7b7c0112dc61ee2ddca87ae8`.
- [x] Pass 182 installed-wheel units, 1,043 all-feature host tests with 36
  declared ignores, two addon tests, strict Rust/Python gates, source
  validation, the 108-package dependency proof, normalized wheel
  reproducibility and fresh 6,526/6,526 collection.
- [x] Advance the two selected rows to `passed`: 839 passed, zero failed,
  seven skipped and 5,680 not run, ledger SHA-256
  `b8c7bebbeb3f2c16880fb7fc08dd30b15d56d504d091c264b732641ef8e81e0f`.
- [ ] Repair the 10 remaining Page-route discrepancies, then continue the
  remaining Python and complete Core JS/TS corpora.

NODE-338: Keep Python ABI 86 and align Route.fulfill HTTP reason phrases.

- [x] Pin the unchanged Chromium 422 status-text row and preserve the exact
  ABI 86 red baseline failure caused by Chrome's `Unprocessable Content`.
- [x] Keep the complete Playwright status-to-reason mapping in Rust and send
  its constant result as `Fetch.fulfillRequest.responsePhrase`; unknown
  100-999 statuses use Playwright's `Unknown` fallback.
- [x] Pass installed candidate and independent oracle 1/1 plus five candidate
  repeats 5/5.
- [x] Rerun all 44 async Chromium Page-route rows in isolation: 32 pass and
  12 fail with no regression, report SHA-256
  `b1cbd5689624ca8b3e24a5c301d5522b4a511781f0bbbd58dca02ba9611e6769`.
- [x] Pass three focused Rust regressions, the 517/517 `page-routes` host
  profile with eight declared live ignores, 180 installed-wheel units,
  all-feature strict Clippy, formatting, source validation, the 108-package
  dependency proof and normalized wheel reproducibility.
- [x] Advance the selected row to `passed`: 837 passed, zero failed, seven
  skipped and 5,682 not run, ledger SHA-256
  `8a6d6a9169086e482fbd8acf6d81162ef42ab209f18c2c174238e5feb85e2ef5`.
- [ ] Repair the 12 remaining Page-route discrepancies, then continue the
  remaining Python and complete Core JS/TS corpora.

NODE-337: Keep Python ABI 86 and promote the additional Page-route public
header-shape rows fixed by NODE-336.

- [x] Rerun all 44 async Chromium Page-route rows in isolated subprocesses:
  31 pass and 13 fail, report SHA-256
  `c1839f9643e9a6c411cb15966a73587bb8f7484db46454f1caf4fb5bb88eef92`.
- [x] Pin native `Referer` and `Origin` lowercase publication in a two-row
  manifest, SHA-256
  `1e705cd72b24e45d93d4242d8698c43d55c302c2676fcb0736beaec1c1b724ce`.
- [x] Preserve the exact ABI 84 red baseline 0/2, pass installed candidate and
  independent oracle 2/2, and pass five candidate repeats 10/10.
- [x] Keep the previously recorded data-URL/equal-request instability outside
  the ledger even though both happened to pass this diagnostic.
- [x] Advance the two stable rows to `passed`: 836 passed, zero failed, seven
  skipped and 5,683 not run, ledger SHA-256
  `eb6b687f25461fa8d8f4182455215dbcf565a47296427680357866df79cc8553`.
- [ ] Repair the 13 remaining Page-route discrepancies, then continue the
  remaining Python and complete Core JS/TS corpora.

NODE-336: Advance Python to ABI 86 with Rust-owned Page and Context extra HTTP
headers.

- [x] Pin three unchanged async Chromium route/referrer rows and preserve an
  exact ABI 84 candidate red baseline of 0/3.
- [x] Add thin async/sync Page and BrowserContext setters that transfer one
  bounded string mapping to the existing Rust header owner.
- [x] Compose the Page-specific implicit navigation referer into route request
  snapshots when Fetch omits it; preserve native referers and lowercase CDP
  request-header names at the Rust public boundary.
- [x] Pass final installed candidate/oracle 3/3 and five-repeat candidate
  stress 15/15.
- [x] Pass 180 installed-wheel units, the exact composed host profile 552/552
  with eight declared live ignores, and the all-feature host profile with
  1,018 library plus 22 integration passes and 26 plus 10 declared live
  ignores. Pass two addon tests, strict Clippy/format/Ruff/Pyright, source
  validation, the 108-package dependency proof, normalized wheel
  reproducibility and 6,526/6,526 collection.
- [x] Advance the three selected Chromium rows to `passed`: 834 passed, zero
  failed, seven skipped and 5,685 not run, ledger SHA-256
  `a0c1caa9d38619302ae8b0eaa49450d1c9db9a5db80cb44e4fa6fb5e69c4b783`.
- [ ] Continue remaining Python and the complete Core JS/TS corpus.

NODE-335: Advance Python to ABI 85 with Rust-owned composable route fallback
and handler-specific unroute.

- [x] Pin eight unchanged async Chromium Page/BrowserContext rows and preserve
  an exact ABI 84 baseline of seven failures plus one existing pass.
- [x] Keep the traversal cursor and effective request overrides in Rust;
  traverse Page before Context and newest before oldest, rematch dynamic URLs,
  merge fallback overrides field by field and settle exactly once.
- [x] Remove only the registrations matching Page/Context scope, string
  pattern and original user-handler identity; keep sync wrapper identity thin.
- [x] Match upstream favicon interception by aborting the internal favicon
  request before dispatching user route handlers.
- [x] Pass final installed candidate/oracle 8/8 and five-repeat candidate
  stress 40/40, advancing seven actual discrepancies with no losses.
- [x] Pass 176 installed-wheel units, 1,037 all-feature host/integration tests
  with 36 explicit live ignores, two addon tests, strict Clippy/format/Ruff/
  Pyright, source validation, the 108-package dependency proof, normalized
  wheel reproducibility and 6,526/6,526 collection.
- [x] Advance the eight selected rows to `passed`: 831 passed, zero failed,
  seven skipped and 5,688 not run, ledger SHA-256
  `4303259b2a6813aa7e3fc8017d89d1cc5d62a38bed6b66eacc3af5600d576722`.
- [ ] Continue regex/predicate matchers, `ignoreErrors`, remaining Python and
  the complete Core JS/TS corpus.

NODE-334: Advance Python to ABI 84 with Rust-owned finite route budgets.

- [x] Pin three unchanged async Chromium Page/BrowserContext `times` rows;
  preserve candidate red 0/3 and independent oracle green 3/3 evidence.
- [x] Store one optional finite count in the Rust registration, atomically
  match/decrement/remove it under the route-state lock, and preserve
  `times=0` as unlimited.
- [x] Refresh affected current sessions after the terminal route action and
  continue transient stale-pattern pauses directly in Rust without invoking
  a released Python callback.
- [x] Release an expiring callback before invoking it; add direct negated
  assertion aliases as thin delegates to the existing `.not_to` facade.
- [x] Pass final installed candidate/oracle 3/3 and five-repeat candidate
  stress 15/15.
- [x] Pass 173 installed-wheel units, 1,011 all-feature host tests with 26
  explicit live ignores, strict Clippy/format/Ruff/Pyright, source validation,
  the 108-package dependency proof, normalized wheel reproducibility and
  6,526/6,526 collection.
- [x] Advance the three Chromium rows to `passed`: 823 passed, zero failed,
  seven skipped and 5,696 not run, ledger SHA-256
  `26023955d6cd01d96099cf7b07ef479d028e86a9fd60038e14c786479f467439`.
- [ ] Continue regex/predicate matchers, `fallback()`, handler-specific
  unroute, remaining Python and the complete Core JS/TS corpus.

NODE-333: Keep Python ABI 83 and align route-glob question-mark semantics.

- [x] Pin the unchanged async Chromium question-mark row; preserve candidate
  red and independent oracle green evidence.
- [x] Treat `?` as a literal in the Rust matcher and parity-escape it for CDP
  Fetch patterns so native prefiltering and authoritative matching agree.
- [x] Bound compiled patterns to 8,193 bytes and explicitly reserve the fixed
  256-pattern transient vector before creating protocol values.
- [x] Pass final installed candidate/oracle 1/1 and candidate repeat 5/5.
- [x] Pass 172 installed-wheel units, 1,008 all-feature host tests with 26
  explicit live ignores, strict Clippy/format/Ruff/Pyright, source validation,
  the 108-package dependency proof, normalized wheel reproducibility and
  6,526/6,526 collection.
- [x] Advance the official row to `passed`: 820 passed, zero failed, seven
  skipped and 5,699 not run, ledger SHA-256
  `aaa91984391a096225cae1684c0f1879249b41feedd8efad7a5203053087e6d0`.
- [ ] Investigate the two current-Chrome unstable Page-route rows separately;
  both reproduce on the unchanged NODE332 wheel and are not attributed to this
  matcher change.
- [ ] Continue the 22 observed Page-route failures, remaining Python corpus
  and complete Core JS/TS corpus.

NODE-332: Advance Python to ABI 83 with the complete continue-override value.

- [x] Pin all ten unchanged Chromium rows from async
  `test_request_continue.py`; preserve the ABI 82 red baseline of 2 passed / 8
  failed and independent oracle 10/10.
- [x] Add a bounded Rust `RouteContinue` value with optional URL, method,
  headers and POST data, same-protocol URL validation and one typed
  `Fetch.continueRequest` serialization path.
- [x] Keep Python thin: normalize public mapping/string/bytes/JSON values and
  pass them through ABI 83 without owning request identity or interception
  state.
- [x] Pass final installed candidate 10/10, 172 installed-wheel units, 1,007
  all-feature host tests with 26 explicit live ignores and 2 addon tests.
- [x] Pass strict Clippy, formatting, Ruff, Pyright, source validation, the
  108-package dependency proof, normalized wheel reproducibility and the
  complete 6,526/6,526 collection.
- [x] Advance all ten official rows to `passed`: 819 passed, zero failed,
  seven skipped and 5,700 not run, ledger SHA-256
  `43861212714734aa05343ef0ff48a87977136adbaecfe9d0c3c461bfb50af2f6`.
- [ ] Repair the remaining Page-route discrepancies, then continue the
  remaining Python and complete Core JS/TS corpora.

NODE-331: Execute and classify the complete async Page-route Chromium file.

- [x] Run all 44 unchanged Chromium rows in isolated subprocesses with a
  30-second per-row hard timeout; preserve the 18 pass / 25 fail / 1 error
  baseline, SHA-256
  `1f40d74f878e0729ddbb7acf7dbd0a3f98c11cabc1530d78860a91ef983a50f4`.
- [x] Pin the 18 existing green rows in
  `page-route-existing-coverage-manifest.json`, SHA-256
  `da5aacd59a19ee1852ffe7d7c0918a32cd8115895ce770ee409523297f8cdece`.
- [x] Pass installed candidate and independent oracle 18/18.
- [x] Advance those rows to `passed`: 809 passed, zero failed, seven skipped
  and 5,710 not run, ledger SHA-256
  `0408aef3e6fa958f60ce74998e4326e40aef2bc906851efa35e9b656e9b5940c`.
- [ ] Repair the 26 explicit discrepancies by cluster: continue overrides;
  regex/unroute/times; requestfailed and redirect chains; service-worker/data-
  URL rules; Page headers; CORS preflight; fulfill path/response/status text;
  resource-profile setup; then rerun the whole file.

NODE-330: Promote the already-supported ABI 82 unroute-all behavior rows.

- [x] Pin async/sync default Page/Context removal in
  `route-unroute-all-basic-manifest.json`, SHA-256
  `f8b3136e07261dcad3ea631ace025ba68309c8db5c6e1f1b3ddc37be73a58c00`.
- [x] Pin the two async in-flight wait/fulfill rows in
  `route-unroute-all-wait-manifest.json`, SHA-256
  `329eee6cd757a0fa7c8f03f0b61343c7df627bb9717a9b4f0bf143eb47816998`.
- [x] Pass installed candidate and independent oracle 6/6 without source,
  assertion, timeout or browser-policy changes.
- [x] Advance all six rows to `passed`: 791 passed, zero failed, seven skipped
  and 5,728 not run, ledger SHA-256
  `d7255f3d41c76a26528036c0eaee54b788a48b2323650b3700f2da7441f7cb05`.
- [ ] Continue handler-specific unroute, `fallback()`, regex/predicate matchers,
  `ignoreErrors`, then remaining Python and complete Core JS/TS corpora.

NODE-329: Advance Python to ABI 82 with scoped unroute-all settlement.

- [x] Pin the two unchanged async Page/Context HAR-abort rows in
  `route-unroute-all-manifest.json`, SHA-256
  `9288dc65838089a70eae6b2cf258e6efaf82727286da0432d6f0125ac0cee429`.
- [x] Preserve the red baseline: both rows fail only because Page and
  BrowserContext `unroute_all()` are absent.
- [x] Remove exact Rust registration scopes, refresh affected session Fetch
  patterns and make `behavior="wait"` observe both queued and settling actions
  through a cancellation-safe fixed-capacity owner.
- [x] Correlate late Chromium error-document commits with the newest started
  loader while preserving interruption for observed competing loaders.
- [x] Pass final installed candidate/oracle 2/2, candidate stress 20/20, 171
  installed-wheel units and 1,004 all-feature host tests with 26 explicit live
  ignores.
- [x] Pass strict Clippy, formatting, Ruff, Pyright, source validation,
  normalized wheel reproducibility and the 108-package dependency proof;
  recollect 6,526/6,526.
- [x] Advance both rows to `passed`: 785 passed, zero failed, seven skipped and
  5,734 not run, ledger SHA-256
  `b58f55b19a62286fdf0c1b06040c017c1d9b00666b3e139847eaf8cc42027b6f`.
- [ ] Continue handler-specific unroute, `ignoreErrors`, regex/predicate and
  `times`, external/ZIP/update HAR, then remaining Python and Core JS/TS.

NODE-328: Advance Python to ABI 81 with composable BrowserContext routes.

- [x] Pin all four unchanged async/sync Context HAR URL-filter/ordinary-route
  composition rows in `context-har-url-filter-manifest.json`, SHA-256
  `18679b37e9b90eb925e3732f129d06eed488211246b912eb2770a803ed1622cd`.
- [x] Preserve the red baseline: all four rows fail only because
  `BrowserContext.route()` is absent.
- [x] Reuse the explicit Rust Context scope, Page-over-Context precedence,
  future-Page/OOPIF initialization and bounded Fetch queues; keep Python to
  handler identity and sync/async callback adaptation.
- [x] Pass final installed candidate/oracle 4/4, candidate stress async 20/20
  plus sync 20/20, 169 installed-wheel units and 1,003 all-feature host tests
  with 26 explicit live ignores.
- [x] Pass strict Clippy, formatting, source validation, normalized wheel
  reproducibility and the 108-package dependency proof; recollect 6,526/6,526.
- [x] Advance all four rows to `passed`: 783 passed, zero failed, seven skipped
  and 5,736 not run, ledger SHA-256
  `c0c16280690a496ffdc374fa77ad1792aedd6670a9c61026d70fc042bce77b17`.
- [ ] Continue regex/predicate and `times`/unroute variants, external/ZIP/
  update HAR, then the remaining Python and complete Core JS/TS corpora.

NODE-327: Align missing-entries HAR fallback while keeping Python ABI 80.

- [x] Pin the unchanged async/sync bad-HAR fallback rows in
  `context-har-invalid-fallback-manifest.json`, SHA-256
  `6d26bdd0caa16f37de43ec9c45ddc6d3534260f12d54dea5cf2e39349ceccc6a`.
- [x] Preserve the red baseline: both rows fail because missing `log.entries`
  is rejected during registration.
- [x] Parse only missing `entries` as a zero-entry archive; preserve explicit
  errors for invalid JSON, missing `log`, malformed entries, external content
  and all bounds.
- [x] Pass final installed candidate/oracle 2/2 and candidate stress async
  10/10 plus sync 10/10; pass 168 installed units, HAR policy 5/5 and host
  1,003/26 ignored.
- [x] Pass strict Clippy, formatting, source validation, normalized wheel
  reproducibility and the 108-package dependency proof.
- [x] Advance both rows to `passed`: 779 passed, zero failed, seven skipped and
  5,740 not run, ledger SHA-256
  `a545a96742ef986a132be551627450e08c8d1fd0ecbfca8a1fe8e5ab142328c8`.
- [ ] Continue Context URL-filter/ordinary-route composition, regex filters,
  external/ZIP/update/unroute capability families, then the remaining corpora.

NODE-326: Advance Python to ABI 80 with composable BrowserContext HAR replay.

- [x] Pin the unchanged async/sync method/redirect, fallback-continue and
  default-abort rows in `context-har-replay-manifest.json`, SHA-256
  `7ed9f484469317a0f4e979d9314ad7b4d203a30e056ea9f2d4e4a24b5219c4ea`.
- [x] Generalize Rust route registrations to explicit Page/Context scope:
  Page always precedes Context, newest wins within one scope, and Context scope
  remains live across an empty Context for future Pages/OOPIFs.
- [x] Bridge early initialization-time Fetch pauses through a fixed 64-entry
  exact Page/Context/session owner and reclaim it on error, detach or topology
  reconciliation.
- [x] Keep Python thin and reject update, ZIP/external content, regex/predicate
  filters and unsupported Context route variants explicitly.
- [x] Pass final installed candidate/oracle 6/6, candidate stress async 30/30
  plus sync 30/30, 168 installed-wheel units and 1,003 all-feature host tests
  with 26 explicit live ignores.
- [x] Pass formatting, strict host/addon Clippy, targeted Ruff, Pyright and the
  108-package dependency proof; recollect 6,526/6,526.
- [x] Advance all six rows from `not-run` to `passed`: 777 passed, zero failed,
  seven skipped and 5,742 not run, ledger SHA-256
  `c52d8ae2a549fc2cfa75e745969863363c8e10705daea947e46a9446d58b89a7`.
- [ ] Continue external/ZIP/update HAR, Context ordinary routing/URL filtering,
  `unroute_all`, then the remaining Python and complete Core JS/TS corpora.

NODE-325: Promote the complete unchanged async/sync Page HAR URL-filter/no-
fallback pair on the installed ABI 79 wheel.

- [x] Pin both official rows and their exact async/sync source hashes in
  `page-har-url-filter-manifest.json`, SHA-256
  `ad9293c598db398137c56ccb9405311db2ef4fff9bfe13cecd61a260f58067fe`.
- [x] Prove matching traffic is fulfilled by the Rust-owned HAR route while
  nonmatching traffic reaches the subsequently registered ordinary Page route.
- [x] Pass final installed candidate/oracle 2/2 and API-separated candidate
  stress async 10/10 plus sync 10/10 without changing code, ABI, feature
  closure or wheel artifacts.
- [x] Record candidate, oracle, async-stress and sync-stress report SHA-256
  values
  `eca2cd0c052686b795e94d5dbdd59a17468e0b952a33dd073a39745ffeba1e94`,
  `807bb7adf649c77448c0963b6f4542c22a674371b341160beffb68a1a0dc23f0`,
  `f460365ce417c1e205cdb6435708fee906c4d2fb9f2afcbb6808d92d64840474`
  and `7ee65e04701576ef9b4d90058bcf8fadb364ec4f46a849e75b06684963b43c19`.
- [x] Advance both rows from `not-run` to `passed`: 771 passed, zero failed,
  seven skipped and 5,748 not run, ledger SHA-256
  `fb11fa9a34ed364ec06254672ac32ef54a7a790362eeaa4f6a562887c2d1f03e`.
- [ ] Continue Context HAR, unmatched abort/fallback, external/ZIP/update
  capability families, then the remaining Python and complete Core JS/TS
  corpora.

NODE-324: Promote the complete unchanged async/sync Page HAR method/redirect
pair on the installed ABI 79 wheel.

- [x] Pin both official rows and their exact async/sync source hashes in
  `page-har-replay-manifest.json`, SHA-256
  `51e666d86132e4d88790025a98dff4ddb114eec22eb3df71e51bf2a23afd7f33`.
- [x] Pass final installed candidate/oracle 2/2 without changing code, ABI,
  feature closure or wheel artifacts.
- [x] Split repeat stress by public API to avoid pytest-asyncio/sync-bridge
  session-loop interference; pass async 10/10 and sync 10/10. Keep the mixed
  11-pass/9-error harness report diagnostic-only and out of the ledger.
- [x] Record candidate, oracle, async-stress and sync-stress report SHA-256
  values
  `2340bfc952cf7c2ec843e2e6166372a32d531e46587f8c080b04aa1235ec9958`,
  `950219a444ee63edccce9d63cc8fade9323dce703b9c05bf97b43c6162437654`,
  `17bea22d89ca4dc5cb862ba3820b8ad39dd42aa5bcb43684d6ebcc4b6f86f964`
  and `ad8c1a66288ec6d2cde1dcdc7ba7e42cac4d5f5c9983ee48d895fa90405004ec`.
- [x] Advance both rows from `not-run` to `passed`: 769 passed, zero failed,
  seven skipped and 5,750 not run, ledger SHA-256
  `dbc28e3a20089d56d2b937d7175f19e4562db76da59477b789e8c14fc375cefd`.
- [ ] Continue Context HAR, URL-filter/not-found policies, external/ZIP/update
  capability families, then the remaining Python and complete Core JS/TS
  corpora.

NODE-323: Advance Python to ABI 79 with composable persistent-Context plain-HAR
replay and Rust-owned computed-style assertion polling.

- [x] Pin the unchanged Playwright Python 1.58.0 Chromium
  `test_should_support_har_option` row in
  `persistent-context-har-manifest.json`, SHA-256
  `a3e57ce38826a608a730a6e382c60e65967cc4b81891aa6cf5118cc0790adf08`.
- [x] Add a backend-neutral `pwright-har-replay` crate and prove the selected
  host dependency delta over `page-routes` is exactly that policy crate.
- [x] Bound archive/file/entry/header/POST/response/redirect storage; compose
  internal HAR decisions with the existing Fetch event driver and reclaim the
  exact retained bytes on unregister/Page close.
- [x] Keep the Python facade thin: path, string URL filter and `not_found`
  policy only. Reject ZIP/external-content/update/Context-routing capability
  gaps explicitly instead of fabricating compatibility.
- [x] Add async/sync `to_have_css` through a Rust-selected computed-style
  program with Rust-owned regex validation, retry and deadline semantics.
- [x] Pass 1,001 all-feature host tests with 26 explicit ignores, 4 HAR crate
  tests, 2 addon tests, 167 installed-wheel units, strict Clippy, Ruff,
  Pyright and the 108-package dependency proof.
- [x] Pass final installed candidate/oracle 1/1 and candidate stress 10/10,
  then recollect 6,526/6,526. Candidate, oracle, stress, installed-unit and
  collection SHA-256 values are respectively
  `74bc73ec033aa2bdd914f94d752846615ea2a4b0801f8e8c9e3ef390fcdafc31`,
  `b48a9c6aa3325a2663fe987ae118058586b0f917c27babf9ba0cc2609893cc02`,
  `300abda8e41433e285970a456c32acd412688f194af3d57e8e77abda36861f5d`,
  `3c33650324d96ed74eaf5562bf795c15fbc5c6bad9e3117d817b43b9806277df`
  and `a3619b07314bb55898cbd6110bb25eb4af3b746c95216f36fe1371dfc9059799`.
- [x] Build two normalized-payload-reproducible wheels, SHA-256
  `c1954548f523af59abc861656595738e8b6ae82c8b02b8f2deaa4e2de6616473`
  and `d0bcdc98b69ddaf9fdcf2d5df482e32fcd2801b9ec43b28f8aa314dae72ebde8`;
  their byte-identical native extension SHA-256 is
  `243f6a6771e86e4d8bce4b7bd96293ea37c558fd2ef79344dfd76b683b3cd2c3`.
- [x] Advance the selected row from `not-run` to `passed`: 767 passed, zero
  failed, seven skipped and 5,752 not run, ledger SHA-256
  `e390e350e8995a48569133ed5be757cd9040a6d1592ab1b86f0ad5ca046476e2`.
- [ ] Continue the two recorded cookie applicability gaps, broader HAR
  capability/corpus coverage, then the remaining Python and complete Core
  JS/TS corpora.

NODE-322: Advance Python to ABI 78 with composable HTTP credentials and
navigation response-body retrieval.

- [x] Pin eleven unchanged Playwright Python 1.58.0 Chromium authentication,
  origin, persistent-Context and popup rows in
  `context-http-credentials-manifest.json`, SHA-256
  `154cd2e468af69fd4a8365e46a8959e3fb84f933626dff388c4e64970a241150`.
- [x] Add independent ordinary and persistent-default Context credentials,
  exact-origin matching and selected-session Fetch-authentication settlement.
  Compose it with Page routes through one shared reclaiming cursor.
- [x] Bound credentials to 16 Contexts, each field to 65,536 UTF-8 bytes and
  actions/attempts/request mappings/initializing sessions to 64 entries each.
  Omit username/password for `Default` and repeated-challenge `CancelAuth`.
- [x] Add a separate `response-body` leaf over frame navigation: eight
  operations, 4 MiB encoded/decoded output, authoritative session routing and
  charged PyO3 copying without a Python body cache.
- [x] Pass 999 all-feature host-library tests with 26 explicit ignores, 22
  non-live integration tests, 2 addon tests, 165 installed-wheel units, strict
  Clippy, Ruff, Pyright and the 108-package dependency proof.
- [x] Build two normalized-payload-reproducible wheels, SHA-256
  `473db3f3170c2124418f5bbd17dae19d8c5cc4f933938f1887233ce862c968b4`
  and `68ee94ea983be2ed9794b21fbba030299dcc8a3e00c4eac320099f23d1dc98aa`;
  their byte-identical native extension SHA-256 is
  `f5b18b328da7c6d6a21e2335151da6289eec167fd9a0652a7818579a53662eca`.
- [x] Pass final installed candidate/oracle 11/11 and candidate stress
  110/110, then recollect 6,526/6,526. Candidate, oracle, stress,
  installed-unit and collection SHA-256 values are respectively
  `cb7809748be7cc915f6125e973c1149b252b11fcf255c7b528489d7ba93a2de2`,
  `7652ef246c7f7007159d663a640e1fa9cc14591465de8a34f2961a035d6b3cc0`,
  `ffe3600bd95491851f7a623388780fb744a51c5fdae49322ab43f44040cf1004`,
  `023b372456d7759abf411a19d06a0352c6a961ef8a313aacde3da503e6e4cf8b`
  and `ec96b3f4af79891a2b2049537ac2650702e83d41ee7ca5539bbb06ce4037ccdb`.
- [x] Advance all eleven rows from `not-run` to `passed`: 766 passed, zero
  failed, seven skipped and 5,753 not run, ledger SHA-256
  `294ab4a8fee1ea783d5f5b8043a54c52648fe5e9539fdecbd7c42c6000b9c2fa`.
- [x] Continue the HAR row in NODE-323.
- [ ] Continue the two recorded cookie applicability gaps, broader HAR
  coverage, then the remaining Python and complete Core JS/TS corpora.

NODE-321: Advance Python to ABI 76 with composable Context geolocation,
permission and persistent launch arguments.

- [x] Pin the unchanged Playwright Python 1.58.0 Chromium
  `test_should_support_geolocation_and_permission_option` row in
  `context-geolocation-permission-manifest.json`, SHA-256
  `60d3b32cabfcfc13b2d951f891da0321e4d590c06956f40262bc13ce2b35e10e`.
  Preserve upstream commit `47a5d35ef4f815a2021349f86ae391f7c20c02d6` and source
  SHA-256 `61dc8a12d0346c7ad07d11dfb1e351582d82e0a71665964c100968478e8c2ac0`.
- [x] Add independent ordinary and persistent-default geolocation and
  geolocation-permission leaves. Keep owned/default/external Context matching
  exact; initialize selected Page/OOPIF sessions with
  `Emulation.setGeolocationOverride` and grant permission through the browser
  session with the exact native Context ID when one exists.
- [x] Bound both Rust registries to 16 selected Contexts, validate finite
  Playwright coordinate ranges, release ordinary Context slots idempotently,
  and retain no Python policy or session mirror.
- [x] Add `persistent-context-launch-arguments` as a separate provider-input
  leaf: at most 256 switches, 65,536 UTF-8 bytes each and 1,048,576 bytes in
  aggregate, with page-opening, profile and remote-debugging ownership
  rejected before launch.
- [x] Preserve the Tailscale-only fixture rule while matching upstream's
  localhost secure-origin assumption: the conformance runner can explicitly
  add only the exact dynamic HTTP fixture origin to Chromium's secure-origin
  allowlist for this class of upstream test, in candidate and oracle lanes.
- [x] Pass nine focused Rust owner/payload tests, 992 all-feature host-library
  tests with 26 explicit ignores, 22 non-live integration tests, 2 addon
  tests, 161 installed-wheel units, strict Clippy, Ruff, Pyright and the
  108-package dependency proof.
- [x] Build two normalized-payload-reproducible wheels, SHA-256
  `8a56c3f671f288aa7d461f20d1eed9097425531b7772e9f6719fed1ab099fc81`
  and `7d8a754afea1a822684c1ffc6daad029fe1f9335a10e21ee6f099502992096aa`;
  their byte-identical native extension SHA-256 is
  `f4808c9bd38bce42cb670c752d6edea3c39fc19328a81213782648a206758cc1`.
  Keep the Homebrew-OpenSSL Mac-local artifact warning explicit.
- [x] Pass final installed candidate/oracle 1/1 and candidate stress 10/10,
  then recollect the complete 6,526/6,526 inventory. Candidate, oracle, stress,
  installed-unit and collection report SHA-256 values are respectively
  `1a4b619753b5fc589702c5f138cb6f6d19a51ef20e022f3e39c7568fa1b53c39`,
  `428ac84f06a7b6f89e8a0aa065c076e31dca56f07545c09c6bf46dfd7d921f47`,
  `0397db4f1e7adde6c912cc89cb2bb93d4b1e39d0c67572ae8e14a02a8b4d451a`,
  `a6da39178d8fc49d105b87529539098a9500811b85b8116885f9f91e62101882`
  and `31366226f4778d8d7a75be3051559b01e0b0b9754b6cd214991e904a629fcffb`.
- [x] Advance the cumulative ledger only for that row: 755 passed, zero
  failed, seven skipped and 5,764 not run, SHA-256
  `f1288cda0a0873a31ca5a5915a661605f57a42cd316df68f0bd8fc51c155fb7f`,
  with no losses.
- [ ] Continue the four original default-Context rows: two recorded
  MagicDNS/`localhost` cookie applicability gaps plus HAR and HTTP-credentials
  product capabilities. Then execute the remaining Python and complete Core
  JS/TS corpora.

NODE-320: Advance Python to ABI 75 with composable Context offline policy.

- [x] Pin the unchanged Playwright Python 1.58.0 Chromium offline row in
  `context-offline-manifest.json`, SHA-256
  `00268111cd781614c53806e3f90903151be78e1db662680f547189c6996b065f`.
  Preserve upstream commit `47a5d35ef4f815a2021349f86ae391f7c20c02d6` and source
  SHA-256 `61dc8a12d0346c7ad07d11dfb1e351582d82e0a71665964c100968478e8c2ac0`.
- [x] Add independent `context-offline` and
  `persistent-context-offline` composition. Keep owned/default/external
  Context selection exact and initialize every selected Page/OOPIF session
  with Playwright Chromium's exact `Network.emulateNetworkConditions` offline
  payload before publication.
- [x] Bound the Rust registry to 16 selected Contexts, count the offline
  persistent default as one slot and release creation/close paths
  idempotently without a Python policy mirror.
- [x] Keep the persistent launch FFI tuple composable by grouping locale,
  timezone, HTTPS, offline, CSP and JavaScript flags into one policy value;
  align the native type stub with that runtime shape.
- [x] Pass five focused offline-owner tests, 983 all-feature host tests with
  26 explicit ignores, 2 addon tests, 159 installed-wheel units, strict
  Clippy, Ruff, Pyright and the 108-package dependency proof.
- [x] Build two normalized-payload-reproducible wheels, SHA-256
  `cdec69d543da109a3071b0b0b52d3e5c35d8344ddcbbdcee2a5931fde18538f9`
  and `2be2930d675c736375cebb3506228792c5eac0065c10bde7f5956311e8ad32a4`;
  their byte-identical native extension SHA-256 is
  `e73bb6233334972f3e58d38370672af658a0c791517862ecb3e50e7c87a123f4`.
  Keep the Homebrew-OpenSSL Mac-local artifact warning explicit.
- [x] Pass final installed candidate/oracle 1/1 and candidate stress 10/10,
  then recollect the complete 6,526/6,526 inventory. Candidate, oracle, stress,
  installed-unit and collection report SHA-256 values are respectively
  `27570a9e7750228f4b60c287573b38d015e0daa944ba6bb1624b7bb0de08cf99`,
  `f8e99a025c088eba2042729572fb97ae047be0c6862e528cfdf4aa4b115c7527`,
  `61bce407c655347e3308e6c5384a8f4c2edb48248562181a9a26c2daced268de`,
  `df4b303b88ec2458834baf4c54489c4773da2e871db87ac7e30a60a41d522bad`
  and `d925042c8bcdb55a7592543f2bd0f7814c592902eafa634e2d719f1bb9869b1c`.
- [x] Advance the cumulative ledger only for that row: 754 passed, zero
  failed, seven skipped and 5,765 not run, SHA-256
  `c38eea63e7096c771a8faa0912c0b1499158c6a11489dfb4848ac75cc4079592`,
  with no losses.
- [ ] Continue the five original default-Context rows: two recorded
  MagicDNS/`localhost` cookie applicability gaps plus geolocation/permission,
  HAR and HTTP-credentials product capabilities. Then execute the remaining
  Python and complete Core JS/TS corpora.

NODE-319: Advance Python to ABI 74 with composable Context timezone policy.

- [x] Pin the unchanged Playwright Python 1.58.0 Chromium timezone row in
  `context-timezone-manifest.json`, SHA-256
  `68c4d90a61e471f7d904c27b1831bbc65280f80fd91c645c6172fd9c00fa25dc`.
  Preserve upstream commit `47a5d35ef4f815a2021349f86ae391f7c20c02d6` and source
  SHA-256 `61dc8a12d0346c7ad07d11dfb1e351582d82e0a71665964c100968478e8c2ac0`.
- [x] Add independent `context-timezone` and
  `persistent-context-timezone` composition. Keep owned/default/external
  Context selection exact and initialize every selected Page/OOPIF session
  with `Emulation.setTimezoneOverride` before publication.
- [x] Bound the Rust registry to 16 selected Contexts and each identifier to
  256 UTF-8 bytes; reject empty/control input, map invalid browser timezone
  identifiers to stable argument errors and release creation/close paths
  idempotently without a Python policy mirror.
- [x] Replace the growing positional Context-policy host call with composable
  `ContextPolicyOptions`, while leaving each feature independently selectable.
- [x] Pass four focused timezone-owner tests, 978 all-feature host tests with
  26 explicit ignores, 2 addon tests, 157 installed-wheel units, strict
  Clippy, Ruff, Pyright and the 108-package dependency proof.
- [x] Build two normalized-payload-reproducible wheels, SHA-256
  `a8b4ae9a06dde9cf9d7f766cb7d326ca3b0e69d773c23b9fdbb14d8c19519632`
  and `377d5981d1d76bf121bf0db45222a0b82e9610bb9f6e9b638f075a67f452f49f`;
  their byte-identical native extension SHA-256 is
  `8b1dd78a036c331ae027e2c1cbb49caa51008ead5b9d4e022c729607f8ba6d02`.
  Keep the Homebrew-OpenSSL Mac-local artifact warning explicit.
- [x] Pass final installed candidate/oracle 1/1 and candidate stress 10/10,
  then recollect the complete 6,526/6,526 inventory. Candidate, oracle, stress,
  installed-unit and collection report SHA-256 values are respectively
  `25b4741916039a3841c6e0416875befd52b46c06eafae057eb4fdcd6b4707e39`,
  `fec89b423b6582adf9e7754e980500e4b6266c17c722e0043eeaf0e0c4093c73`,
  `1a3674e68420813a531778093500877aac49868cea700c60d9592ae69a716c9d`,
  `1ea73cebd5b5a44578cd2af4816d8697e5f6381461bc55884cfdaa7304568d9f`
  and `46ef609d96e1cdd9be58f41f7d3878f2d4b6da63928cbcd03d55e04895b54420`.
- [x] Advance the cumulative ledger only for that row: 753 passed, zero
  failed, seven skipped and 5,766 not run, SHA-256
  `a48b4d1c0e8bdce904b95194393ee47709c01313c121843e81e1410f524b27fc`,
  with no losses.
- [ ] Continue the six original default-Context rows: two recorded
  MagicDNS/`localhost` cookie applicability gaps plus geolocation/permission,
  HAR, HTTP credentials and offline product capabilities. Then execute the
  remaining Python and complete Core JS/TS corpora.

NODE-318: Advance Python to ABI 73 with composable JavaScript-disabled
Contexts and the prerequisites required by the pinned click row.

- [x] Pin four unchanged Playwright Python 1.58.0 Chromium rows in
  `context-javascript-disabled-manifest.json`, SHA-256
  `7f2e326e8c7d5ac9623279fac00277468ee9ae25394f11a10b6ac7d8d90641c4`.
- [x] Add independent `javascript-disabled` and persistent-default
  composition. Keep owned/default/external Context selection exact and apply
  `Emulation.setScriptExecutionDisabled {"value": true}` to each selected
  Page/OOPIF session before publication.
- [x] Bound the policy registry to 16 selected Contexts, count the disabled
  persistent default as one slot, and release creation/close paths
  idempotently without a Python policy mirror.
- [x] Add thin async/sync `Page.expect_navigation` over the existing bounded
  Rust frame-navigation owner. Arm before the user action and keep URL
  predicates explicitly unsupported until installed.
- [x] Select the first visible clipped client-rect fragment for default Locator
  click positions, preserving explicit-position border-box semantics; accept
  main-world console replay before paired-world readiness without aborting
  topology initialization.
- [x] Pass four focused JavaScript-policy tests, 974 all-feature host tests
  with 26 explicit ignores, 2 addon tests, 157 installed-wheel units, strict
  Clippy, Ruff, Pyright and the 108-package dependency proof.
- [x] Build two normalized-payload-reproducible wheels, SHA-256
  `7c2e48a4705d597aba896763e50b67ddfd6d8d03c562312c5dcd3f24ba687859`
  and `89b60d08b5e5691aeb4de049a2d10a74f0a42947f42fdf57c3693d0eabd1ab9a`;
  their byte-identical native extension SHA-256 is
  `26e341524d4f6bf5703089a1db57343cc885e8c243ed2bfc6448b72dfd9a41a0`.
  Keep the Homebrew-OpenSSL Mac-local artifact warning explicit.
- [x] Pass final installed candidate/oracle 4/4 and candidate stress 40/40,
  then recollect the complete 6,526/6,526 inventory. Candidate, oracle, stress,
  installed-unit and collection report SHA-256 values are respectively
  `991bbd1aa335772d0b59d33cd13e0afe4a980a431f8f72886473cf4261a7affe`,
  `1068243f7512a4b54849b6cfb8ac0c733a197c41bb48db8d4a1c87794f96e286`,
  `1ca6b2b46ad43527177358dc66947350d1cef8307a17630933039a0f0f8c40ce`,
  `b3ed8cb9a09f9945236a3949e0b5bdc255f0790ab1471c414f105fdfbcd16960`
  and `f440d9f4e46739f28b5484f5c5f0bcd427fe2e3dee37be575f989f64d82fade6`.
- [x] Advance the cumulative ledger from 748 to 752 passed, zero failed, seven
  skipped and 5,767 not run, SHA-256
  `93f22527ff4e8f89d95a525a17393d3d5d3463432e6e9e75f7135adcea7a81a3`,
  with no losses.
- [ ] Continue the remaining seven Chromium rows from the original default-
  BrowserContext diagnostic, including two MagicDNS fixture gaps, then execute
  the remaining Python and complete Core JS/TS corpora.

NODE-317: Advance Python to ABI 72 with composable Context CSP bypass.

- [x] Pin four unchanged Playwright Python 1.58.0 Chromium rows in
  `context-bypass-csp-manifest.json`, SHA-256
  `6748a04a6e0e1f0340d7e7c6e14ef6ba5ddd4e0e91361dd3fe2feb9f57fc4c64`.
- [x] Add independent `bypass-csp` and persistent-default composition. Keep
  exact owned/default/external Context selection and initialize every selected
  Page/OOPIF session with `Page.setBypassCSP` before publication.
- [x] Bound the shared Rust policy registry to 16 Contexts, count the selected
  persistent default as one slot, and release creation/close paths
  idempotently without a Python policy mirror.
- [x] Pass four focused CSP-owner tests, 970 all-feature host tests with 26
  explicit ignores, 2 addon tests, 152 installed-wheel units, strict Clippy,
  Ruff, Pyright and the 108-package dependency proof.
- [x] Build two normalized-payload-reproducible wheels, SHA-256
  `44a9ec5ee2be17cab8036084dfa46c63fc717904692175e6fa66bafd8ef21f70`
  and `319a9d9c11cf43d55a1b34eb498b9e24dcc6048a5e4ca43931a880a76c5ca7eb`;
  their byte-identical native extension SHA-256 is
  `0a2ce2ad114c52ff3cb51328557386db291e922f6684ffa8b2d50133b8fe721d`.
  Keep the Homebrew-OpenSSL Mac-local artifact warning explicit.
- [x] Pass final installed candidate/oracle 4/4 and candidate stress 40/40,
  then recollect the complete 6,526/6,526 inventory.
- [x] Advance the cumulative ledger from 744 to 748 passed, zero failed, seven
  skipped and 5,771 not run, SHA-256
  `713d4047df50050430e4bfec6c843b171806653db19e8c0df6da6e66cd942508`,
  with no losses.
- [ ] Add the separately blocked iframe CSP row only after the independent
  `ElementHandle.content_frame` capability exists; do not count it now.
- [ ] Continue the remaining eight Chromium rows from the original default-
  BrowserContext diagnostic, including two MagicDNS fixture gaps, then execute
  the remaining Python and complete Core JS/TS corpora.

NODE-316: Advance Python to ABI 71 with persistent-Context extra HTTP headers.

- [x] Pin the unchanged Playwright Python 1.58.0 Chromium row in
  `persistent-context-extra-http-headers-manifest.json`, SHA-256
  `5345d32db44ce2f21a3c8ebc6fec811a498c695f68972b66564a39677d2d1041`.
- [x] Compose `persistent-context-extra-headers` from persistent launch and
  the existing header owner. Admit one immutable bounded default before
  initial topology, select only the persistent default Context, and keep later
  Context/Page update, compensation and retirement semantics in the same
  `HeaderSettings` owner.
- [x] Add thin async/sync Python launch forwarding through ABI 71; validate at
  most 256 string pairs / 64 KiB before native work and retain a fixed 512 KiB
  native-call allowance.
- [x] Pass 12 focused header-owner tests, 966 all-feature host tests with 26
  explicit ignores, 2 addon tests, 150 installed-wheel units, strict Clippy,
  Ruff, Pyright and the 108-package dependency proof.
- [x] Build two normalized-payload-reproducible wheels, SHA-256
  `f954e83ec2343be3b52fe4ad0f8a3393c343211f4779bea8f6a3d5a7857ec7e2`
  and `1f0a64b3d822ca9b018b7ba64c94d8852f53aaf0de6069f33ab4ef1bb2960ebc`;
  their byte-identical native extension SHA-256 is
  `a5b9fcb1d52d72986a017de082b0ee342fa57b5e64b413e52827c6d0c22392d2`.
  Keep the unvendored-OpenSSL warning explicit before distribution.
- [x] Pass final installed candidate/oracle 1/1 and candidate stress 10/10,
  then recollect the complete 6,526/6,526 inventory.
- [x] Advance the cumulative ledger from 743 to 744 passed, zero failed, seven
  skipped and 5,775 not run, SHA-256
  `19d1e6e8365cf3c56b86e96edc76d10a6a619b797f967caedb477cacfbd69411`,
  with no losses.
- [ ] Continue the remaining nine Chromium rows from the original default-
  BrowserContext diagnostic, including the two recorded MagicDNS fixture gaps;
  then execute the remaining Python and complete Core JS/TS corpora.

NODE-315: Advance Python to ABI 70 with persistent-Context media defaults.

- [x] Pin the three unchanged Playwright Python 1.58.0 Chromium rows in
  `persistent-context-media-manifest.json`, SHA-256
  `a2da1fb06d4bd6039f7255d1f95e92f853ef5260b491f11fa1236a19f2c956f9`.
- [x] Compose `persistent-context-emulate-media` from the existing persistent
  launcher and media owner. Admit one immutable Rust default before initial
  Page topology, replay it for Page/OOPIF sessions in only the persistent
  default Context, and share the existing capacity-one mutation gate.
- [x] Add thin async/sync Python launch options and ABI 70 tuple forwarding;
  validate color scheme, reduced motion and contrast before native work.
- [x] Pass four focused media-owner tests, 964 all-feature host tests with 26
  explicit ignores, 2 addon tests, 149 installed-wheel units, strict Clippy,
  Ruff, Pyright and the 108-package dependency proof.
- [x] Build two normalized-payload-reproducible wheels, SHA-256
  `3b7ddb9ea476955dcf442a357846a9856ef1e625f304c435ac970685f1a737a8`
  and `e99fc988705d6daafce55e60aa930b49f7964aa3902fb200307b3111cd05f353`;
  their native extension SHA-256 is
  `6a44342c2790626304fa7483f5537d2f611223a22068bbb667149c48c5654c53`.
  Keep the unvendored-OpenSSL warning explicit before distribution.
- [x] Pass final installed candidate/oracle 3/3 and candidate stress 30/30,
  then recollect the complete 6,526/6,526 inventory.
- [x] Advance the cumulative ledger from 740 to 743 passed, zero failed, seven
  skipped and 5,776 not run, SHA-256
  `8a7fe5975b0bbbf792cc8c13a0b8e81a11a70bb8f17dcb0e8f439abe30e67efb`,
  with no losses.
- [ ] Continue the remaining 10 Chromium rows from the original default-
  BrowserContext diagnostic, including the two recorded MagicDNS fixture gaps;
  then execute the remaining Python and complete Core JS/TS corpora.

NODE-314: Keep Python ABI 69 and add persistent Context launch-argument
preflight without claiming arbitrary flag support.

- [x] Pin the unchanged async Chromium page-argument rejection row in a fixed
  one-case manifest, SHA-256
  `4af8748fab33f52ce7ee71742b94265a6655a8d7b12688b5a3b90a42ce32dd86`.
- [x] Add the public async/sync `args` shape and validate it before native
  launch: at most 256 strings, 64 KiB each and 1 MiB total.
- [x] Reproduce Chromium Playwright's pre-launch errors for user-data-dir,
  remote-debugging-pipe and bare page-opening arguments. Preserve empty args as
  a no-op and keep non-empty switch-only args explicitly unsupported.
- [x] Pass 148 installed-wheel units, including async and sync preflight
  regressions, plus Ruff and strict Pyright without changing
  ABI 69, the native extension, Rust owners or the dependency closure.
- [x] Build two normalized-payload-reproducible wheels, SHA-256
  `d17207c54c374c072df4d752df38959ecd376c5918dcc215fe3b46a22cb93b07`
  and `76bfdf8d21cb28d268139106d225cf1d989ae53e629eb0805bfd9278efc0b052`;
  native SHA-256 remains
  `d2332cab6551ab9c189acd77678bf7ba43d20c167e5bc4d9c27cebcdd71b430f`.
- [x] Pass the unchanged official row candidate/oracle 1/1 and candidate
  stress 10/10, then recollect 6,526/6,526 rows.
- [x] Advance the cumulative ledger from 739 to 740 passed, zero failed, seven
  skipped and 5,779 not run, with no losses.
- [x] Restore Python 3.9 compatibility in the cumulative summarizer by
  removing a redundant `zip(..., strict=True)` after its explicit equal-length
  check; pass strict Pyright and reproduce the exact ledger hash with system
  Python 3.9.6.
- [ ] Superseded by NODE-315: continue the remaining 10 Chromium rows from the
  original default-BrowserContext diagnostic, including the two recorded
  MagicDNS fixture gaps;
  then continue the rest of Python and the complete Core JS/TS corpus.

NODE-313: Advance Python to ABI 69 with independently selectable
BrowserContext cookies.

- [x] Pin four unchanged Playwright Python 1.58.0 Chromium rows and their
  source hash in `context-cookies-manifest.json`; preserve all four outcomes.
- [x] Add browser-root CDP Storage get/set/clear commands and route the exact
  default or owned Context ID without requiring a Page session or cookie
  mirror.
- [x] Implement Rust-owned Playwright URL/domain/path validation and rewriting,
  URL filtering, clear matchers, expiry and partition identity. Preserve the
  internal `hasCrossSiteAncestor` bit during filtered clear and Playwright's
  empty exact-filter and empty-URL truthiness semantics.
- [x] Add thin async/sync Python `cookies`, `add_cookies` and `clear_cookies`
  facades, ABI 69 native methods and typed public cookie shapes. Bound values
  before FFI copying and keep Chromium as the only cookie source of truth.
- [x] Enforce 1,024 cookies, 16 KiB ordinary fields, 4 MiB cookie batches and
  results, 256 URLs, 64 KiB per URL, 1 MiB URL batches, 16 KiB regex sources
  and a 10 MiB FFI allowance.
- [x] Pass 130 CDP tests, 962 all-feature host tests with 26 explicit ignores,
  2 addon tests, 146 installed-wheel units, strict release Clippy, Ruff,
  Pyright and the unchanged 108-package dependency proof.
- [x] Build two normalized-payload-reproducible ABI3 wheels. Archive SHA-256
  values are
  `174a30499a3c21c746d4ed5ffb5461247afe012bc9207c32834c68c0801382d4`
  and `faa8f553d1643ed4594997acf1fd06dd75f730c3981cbf99fe12d3dda0d69c58`;
  native SHA-256 is
  `d2332cab6551ab9c189acd77678bf7ba43d20c167e5bc4d9c27cebcdd71b430f`.
  Keep the unvendored-OpenSSL warning explicit before distribution.
- [x] Establish exact candidate/oracle parity on the four fixed rows. Count the
  two fixture-independent passes; retain the two hardcoded-`localhost` rows as
  MagicDNS provider-applicability gaps rather than product results. Repeat ten
  times in one process: 20 pass and the same 20 hostname-only outcomes.
- [x] Recollect 6,526/6,526 rows and advance the cumulative ledger from 737 to
  739 passed, zero failed, seven skipped and 5,780 not run, with no losses.
- [ ] Continue the remaining 14 Chromium rows from the original default-
  BrowserContext diagnostic, including resolving the two recorded fixture
  applicability gaps without weakening upstream assertions; then continue the
  rest of Python and the complete Core JS/TS corpus.

NODE-312: Keep Python ABI 68 and add the thin BrowserContext event facade.

- [x] Reproduce the unchanged official async Chromium persistent-Context close
  row. The installed ABI 68 candidate reaches browser behavior but fails
  because `BrowserContext.on` is absent; the independent Playwright 1.58.0
  oracle passes 1/1. Keep the red candidate report diagnostic-only.
- [x] Add async/sync `BrowserContext.on`, `once` and `remove_listener` for the
  existing Rust-published `page` and `close` lifecycle events and optional
  `console` events. Add no polling, lifecycle watcher, native export or ABI
  change.
- [x] Preserve exact public callback identity in the sync adapter and wrap
  async Page, ConsoleMessage and closing Context values into their existing
  sync facade identities. Route sync handler exceptions through the existing
  bridge error owner.
- [x] Hold exactly one existing bounded Rust Context-console subscription while
  the first Python console listener is installed and release it when the last
  one is removed. Add no new queue, task, timer, transport, session or resource
  allowance.
- [x] Pass all 141 installed-wheel units, Ruff, strict Pyright, formatting and
  the 108-package dependency proof.
- [x] Build two final ABI3 wheels and verify normalized reproducibility. Their
  SHA-256 values are
  `2487ed3dbbac1e4fa7047e2cfbf7b3c6e5f0978b94eb972db27b9f6f60a4acf3`
  and `21d9631c7e361584cad656a7eb5332d9332cdd86db7fb1314c08fae8f26e778b`;
  the byte-identical native extension SHA-256 is
  `890e6b4d7023d0ee13ff1a4bfa1b823416297a5ad273dbc5c1d84c2ceabc3f17`.
- [x] Pass the unchanged official row 1/1 in the final installed candidate and
  independent oracle, then repeat the candidate 10/10 in one process.
- [x] Collect all 6,526 upstream Python rows and advance the ledger from 736 to
  737 passed, zero failed, seven skipped and 5,782 not run. Exact comparison
  changes only that Chromium row from not run to passed and has no losses.
- [ ] Continue the remaining 16 Chromium rows from the original default-
  BrowserContext diagnostic, the rest of Python and the complete Core JS/TS
  corpus.

NODE-311: Advance Python to ABI 68 with independently selectable persistent-
Context device emulation.

- [x] Reproduce five unchanged official async Chromium default-Context rows.
  The ABI 67 candidate rejects the persistent device options; the independent
  Playwright 1.58.0 oracle passes 5/5. Keep the red candidate report and the
  first missing-browser oracle attempt diagnostic-only.
- [x] Add `persistent-context-device-emulation` over `persistent-context` plus
  `device-emulation`. Admit the default device policy before initial Page
  topology initialization and prove the leaf adds no Pwright package beyond
  the persistent-launch closure.
- [x] Reuse the existing Rust device installation, user-agent composition and
  Page/OOPIF initializer. Select the persistent default only for
  `CdpContextOwnership::Default`; later owned Contexts use their registry
  entries and attached external Contexts inherit nothing.
- [x] Count the default against the existing 16-Context device registry and
  byte budget. Add no queue, task, timer, transport, session registry,
  browser-process allowance or dependency.
- [x] Advance the package/native handshake to ABI 68. Extend the persistent-
  launch tuple to `(base_url, locale, ignore_https_errors, device,
  executable_path, channel, timeout_ms)` and expose async/sync viewport,
  screen, user-agent, scale, mobile and touch options.
- [x] Add Playwright-compatible async/sync `Page.viewport_size` snapshots.
  Python retains only the public tuple while Rust remains authoritative for
  browser emulation and Page-level viewport mutation.
- [x] Pass 954 all-feature host tests with 26 explicit live/diagnostic
  ignores, 2/2 addon tests, all 137 installed-wheel units, formatting,
  selected/all-feature release Clippy, the 108-package dependency proof, Ruff
  and strict Pyright.
- [x] Build two final ABI3 wheels and verify normalized reproducibility. Their
  SHA-256 values are
  `a1ca0c2bb6d80a0e9ac6fadfa3e778077acc3f42cd6ef9d3e0a55101ba798059`
  and `654386396048318f4fb8fdfbf6839da2b8c7ee69be19b74a921569370a30987f`;
  the byte-identical native extension SHA-256 is
  `c4d78c799f17dc1955514593fe8132173bf48c273d14a1a2ac4a74cd36703fe0`.
- [x] Pass the five unchanged official rows 5/5 in final installed candidate
  and independent oracle, then repeat the candidate 50/50 in one process.
- [x] Collect all 6,526 upstream Python rows and advance the ledger from 731
  to 736 passed, zero failed, seven skipped and 5,783 not run. Exact comparison
  changes only those five Chromium rows from not run to passed and has no
  losses.
- [ ] Continue the remaining 17 Chromium rows from the original default-
  BrowserContext diagnostic, the rest of Python and the complete Core JS/TS
  corpus.

NODE-310: Advance Python to ABI 67 with independently selectable persistent-
Context HTTPS-error bypass.

- [x] Reproduce the unchanged official async Chromium row against the ABI 66
  installed candidate: `ignore_https_errors` is rejected before persistent
  launch. Keep that red report diagnostic-only; the independent Playwright
  1.58.0 oracle passes 1/1.
- [x] Add `persistent-context-ignore-https-errors` over `persistent-context`
  plus `ignore-https-errors`. Admit the default-Context policy before initial
  Page topology initialization and prove the leaf adds no Pwright package
  beyond the persistent-launch closure.
- [x] Reuse the existing Rust installation and Chromium
  `Security.setIgnoreCertificateErrors` initializer. Select the persistent
  default only for `CdpContextOwnership::Default`; do not affect attached
  external Contexts or unselected later owned Contexts.
- [x] Count the selected default against the existing fixed 16-Context HTTPS
  policy capacity, leaving at most 15 later selected owned Contexts. Add no
  queue, task, session registry, timer, transport, process allowance or
  dependency.
- [x] Advance the package/native handshake to ABI 67. Extend only the
  persistent-launch tuple to `(base_url, locale, ignore_https_errors,
  executable_path, channel, timeout_ms)` and the async/sync
  `BrowserType.launch_persistent_context(ignore_https_errors=...)` facades.
- [x] Pass four focused Rust policy regressions, 952 all-feature host tests
  with 26 explicit live/diagnostic ignores, 2/2 addon tests, all 136 installed-
  wheel units, formatting, selected/all-feature release Clippy, the 108-
  package dependency proof, Ruff and strict Pyright.
- [x] Build two final ABI3 wheels and verify normalized reproducibility. Their
  SHA-256 values are
  `c153464a16a2083899e44a3637d48d2cf512a9d669675feed27a244df2f65880`
  and `f78552d51c1ecc9c2cac8a8b8193896663c71213d487a57903c8d45c184f1387`;
  the byte-identical native extension SHA-256 is
  `7f32e973622c1b7e1edbd1465637bebbc94af840972bf5457ee13df500c265eb`.
- [x] Pass the unchanged official persistent HTTPS row 1/1 in final installed
  candidate and independent oracle, then repeat the candidate 10/10 in one
  process to prove policy and persistent-session capacity are reclaimed.
- [x] Collect all 6,526 upstream Python rows and advance the ledger from 730
  to 731 passed, zero failed, seven skipped and 5,788 not run. Exact comparison
  changes only that Chromium row from not run to passed and has no losses.
- [ ] Continue the remaining 22 Chromium rows from the original default-
  BrowserContext diagnostic, the rest of Python and the complete Core JS/TS
  corpus.

NODE-309: Release persistent Python Browser sessions when their default
`BrowserContext` closes.

- [x] Run all 27 official async Chromium default-BrowserContext rows in
  isolated diagnostic mode. Four pass and 23 retain explicit product
  discrepancies. Select only the three previously unrecorded persistent-
  profile baseline rows; the locale row is already NODE-307 evidence.
- [x] Reproduce the lifetime leak in one Python process. A ten-repeat selection
  performs five persistent launches per repeat; the unrepaired wheel reaches
  the fixed 16-session environment capacity during repeat four and finishes
  only 10/30 test invocations.
- [x] Mark a Browser created by `launch_persistent_context` without changing
  connected-Browser behavior. After normal default-Context retirement,
  `BrowserContext.close()` closes its unique native Browser session so Rust can
  remove that session from the existing environment owner.
- [x] Remove a successfully closed Browser from `Playwright._browsers`; retain
  no closed language Browser/Context graph. Sync behavior reuses the corrected
  async bridge. Add no task, queue, registry, native resource allowance or
  dependency.
- [x] Keep ABI 66: this is Python ownership routing over the existing native
  `close()` operation and adds no native export, feature or option.
- [x] Pass three new AAA ownership regressions, all 135 installed-wheel units,
  all-feature addon release Clippy, formatting, the 108-package dependency
  proof, Ruff and strict Pyright.
- [x] Build two final ABI3 wheels and verify normalized reproducibility. Their
  SHA-256 values are
  `1bdd9b424f21d52def2b82901df4b4069e8c605e946b4476aa8312f8519120ba`
  and `58cc07b09356c442130238faf876336164964e9d6ff17429dcdf8024b66ae8d0`;
  the unchanged byte-identical native extension SHA-256 is
  `45fc56e1138798ad239e06c066b16e6e6ed8c5eab2064e9a7a9e1e92cf975024`.
- [x] Pass the three unchanged official persistent-profile baseline rows 3/3
  in final installed candidate and independent oracle. Repeat them 10 times:
  30/30 testcase invocations and 50 persistent launches pass in one process,
  proving capacity is recycled beyond the limit of 16.
- [x] Collect all 6,526 upstream Python rows and advance the ledger from 727
  to 730 passed, zero failed, seven skipped and 5,789 not run. Exact comparison
  changes only those three rows from not run to passed and has no losses.
- [ ] Continue the remaining 23 Chromium rows in this file, the rest of the
  Python corpus and the complete Core JS/TS corpus.

NODE-308: Make `wait_for_function` timeout completion acknowledge the current
admitted predicate attempt before returning.

- [x] Reproduce the official Chromium `wait_for_function` file: the first
  candidate stress run passed 58/60 but twice observed one `console.log` after
  public timeout, while the independent Playwright 1.58.0 oracle passed 6/6.
- [x] Preserve the public deadline while canceling the Rust poll owner and
  awaiting only its currently admitted predicate attempt plus the ordinary
  command/publication fence. Do not start another poll after cancellation.
- [x] Dispose a Handle that becomes ready after the deadline, preserve
  stop/disconnect as authoritative `TargetClosed`, and add no task, queue,
  transport, registry, retained session or dependency.
- [x] Keep ABI 66: this repairs internal completion ordering and adds no native
  export, feature, option or Python-side lifecycle owner.
- [x] Pass 8/8 focused Rust regressions, 950 all-feature host tests with 26
  explicit live/diagnostic ignores, 2/2 addon tests, formatting, strict
  selected/all-feature release Clippy, the 108-package dependency proof, Ruff,
  strict Pyright and 132/132 installed-wheel Python units.
- [x] Build two final ABI3 wheels and verify normalized reproducibility. Their
  SHA-256 values are
  `752e68fd89b54c8dd4cb5a5552ff6c847cf46736fb4d9c70465e9c5b58e57320`
  and `a1827bb7227124bb298d94d3b885dc317d7dc16c61fce7aa9560246c5f0a0f86`;
  the byte-identical native extension SHA-256 is
  `45fc56e1138798ad239e06c066b16e6e6ed8c5eab2064e9a7a9e1e92cf975024`.
- [x] Pass all six unchanged official async Chromium rows in final installed
  candidate and independent oracle, then pass the final candidate repeat
  60/60. The initial 58/60 report remains diagnostic and is excluded from the
  ledger.
- [x] Collect all 6,526 upstream Python rows and advance the ledger from 721
  to 727 passed, zero failed, seven skipped and 5,792 not run. Exact comparison
  changes only those six rows from not run to passed and has no losses.
- [ ] Continue the remaining Python and complete Core JS/TS corpora.

NODE-307: Advance Python to ABI 66 with independently selectable persistent-
Context locale.

- [x] Add `persistent-context-locale` over `persistent-context` plus
  `context-locale`. Admit the default-Context locale before initial Page
  topology initialization and prove the leaf adds no Pwright package beyond
  the existing persistent-Context closure.
- [x] Match both upstream Chromium locale commands on every selected Page and
  OOPIF. One stateless composition boundary emits exactly one
  `Emulation.setUserAgentOverride`, preserving a selected device `userAgent`
  while adding `acceptLanguage`; the locale initializer then emits
  `Emulation.setLocaleOverride`. The first controls `navigator.language`; the
  second controls ICU/`Intl`. Suppress only Chromium's exact shared-renderer
  duplicate-locale error.
- [x] Count the persistent default locale against the existing 16-Context,
  256-byte-per-locale limit. Retain no session id, Worker copy, task, queue,
  timer or browser process beyond the selected persistent launch owner.
- [x] Advance the package/native handshake to ABI 66 and extend only the
  persistent-launch tuple plus async/sync
  `BrowserType.launch_persistent_context(locale=...)` forwarding.
- [x] Pass seven locale ownership/bounds/ICU-payload regressions, four shared
  user-agent composition regressions, 948 all-feature host tests with 26
  explicit live/diagnostic ignores, 2/2 addon tests, formatting, strict
  selected/all-feature release Clippy, the 108-package dependency proof, Ruff,
  strict Pyright and 132/132 installed-wheel units.
- [x] Build two final repaired ABI3 wheels and verify normalized
  reproducibility. Their SHA-256 values are
  `d05505434a588149a878a1d06657cbfbecd4ff9046ad6cf79787e26fd1c5f870`
  and `e3cbc6a377ce20492652baeb8f46ac49f79088dced638bcd8a8bddf40ebabb79`;
  the native extension SHA-256 is
  `f8e49c7d08b22a639fadc5f1a2b6e44aad1c3ce46de83f9f29763ac379105b3b`.
- [x] Pass the unchanged official Chromium persistent-locale row in the final
  installed candidate and independent oracle, repeat the candidate 10/10 and
  keep the Context-locale Worker regression green. The first provisional run
  returned `en-US` and correctly exposed the missing `acceptLanguage`
  command; it remains diagnostic evidence and is excluded from the ledger.
- [x] Pass an installed-wheel real-Chrome composition probe proving that
  `Browser.new_context(user_agent=..., locale=...)` preserves the exact device
  UA and reports the selected `navigator.language`; release the Context and
  attached session without leaking a tab.
- [x] Collect all 6,526 upstream Python rows and advance the ledger from 720
  to 721 passed, zero failed, seven skipped and 5,798 not run. Exact comparison
  changes only the selected persistent-locale Chromium row and has no losses.
- [ ] Continue the remaining Python and complete Core JS/TS corpora.

NODE-306: Advance Python to ABI 65 with independently selectable Context locale.

- [x] Add `context-locale` as a bounded owned-Context policy and prove that it
  adds no Pwright package beyond the existing base closure.
- [x] Retain at most 16 nonempty, control-free locale strings of 256 UTF-8
  bytes each. Apply `Emulation.setLocaleOverride` to each owned Page/OOPIF
  session and suppress only Chromium's exact shared-renderer duplicate-locale
  error; propagate every other failure.
- [x] Match upstream Chromium behavior: dedicated and nested Workers inherit
  the renderer locale, so locale adds no Worker command, registry, task, queue
  or feature and does not alter Worker attach ordering.
- [x] Advance the exact package/native handshake to ABI 65 and keep async/sync
  `Browser.new_context(locale=...)` as thin option forwarding.
- [x] Pass five locale ownership/bounds/error-classification/payload
  regressions, all-feature host/addon tests, formatting, strict selected/
  all-feature release Clippy, the 108-package dependency proof, Ruff, strict
  Pyright and 131/131 installed-wheel Python units on the corrected source.
  The host passes 942 tests with 26 explicit live/diagnostic ignores and the
  addon passes 2/2.
- [x] Build two corrected-source repaired ABI3 wheels and verify normalized
  reproducibility. Their SHA-256 values are
  `5748b3dd7310752b8bac8a7a39588b06e706f104f647cc1ac6be30aec3da24b1`
  and `7865ee31d2dbdebcabbf18b9e51dc9fd47dcf45e8968a9c5e1f3743785200abb`.
  Earlier provisional hashes are obsolete because they contained a
  non-upstream Worker command path.
- [x] Pass the unchanged official async Chromium Context-locale Worker row in
  candidate and oracle, repeat it 10/10, pass the complete Worker file 13/13
  and advance the 6,526-row ledger from 719 to 720 without losses. A real red
  candidate run exposed Chromium's shared-renderer duplicate override; it is
  diagnostic evidence and is not merged into the ledger.
- [x] Promote the final wheel after proving its complete runtime payload is
  byte-identical to the browser-tested corrected wheel; only wheel metadata,
  RECORD and CycloneDX identity changed with the final README. Fresh candidate
  collection is 6,526/6,526 and the ledger is 720 passed, zero failed, seven
  skipped and 5,799 not run, SHA-256
  `20e36211f8f1bc6027c7c6998aa7060cb9d43262eadcaf7eb60561bfde4a66f3`.
- [ ] Continue the
  remaining Python and complete Core JS/TS corpora.

NODE-305: Advance Python to ABI 64 with independently selectable Worker-console
argument Handles.

- [x] Add `worker-console-argument-handles` over `worker-console-events` plus
  `worker-handle-evaluation`; prove its selected Pwright package closure is
  exactly the existing Worker Handle closure.
- [x] Admit, validate and transfer every ordered console RemoteObject as one
  batch. On parsing, capacity, queue or close failure, release each native
  object exactly once without publishing a partial `ConsoleMessage.args`.
- [x] Reuse the global 64 Worker Handles, eight concurrent Handle calls and 64
  Handle cleanup reservations. Keep the console FIFO at 128 notices/16 MiB.
- [x] Extend Worker-created Handles with exact-context `json_value`; return a
  retained primitive without native I/O and evaluate object projections in the
  original Worker execution context.
- [x] Advance the package/native handshake to ABI 64. Keep Python thin and
  accept both 11-field legacy and 12-field argument-Handle notice descriptors.
- [x] Pass five new Rust ownership/bounds regressions, 937 all-feature host
  tests with 26 explicit live/diagnostic ignores, 2/2 addon tests, formatting,
  strict selected/all-feature release Clippy, the 108-package dependency
  proof, Ruff, strict Pyright and 131/131 installed-wheel Python units.
- [x] Build two repaired ABI3 wheels and verify normalized reproducibility;
  their payloads differ only in permitted CycloneDX identity.
- [x] Pass the unchanged official async Chromium Worker console-Handle row in
  installed candidate and independent oracle, plus 10/10 repeated candidate
  invocations. The complete unchanged Chromium Worker file is 12/13.
- [x] Update the 6,526-row ledger to 719 passed, zero failed, seven skipped and
  5,800 not run, with exactly one not-run-to-passed change and no losses.
- [ ] Add BrowserContext locale as an independent capability, then continue
  the remaining Python and complete Core JS/TS corpora.

NODE-304: Advance Python to ABI 63 with independently selectable dedicated-
Worker Handle evaluation.

- [x] Add `worker-handle-evaluation` over `worker-events` plus `handles` and
  prove its selected Pwright package closure equals Page events plus Handles.
- [x] Evaluate and publish Handles in the exact retained Worker session and
  unique execution context. Reuse the opaque Handle registry instead of
  routing Worker values through a Frame or main-world alias.
- [x] Enforce 64 retained Handles, eight concurrent Handle calls and 64
  cleanup reservations. Make every Handle core retain a weak Worker owner so
  disposal cannot keep the Worker session or CDP connection alive.
- [x] Advance the exact package/native handshake to ABI 63 and add only thin
  async/sync `Worker.evaluate_handle` and `JSHandle.get_property` facades.
- [x] Pass five focused Worker Handle regressions, 932 all-feature host tests
  with 26 explicit live/diagnostic ignores, 2/2 addon tests, strict selected/
  all-feature release Clippy, the 108-package dependency proof, Ruff, strict
  Pyright and 131/131 final installed-wheel Python units.
- [x] Build two repaired ABI3 wheels and verify normalized reproducibility.
- [x] Pass the unchanged official async Chromium Worker Handle lifecycle row
  in final installed candidate and independent oracle, plus 10/10 repeated
  final-wheel invocations. The complete unchanged Chromium Worker file is
  11/13; console argument Handles and Context locale remain explicit failures.
- [x] Update the 6,526-row ledger to 718 passed, zero failed, seven skipped and
  5,801 not run, with exactly one not-run-to-passed change and no losses.
- [ ] Add Worker console argument Handles and BrowserContext locale as
  separate capabilities. Extend Worker-created Handles beyond the selected
  `get_property` lifecycle slice before claiming full Worker Handle parity,
  and continue the remaining Python and complete Core JS/TS corpora.

NODE-303: Keep Python at ABI 62 and add independently selectable dedicated-
Worker network attribution.

- [x] Add `worker-network-events` over `worker-events`, `request-events` and
  `response-events`; prove that its package closure is exactly the combined
  network/page-events closure rather than another adapter stack.
- [x] Enable the Network domain on retained Worker sessions before debugger
  resume. Route Worker and nested-Worker events to the owning Page through a
  64-entry related-session registry without adding Page adapter lanes.
- [x] Key native request correlation by exact `(session, request-id)` scope,
  retain the original native metadata, share the Page's 64-active-request
  limit and drain only the detached Worker's requests.
- [x] Reuse the existing Python Browser network watcher. Add weak Page Request
  and Response identity maps so event, waiter and `response.request` paths
  share public identity without unbounded language retention.
- [x] Pass six focused Rust regressions, 927 all-feature host tests with 26
  explicit live/diagnostic ignores, 2/2 addon tests, selected/all-feature
  strict release Clippy, the 108-package dependency proof, Ruff, strict
  Pyright and 130/130 final installed-wheel Python units.
- [x] Build two repaired ABI3 wheels and verify normalized reproducibility.
- [x] Pass both unchanged official async Chromium Worker-network rows in the
  final installed-wheel candidate and independent oracle, plus 20/20 repeated
  candidate invocations. The complete unchanged Chromium Worker file is now
  10/13; the other three rows remain explicit independent gaps.
- [x] Update the 6,526-row ledger to 717 passed, zero failed, seven skipped and
  5,802 not run, with exactly two not-run-to-passed changes and no losses.
- [ ] Implement Worker console argument Handles, `Worker.evaluate_handle` and
  BrowserContext locale as separate capabilities. Continue the remaining
  Python and complete Core JS/TS corpora; this checkpoint is not full
  Playwright parity.

NODE-302: Keep Python at ABI 62 and add independently selectable dedicated-
Worker error publication.

- [x] Add `worker-error-events` as a separate host/addon/package feature over
  `worker-events` plus `page-errors`; prove it adds no Pwright package beyond
  the existing `page-events` closure.
- [x] Resolve ordinary Frame and Page sessions first, then use the exact
  retained Worker session as the owning-Page fallback for
  `Runtime.exceptionThrown`. Reuse the existing bounded PageError publication
  queue and history without adding a second task, FIFO or resource allowance.
- [x] Keep the Python boundary unchanged: the existing Page-error watcher,
  public `Error` conversion and Page/Context callback identities receive the
  Worker exception. Keep native ABI 62 because no export changed.
- [x] Pass 7/7 focused Page-error host tests, 927 all-feature host tests with
  26 explicit live/diagnostic ignores, 2/2 addon tests, selected/all-feature
  strict release Clippy, the 108-package dependency proof, Ruff, strict
  Pyright and 129/129 final installed-wheel Python units.
- [x] Build two repaired ABI3 wheels and verify normalized reproducibility.
- [x] Pass the unchanged official async Chromium Worker-error row in the final
  installed-wheel candidate and independent oracle, plus 10/10 repeated
  candidate invocations. Update the 6,526-row ledger to 715 passed, zero
  failed, seven skipped and 5,804 not run, with exactly one
  not-run-to-passed change and no losses.
- [ ] Implement Worker console argument Handles, `Worker.evaluate_handle`,
  BrowserContext locale and Worker network attribution as separate
  capabilities. Continue the remaining Python and complete Core JS/TS
  corpora; this checkpoint is not full Playwright parity.

NODE-301: Advance Python to ABI 62 with independently selectable dedicated-
Worker console events.

- [x] Add `worker-console-events` as a separate host/addon/package feature
  over `worker-events` plus `console-events`; prove it adds no Pwright package
  beyond the existing `page-events` closure.
- [x] Keep exact Worker-session correlation, the 128-notice/16 MiB FIFO,
  diagnostic RemoteObject release and cleanup admission in Rust. Release the
  console leaf before the Worker owner registry during host shutdown.
- [x] Export one bounded native notice read and use one Python Browser watcher
  to publish the same `ConsoleMessage` identity to Worker, Page and Context,
  including `ConsoleMessage.worker`. Do not fabricate argument Handles.
- [x] Advance the exact Python/native handshake to ABI 62 because the exported
  native method surface changed.
- [x] Pass 3/3 Worker-console host tests, 926 all-feature host tests with 26
  explicit live/diagnostic ignores, 2/2 addon tests, selected/all-feature
  strict release Clippy, the 108-package dependency proof, Ruff, strict
  Pyright and 129/129 installed-wheel Python units.
- [x] Build two repaired ABI3 wheels and verify normalized reproducibility.
- [x] Pass three unchanged official async Chromium Worker-console rows in the
  final installed-wheel candidate and independent oracle, plus 30/30 repeated
  candidate invocations. Update the 6,526-row ledger to 714 passed, zero
  failed, seven skipped and 5,805 not run, with exactly three
  not-run-to-passed changes and no losses.
- [x] Implement Worker error publication as the independent NODE-302
  capability.
- [ ] Implement Worker console argument Handles, `Worker.evaluate_handle`,
  BrowserContext locale and Worker network attribution as separate
  capabilities. Continue the remaining Python and complete Core JS/TS
  corpora; this checkpoint is not full Playwright parity.

NODE-300: Keep Python at ABI 61 and make dedicated-Worker retirement safe
across Page, Context and Browser shutdown.

- [x] Reproduce the active-Worker shutdown hang and isolate it to
  `browser.close()` after Page and Context close have completed.
- [x] Prove the Page can be removed before a late Worker detach notice, and
  that the resulting unknown-Page traceback retains the native Worker owner
  and CDP connection past the Rust `io_finished` barrier.
- [x] Revoke Worker owners on Page, Context and Browser terminal paths, emit
  close once, reject post-close evaluation, and tolerate exact late detach
  notices without fabricating a synthetic detach.
- [x] Add AAA lifecycle coverage and pass Ruff, strict Pyright and 128/128
  installed-wheel Python units.
- [x] Build two repaired ABI3 wheels and verify normalized reproducibility.
- [x] Pass four unchanged official async Chromium Worker rows from the final
  installed wheel. Update the 6,526-row ledger to 711 passed, zero failed,
  seven skipped and 5,808 not run, with exactly four not-run-to-passed changes
  and no losses.
- [x] Implement Worker console publication as the independent NODE-301
  capability.
- [ ] Implement Worker error publication, Worker Handle evaluation,
  BrowserContext locale and Worker network attribution as independent
  capabilities. Continue the remaining Python and complete Core JS/TS
  corpora; this checkpoint is not full Playwright parity.

NODE-299: Keep Python at ABI 61 and close the official sync caller-dispatcher
discrepancy without moving browser work out of Rust.

- [x] Add a caller-thread asyncio dispatcher compatible with the official
  Playwright `sync_gather` fixture and expose it as `_dispatcher_fiber`.
- [x] Use greenlets only to suspend concurrent sync facade calls. Keep every
  browser coroutine on the single background loop, retain the existing
  32-call admission bound, and resume waiters only from completed futures.
- [x] Make `greenlet>=3.1.1,<4` an explicit wheel dependency; clear the caller
  running-loop marker, cancel helper tasks and close both loops on every stop
  or failed-start path.
- [x] Add AAA coverage for caller-loop task pumping, two overlapping sync
  greenlets, loop-marker cleanup, duplicate-owner rejection and thread join.
- [x] Pass Ruff, Pyright and 127/127 installed-wheel Python units.
- [x] Build two repaired ABI3 wheels, verify normalized reproducibility, and
  prove a clean install acquires greenlet and reports native ABI 61.
- [x] Pass the unchanged focused close/rejection row and all 27/27 cases in the
  complete unchanged Chromium `tests/sync/test_sync.py` from the installed
  wheel. Update the 6,526-row ledger to 707 passed, zero failed, seven skipped
  and 5,812 not run, with exactly one failed-to-passed change and no losses.
- [ ] Continue the complete Python and Core JS/TS corpora; this sync-file
  checkpoint is not full Playwright parity.

NODE-298: Advance Python to ABI 61 with independently selectable dedicated
Worker events and replace the exact official sync discrepancy.

- [x] Add independent host/addon/package `worker-events` features over
  `page-events`; prove the leaf adds no Pwright package beyond that closure.
- [x] Keep Worker session retention, exact Page/nested-Worker association,
  creation order, readiness and detach retirement in Rust. Bound the leaf at
  64 live owners, 128 notices, 1 MiB URLs and eight concurrent evaluations.
- [x] Initialize retained Worker sessions before debugger resume, with Runtime
  enabled and recursive auto-attach installed; cover selection and command
  ordering with an AAA topology regression.
- [x] Keep Python to one Browser watcher plus public async/sync `Worker`,
  `page.workers`, event, predicate and evaluation adaptation.
- [x] Pass the 3/3 standalone Worker host gate, the 1/1 topology integration
  regression, selected/all-feature strict release Clippy, the 108-package
  dependency proof, Ruff, Pyright and 125/125 installed-wheel Python units.
- [x] Build two repaired ABI3 wheels and verify normalized reproducibility.
- [x] Pass the unchanged official Chromium Worker row and improve the complete
  unchanged sync file from 25/27 to 26/27. Merge the non-overlapping full-file
  report into the 6,526-row ledger: 706 passed, one failed, seven skipped and
  5,812 not run, with one failed-to-passed change and no losses.
- [x] Implement and independently verify caller-loop `sync_gather` behavior in
  NODE-299.
- [ ] Continue the complete Python and Core JS/TS corpora; do not treat this
  checkpoint as full parity.

NODE-297: Advance Python to ABI 60 with independently selectable successful
download publication and replace the exact official sync discrepancy.

- [x] Add independent host/addon/package `download-events` features over
  `frame-lifecycle`; prove its dependency closure equals `page-events` and does
  not pull network request/response features.
- [x] Keep exact-Context Chromium policy, Frame/Page correlation, GUID-named
  temporary paths, completion/cancel state, FIFO notice ordering and terminal
  cleanup in Rust. Bound the leaf at 16 Contexts, 64 owners, 64 notices and
  4,096-byte paths.
- [x] Teach the Rust Locator click navigation barrier that a same-Frame
  `Browser.downloadWillBegin` completes a navigation which became a download;
  cover the behavior with an AAA regression.
- [x] Keep Python to one Browser-scoped native watcher and public async/sync
  `Download` adaptation, including sync predicates and `EventInfo.value`.
- [x] Pass the 3/3 standalone download gate, the 1/1 click-barrier regression,
  selected/all-feature strict release Clippy, the 108-package dependency
  proof, Ruff, Pyright and 123/123 installed-wheel Python units.
- [x] Build two repaired ABI3 wheels and verify normalized reproducibility.
- [x] Pass the unchanged official Chromium download row and improve the
  complete unchanged sync file from 24/27 to 25/27. Merge the non-overlapping
  full-file report into the 6,526-row ledger: 705 passed, two failed, seven
  skipped and 5,812 not run, with one failed-to-passed change and no losses.
- [x] Implement and independently verify Worker events in NODE-298.
- [ ] Implement and independently verify caller-loop `sync_gather` behavior.
  Continue the complete Python and Core JS/TS corpora; do not treat this
  checkpoint as full parity.

NODE-296: Advance Python to ABI 59 with independently selectable Page request
events and replace the exact official sync discrepancy.

- [x] Add independent host/addon/package `request-events` features over the
  existing `network-events` transport without implying request history or
  response events; prove its dependency closure equals `network-events`.
- [x] Keep Context subscriptions, Page association, FIFO notice ordering,
  request/history ownership, topology reconciliation and cleanup in Rust.
  Bound the leaf at 16 Contexts, 256 notices and the shared 16 MiB network
  queue budget.
- [x] Keep Python to one Browser-scoped native watcher, public Request wrapper
  publication, and exact async/sync callback identity adaptation. Repair the
  sync wrapper so request callbacks never leak an async facade.
- [x] Pass 8/8 standalone request-event and 16/16 combined network Rust tests,
  selected/all-feature strict release Clippy, the 108-package dependency
  proof, Ruff, Pyright and 120/120 installed-wheel Python units.
- [x] Build two repaired ABI3 wheels and verify normalized reproducibility.
- [x] Pass the unchanged official Chromium network-event row and improve the
  complete unchanged sync file from 23/27 to 24/27. Merge the non-overlapping
  full-file report into the 6,526-row ledger: 704 passed, three failed, seven
  skipped and 5,812 not run, with one failed-to-passed change and no losses.
- [ ] Implement and independently verify the remaining sync-file gaps:
  successful download publication, Worker events and caller-loop
  `sync_gather` behavior. Do not treat this checkpoint as full-corpus parity.

NODE-294: Advance Python to ABI 57 and project Rust-owned console events
without fabricating argument Handles.

- [x] Add an independently selectable addon/package `console-events` feature
  over `page-events` and the existing host console-event observer; keep
  `console-history` independently selectable.
- [x] Deliver type, text, diagnostic argument JSON, source location and Page
  identity through the existing ordered Rust lifecycle publication. Keep
  Python limited to facade identity, callback/waiter dispatch and sync wrapper
  adaptation.
- [x] Refactor Page-error fields into one descriptor so the PyO3 callback tuple
  stays within supported arity, and update the typed native stub to ABI 57.
- [x] Replace the under-sized lifecycle conversion allowance with a 256 KiB
  reservation and compile-time proofs covering maximum Page-error and console
  payloads plus metadata URL copies.
- [x] Pass the complete release wheel build, all-feature strict Clippy,
  standalone `console-events` check/Clippy, 108-package dependency proof,
  Ruff, strict Pyright and 112/112 Python units.
- [ ] Publish real Rust-owned console argument Handles before claiming
  `ConsoleMessage.args`; do not reinterpret the bounded JSON diagnostic
  snapshot as Handles.
- [ ] Run unchanged official `test_console_repr[chromium]` and
  `test_console_should_work[chromium]` in candidate and independent-oracle
  lanes and merge exact outcomes. The zero-tab guard still sees an unrelated
  Gemini page in the shared dedicated Chrome; do not close it or infer runtime
  evidence from the source gates.

NODE-293: Complete Node ElementHandle DOM source ownership without claiming an
unexecuted browser comparison.

- [x] Move exact-owner `textContent`, ordered `$$` publication and the
  temporary `$$eval` array/evaluation/disposal lifecycle into the optional
  Rust `element-handle-dom` leaf.
- [x] Keep Node limited to selector/source validation, error shaping and
  failure-safe wrapper publication; add thin Page/Frame `textContent` Locator
  delegations.
- [x] Match upstream scoped XPath behavior by prefixing root-relative `/`
  expressions inside non-Document roots and filtering non-Element XPath
  results before publication.
- [x] Pass 273/273 package tests, strict TypeScript, standalone strict Clippy,
  selected native build/load, dependency boundaries, formatting and diff
  checks.
- [ ] Run the unchanged query-selector 7-row, eval-on-selector 6-row and
  innerText/textContent 4-row selections in candidate and independent-oracle
  lanes, then merge verified reports into both ledgers. The zero-tab guard
  currently sees an unrelated Gemini page in the shared dedicated Chrome; do
  not close it or treat source completion as runtime evidence.

NODE-292: Project the existing optional Rust ElementHandle DOM owner through
the Node compatibility facade and account for the unchanged official rows.

- [x] Add independently selectable addon/package `element-handle-dom` profiles
  that imply `handles` without changing the default artifact or ABI 1.
- [x] Move `ElementHandle.innerText`, `$` and `$eval` first-match selection to
  typed N-API calls. Rust owns selector compilation, exact target identity,
  retained-handle publication and no-match release; Node only shapes public
  values and errors.
- [x] Add thin `Page.innerText` and `Frame.innerText` methods over the existing
  Rust Locator owner.
- [x] Pass unchanged official query-selector 7/7, eval-on-selector 6/6 and
  inner-text 2/2 in candidate and independent oracle lanes. Candidate report
  SHA-256 values are
  `26daf190d3b76fe3b74f7775e6ed7e26ba6587e50ce8ff4df9c40a564e8c641f`,
  `da103daf6b9ad6cabd363423ad5a77fdd57275a8acd337bd4095faf0fa4f0c91`
  and
  `c216deea53fd876729ba8699c0a0e0d1510ba8fdc5148c3d8022407ff3a69b0f`;
  oracle values are
  `bf69f295c4b9f7b1dfea6c7b78c22ab35c623efc48837ee1ef057f5a1b5477e2`,
  `4de4f288a4e00d3df628f756f23ca994609393283cd8376d19b67e83119b026f`
  and
  `d5561e8c0563123d457b3d36c418996d0cf1e5be648fcbc2ac10043f3271182c`.
- [x] Add validated incremental Core-ledger merging. The candidate ledger is
  1,216 expected / one skipped / 31 unexpected / 47,229 not run, SHA-256
  `6d98659a9a5de4d92c3d74ae4ee6fad7e836c3005384f4423ce20e85bf8a2a08`;
  oracle is 899 expected / 47,578 not run, SHA-256
  `a63c4bcd6dcd69395a758e7700e8e0e83923e1edd0c669c53641e57c97bcdb5b`.
- [x] Repair exact native-class registries for APIRequest feature combinations,
  remove the runtime `sw_vers` process launch, and make Page-binding
  acknowledgement await the bounded Rust command after callback invocation.
- [x] Pass 272/272 package tests, strict TypeScript, the full dependency/native
  class/no-process boundary proof, standalone/full-profile native loads,
  release checks, formatting and selected-profile strict Clippy.
- [x] Superseded by NODE-293 source completion for `ElementHandle.textContent`,
  `$$` and `$$eval`; the official browser rerun and ledger movement remain
  unchecked there. Keep all 31 unexpected and 47,229 unexecuted Core
  invocations explicit; full Core/Python conformance is not complete.

NODE-291: Advance to Python ABI 56 and repair the two ElementHandle failures
without implementing browser semantics in Python.

- [x] Add independently selectable host/addon `element-handle-dom` features;
  keep `handles` usable without selecting this DOM surface.
- [x] Compile typed descendant selectors against the existing element root in
  Rust, publish only an actual element and synchronously acknowledge native
  release for a no-match result.
- [x] Read `HTMLElement.innerText` in Rust and preserve Playwright's
  `Node is not an HTMLElement` error. Python async/sync methods are thin calls.
- [x] Pass the two unchanged official Chromium rows from the final ABI 56
  wheel, report SHA-256
  `55449eed1e22968a614813cec9eb8daab124a1aec6b22b1342c6deed361f3dbd`.
- [x] Merge exact replacements into the 25-row sync tranche: 16 passed / nine
  failed, SHA-256
  `c118e64d7029d39463f1d1a8212398b1b545bc76f6cb1f88bcff5a1871619fb8`.
  Candidate is now 18/27 for the file; oracle remains 26/27.
- [x] Record tranche/cumulative ledger SHA-256 values
  `6fb7ddee2fc902d4f142d8f6da3e7b888ea1f2f07f1c84055c4618ae266c6f1e`
  and
  `a5a1ce1effe030bc97ad9cb9c2405c0f7ee20ed9ba0961f34770f314945501a1`.
  Cumulative candidate is 682 passed, nine failed, seven skipped and 5,828
  not run.
- [x] Build normalized-reproducible ABI 56 wheels with archive SHA-256 values
  `faba2f1c41352d1548e3e94505a3c4ff1ce4b2da39dc8d1b26861355e74888b7`
  and
  `c5501536fbdfc473c71fde14c313d68f6051645c6865fcd6df72c1d509364349`.
- [x] Pass 111 installed Python tests with five expected skips, two focused
  release Rust tests, release Clippy, Ruff, strict Pyright and fresh
  6,526/6,526 collection. Collection report SHA-256 is
  `f3374fe294c20aff18ae1d279acf92ebf70cbe2d0af13813173151eafd5f6a42`.
- [ ] Repair the nine remaining sync failures; eight are candidate-specific
  and one is shared with the official oracle in this isolated harness.

NODE-290: Keep Python ABI 55 and execute every previously unrecorded Chromium
row in unchanged sync `test_sync.py` before selecting another surface.

- [x] Pin unchanged `tests/sync/test_sync.py` at SHA-256
  `f371ee96a8eacc0721e7e3514223992d37ecba8fec86e6396584de4049e03579`.
- [x] Preserve the complete 25-row independent-oracle result: 24 pass and
  `test_sync_playwright_multiple_times[chromium]` fails because the official
  package does not raise its expected sync-inside-asyncio error in this
  isolated harness. Report SHA-256 is
  `3ce79c1d557e9593ba2c4f63db88e3d17db4231e7544516baa19b1e7c73bcb63`.
- [x] Preserve the complete candidate baseline, 9 passed / 16 failed, report
  SHA-256
  `d8bb9824057c1381c2d32158d84c5d92570ea743f69f60a5ebc68ed36dc99626`.
- [x] Add exact public repr shapes for Page, BrowserContext, Browser and
  BrowserType. `BrowserType.executable_path` is honestly empty because this
  package bundles no browser. Keep the upstream-private `_impl_obj` bridge in
  the official shim only; it is not a product API promise.
- [x] Publish the existing Rust request-history owner with Response wait and
  notice records, and expose thin async/sync `Response.request` wrappers.
  Python does not reconstruct request identity or network semantics.
- [x] Rerun the five repaired rows from the final wheel, 5/5. Merge those exact
  replacements with the baseline to produce 14 passed / 11 failed, report
  SHA-256
  `d9b84034f306be4f0bc61ceb9438a2aa79e19ce470c0906981ba17507514696f`.
  Candidate/oracle tranche-ledger SHA-256 values are
  `092b98461c8def4cb2d54b3a463f42b9231e7a8ab74636b9299604a8f2f07177`
  and
  `cf0b46f3d4d2166e08e3353a4a56fee35ee4f3b8ec8d32bb9e9d682e8cf074ea`.
  Cumulative candidate is 680 passed, 11 failed, seven skipped and 5,828 not
  run, SHA-256
  `099db2a6f4937871240a3fb7c329606cb2f9c81fccb58932534bc3c0ef45116e`;
  cumulative oracle is 671 passed, one failed, seven skipped and 5,847 not
  run, SHA-256
  `5e0133285e5a74230173ec2df55f001c5424413a3b10f676cf9a2b3973c33a93`.
- [x] Build normalized-reproducible ABI 55 NODE-290 wheels with archive
  SHA-256
  `bcb27c0bdf17422d0589426c1d8e992aa008f24ff8fe4e49c1bd9a8469d4c99a`
  and
  `f96c9dbbe14c606f7bca41f8310e03947007814e7e2039671703f247c1e8d9ae`.
- [x] Pass 109 installed Python units with five expected skips, 10 focused
  Rust network-history tests, release Clippy, Ruff, strict Pyright and a fresh
  6,526/6,526 candidate collection.
- [ ] Keep the 11 sync failures explicit. One matches the red oracle; the ten
  candidate-specific gaps cover stopped-loop rejection, concurrent-close
  adaptation, console publication, downloads, ElementHandle query/text,
  request publication, nested waiter error precedence and workers. Continue
  the other 5,828 candidate Python rows plus all 48,477 Core JS/TS
  invocations. This is not a green-file or full-suite claim.

NODE-289: Keep Python ABI 55 and complete the applicable Chromium coverage of
unchanged async `test_asyncio.py`.

- [x] Pin unchanged `tests/async/test_asyncio.py` at SHA-256
  `544f9193333ed0df5d87e0f5978b00f04b709aaba80e477cae1e5d1701b1a0ae`.
- [x] Preserve the first independent-oracle attempt as red infrastructure
  evidence: without the Pwright marker Page, fixture setup fails because the
  dedicated-CDP adapter cannot create a Context. With the marker restored, the
  unchanged oracle passes both selected rows; report SHA-256 values are
  `a5ea2c01f248e1fafa01df574c8cef8627fa02724d0911735b9c4f0d18536dae`
  and
  `6328485903ac03a3226871803ca6380c99f6e2369a9c94b6f98c529a28abbb0f`.
- [x] Keep the red candidate baseline. TaskGroup cancellation passes; the API
  name row fails only because its correctly classified native
  `JavascriptError` begins with the browser message instead of
  `Page.evaluate:`. Baseline report SHA-256 is
  `6cb059ea2647f9388c61921b22ada302dc7b52213c0bca5f359303d5f9c099c0`.
- [x] Add a thin Python public-boundary error rewrite. Preserve the exception
  type, stable native `code`, public `name` and browser stack; do not change
  Rust classification, native ABI, browser protocol or resource ownership.
- [x] Final installed candidate and independent oracle pass 2/2 with identical
  report SHA-256
  `6328485903ac03a3226871803ca6380c99f6e2369a9c94b6f98c529a28abbb0f`.
  Candidate/oracle tranche ledgers are
  `a8571298097874f152b85105f5b6e7b41d3ded9aa7811405d454cb0bdd20adba`
  and
  `6b7d99b7dd58f684e11e6ea191671a02a231f3b6e46663c3f1b58866edae5b88`.
  Cumulative candidate is 666 passed, seven skipped and 5,853 not run,
  SHA-256
  `2e8f750a7e2a16af688489ef6aff90fc9467946109b18a007eccfb5e4626fd0d`;
  cumulative oracle is 647 passed, seven skipped and 5,872 not run, SHA-256
  `6d54d97cd037a0e17c8b8f1ce923a6c8459343e04b41ffa19eb5cc5ee98d3215`.
- [x] Build normalized-reproducible repaired ABI 55 NODE-289 wheels with
  archive SHA-256
  `933297c96848dbbe99381ef1a353100bfae8ac9db12c9ec21bba16d3f007aef3`
  and
  `d11377c024ed993dd37b345e95481e0fdc91fb6074e4a4d30a499b80e6ee12e2`.
- [x] Pass 107 installed Python units with five expected skips, Ruff, strict
  Pyright and a fresh 6,526/6,526 candidate collection.
- [ ] Keep the Firefox/WebKit rows for browser-using asyncio tests as explicit
  provider gaps, and continue the other 5,853 candidate Python rows plus all
  48,477 Core JS/TS invocations. This is not a full-suite claim.

NODE-288: Keep Python ABI 55 and close three selector-wait compatibility rows
across async cancellation, timeout error shape and the sync API.

- [x] Pin unchanged async `test_asyncio.py`, async `test_expect_misc.py` and
  sync `test_sync.py` at SHA-256
  `544f9193333ed0df5d87e0f5978b00f04b709aaba80e477cae1e5d1701b1a0ae`,
  `61e5d93df621e59a44717532192194bce1a85cc858d5a1f2e69ba724ce0945a6`
  and
  `f371ee96a8eacc0721e7e3514223992d37ecba8fec86e6396584de4049e03579`.
- [x] Preserve the all-green 3/3 independent oracle: cancel a pending selector
  call without an unretrieved future, expose `TimeoutError.name`, and use
  `wait_for_selector` on an existing sync Page.
- [x] Keep the red candidate baseline. Cancellation and sync pass; timeout
  fails only because the correctly classified native exception lacks `.name`.
- [x] Add the Playwright-shaped `name` property to the existing Python-visible
  native exception classes, with an override setter for Page-error records.
  Do not change ABI, protocol behavior, Rust wait ownership or resource limits.
- [x] Final installed candidate and independent oracle pass 3/3. Report
  SHA-256 values are
  `95061944d9d6e0f66207653b10171d7ebd9d4881683efa7f8efdac77a7c3c611`
  and
  `18763889805affdb62fd0dc6951e665ba662fa67068d7f03b153820c1e6798c4`.
  Candidate/oracle tranche ledgers are
  `1aa8ced79b0daeb738c681fb6a8903f3ceaf33ca6266b984efc4892343dd810a`
  and
  `9f9143a3dfcad40b0b9d572bd87d59f066fde4d3ff154e083e9ce6a447f27dd4`.
  Cumulative candidate is 664 passed, seven skipped and 5,855 not run,
  SHA-256
  `7f8c62ece39f4634eba752e80441a53aabf4f4272cffd5ce84689a75becddfef`;
  cumulative oracle is 645 passed, seven skipped and 5,874 not run, SHA-256
  `b4bd2c9c36f3508b9a5b359d95d7d7685801bede216a55474222b1507ea21565`.
- [x] Build normalized-reproducible repaired ABI 55 NODE-288 wheels with
  archive SHA-256
  `5c2dcc36835eb0c5b26b60b6499824a843175910eba503f0d9ec6b6244f973f1`
  and
  `53016151567480d1af17f39111fb380b3e6148a88ab78f1aeaca1e5f733db0db`.
- [x] Pass 106 installed Python units with five expected skips, Ruff, strict
  Pyright and a fresh 6,526/6,526 candidate collection.
- [ ] Continue the other 5,855 candidate Python rows and all 48,477 Core JS/TS
  invocations. This three-row Chromium checkpoint is not a full-suite claim.

NODE-287: Advance Python to ABI 55 and close two applicable Chromium issue
rows with a thin Page selector-wait facade over the existing Rust Locator
owner.

- [x] Pin unchanged `tests/async/test_issues.py` at SHA-256
  `5d99e3d911d610cdb116e6b2c9e7d7ac7de8df051cfbe823712297465c375bea`.
  Select `test_connection_task_cancel[chromium]` and
  `test_issue_195[chromium]`; both pass in the independent oracle.
- [x] Add async/sync `Page.wait_for_selector` without Python polling. Default
  non-strict calls derive native `nth(0)`, Rust owns Frame resolution plus one
  total wait deadline, and only the existing Handle publication path may
  return an `ElementHandle`.
- [x] Verify task cancellation against the unchanged upstream case. The
  winning visible wait returns a real Handle, the unresolved sibling raises
  `CancelledError`, and fixture teardown exits normally.
- [x] Preserve applicability diagnostics instead of greenwashing them. The
  whole-file oracle report is red only because `test_issue_189` directly uses
  unsupported generic `launch(ignore_default_args=...)`; the separate service-
  worker-policy report retains two HTTP/MagicDNS console timeouts outside the
  ledger.
- [x] Final installed candidate and independent oracle pass 2/2. Report
  SHA-256 values are
  `18b7130d89320ae47ee72ee45c452d4d6a7b7204885921a1b5b7b624b6851718`
  and
  `991b5bf6cc6fc47d38cf7f4ce02fbca7964ef3b6c2c59bad9ec71cff2bcacb1f`.
  Candidate/oracle tranche ledgers are
  `67542a752ae8dbd54d91dfab1ed69ca7ae59995a59bbb12894337493f05b00d0`
  and
  `c59574200b25fb454bccc03e7ad041ebe0bfc652c1fb8213db3ef93b192aad96`.
  Cumulative candidate is 661 passed, seven skipped and 5,858 not run,
  SHA-256
  `169dfbabd9bf984d59082cdf4c0b68a84941a67c7e04c8997cc63645243561e9`;
  cumulative oracle is 642 passed, seven skipped and 5,877 not run, SHA-256
  `a2f4c2118f7136c5f607e594638b68acc95fb88edd1e7f308f76cd799b4f4499`.
- [x] Build normalized-reproducible repaired ABI 55 wheels with archive
  SHA-256
  `11487ff7a6afb5b7d4ba021cc746dea4003bb646bfc3301b7a74a4b79a674869`
  and
  `4b493951e935460f9b3d752424f41b60bf58b10315306fdfb3633ffd141981aa`.
- [x] Pass 105 installed Python units with five expected skips, 497 selected
  Rust units with 16 explicit live ignores, strict Ruff/Pyright/Clippy,
  formatting and the 108-package dependency proof.
- [ ] Continue the other 5,858 candidate Python rows and all 48,477 Core JS/TS
  invocations. This two-row Chromium checkpoint is not a full-suite claim.

NODE-286: Advance Python to ABI 54 and close the complete unchanged sync input
file with composable Rust-owned file-chooser events and bounded exact-node file
mutation.

- [x] Pin `tests/sync/test_input.py` at SHA-256
  `c203939164958aca0babcee63f2d9ae58afce16199fc2b6b5adbc4d6b2284fed`.
  Its six expanded rows are fully accounted: both Chromium rows execute and
  four Firefox/WebKit rows remain explicit provider-gap `not-run` outcomes.
- [x] Add the independently selectable `file-chooser-events` composition.
  Enable interception on each selected Page/OOPIF session, map the event to the
  exact Page and utility route on the same CDP cursor, and publish through the
  existing Page lifecycle fence rather than adding another event loop.
- [x] Reuse the bounded file-input payload/path owner for the event's exact
  backend node. Admit at most 16 live chooser owners, charge 2 KiB per owner
  plus 1 KiB installation state, and release the unique utility object group
  on every settlement path.
- [x] Preserve the teardown diagnostic: a strong native connection let the
  assertion pass but hung fixture shutdown. Replace it with a weak connection
  plus exact route, then verify the unchanged official process exits cleanly.
- [x] Final repaired-wheel candidate and independent oracle pass 2/2; report
  SHA-256 values are
  `2b2f38ee68d026f3ea1d6eb6f46f8e30edfb8a2a0882921e2cd57622a50d1d38`
  and
  `0c84f8e75291bdae4bd87685ba9efbc86c12e598b5859a1b7e1f8d7e42e15685`.
  Candidate/oracle tranche ledgers are
  `f39fc100ccaf2601cf852b32ca3e8c6543f816e0c34b6ff82eb251673f777b89`
  and
  `ab9953fae7ddd08720f454d84fb5be75ec0b61b3abf48f6bbd95a2eefc37ebea`.
  Cumulative candidate is 659 passed, seven skipped and 5,860 not run,
  SHA-256
  `557e937c00ccc8e9065c37df2c14fde9b7d71ae0c1b9e011f02171b52d71730a`;
  cumulative oracle is 640 passed, seven skipped and 5,879 not run, SHA-256
  `d8bb9714f250d46c17c2afd6e5468eb98f8acf8a7a0f84f0a016bffd8d553534`.
- [x] Build normalized-reproducible repaired ABI 54 wheels with archive
  SHA-256
  `ff2b00f7524235c3dbdfb1f93f3e29b7d9b3da885b550819c8be2b8df0eb6752`
  and
  `6584f4b92359e0fd1cddcfaf0bb814980b682eec5a2033457077c97c005d385c`.
  Pass 103 installed Python units, 497 selected Rust units with 16 explicit
  live ignores, strict Ruff/Pyright/Clippy, formatting and the 108-package
  dependency proof.
- [ ] Continue the other 5,860 candidate Python rows and all 48,477 Core JS/TS
  invocations. This two-row Chromium checkpoint is not a full-suite claim.

NODE-285: Advance Python to ABI 53 and close the complete unchanged async Page
network-request file with composable Rust-owned request history, frame and
post-data behavior.

- [x] Pin `tests/async/test_page_network_request.py` at SHA-256
  `6ead05641f44950db30e7ab038c973226ebe726312f39b9d7d7de2aacaf4a388`.
  Its six expanded rows are fully accounted: both Chromium rows execute and
  four Firefox/WebKit rows remain explicit provider-gap `not-run` outcomes.
- [x] Add thin `BrowserContext.wait_for_event("request")`, `Request.frame`,
  `Request.post_data_json`, `Request.response()` and navigation-request
  facades. Rust owns the bounded waiter queue and the exact shared request-
  history owner; Python only forwards the opaque records.
- [x] Match Playwright's popup-first-request ordering in the optional
  `request-frame` leaf. The network adapter observes `Page.frame*` and
  `Network.*` on the same CDP cursor, retains at most 64 Frame IDs and one
  boolean per at-most-64 active request, and accounts both maps in the
  production resource profile. A later topology snapshot can no longer turn
  the popup's unavailable Frame into an available one.
- [x] Final installed candidate and independent oracle pass both Chromium
  rows; report SHA-256 values are
  `c6fd93810787dea30f3beb58ec44202bdff7aad5975f1259852f363faae454cc`
  and
  `1d0416cb3d1e95cb2eab41e3118b7444fe2e47403a6bd98039d70837b392cba6`.
  Three consecutive NODE-281-through-NODE-285 candidate regressions pass
  10/10, SHA-256
  `60b7992906b838a93a98f8c61aa2a803dbc7485e9b3a358530aaea3f562b26f2`
  and
  `ac10e36d1bc833396547276ac1af8e9c79ff0932a5e2eaae9201936621bb34f7`
  and
  `cfe2c9dc9e7733e09e9b26bce562837a1b0a0dd75fe44347f0a2c6d81c7adac9`.
- [x] Record candidate/oracle tranche-ledger SHA-256 values
  `e7b97ab2482e90363debe305044ccd21efc08e7eaaa95f368ffb571abe02ba7f`
  and
  `3e7525c78dabbb19ab31e069074b0fab1258afae97d6e8dfebca627525c481c2`.
  Cumulative candidate is 657 passed, seven skipped and 5,862 not run,
  SHA-256
  `83ed08bf4d1f711e6d506db5a2315bc6e7aed49dff4717d0db95750074c0318b`;
  cumulative oracle is 638 passed, seven skipped and 5,881 not run, SHA-256
  `eb03126624a1c047420fa30d1a8ab1b8d5ceacd793bcca2e0f081d495f0b1d96`.
- [x] Build normalized-reproducible ABI 53 wheels with archive SHA-256
  `da1fcbdee1c0e1bfac900f3d054851942c792b5fd5958d2f15405c1c49d4b41c`
  and
  `0e51fcf2a4996266d7c19b070b2515fea5f335c00ec3414150b30c6b9f52fad0`.
  Pass 103 installed Python units, selected Rust tests, strict
  Ruff/Pyright/Clippy gates and the 108-package dependency proof. Fresh
  6,526/6,526 collection has SHA-256
  `66affc14f9da9eb21d94b750855572592ea92d1f0b039fbf6f18cdaeb01687b7`.
- [ ] Continue the other 5,862 candidate Python rows and all 48,477 Core JS/TS
  invocations. This two-row Chromium checkpoint is not a full-suite claim.

NODE-284: Advance Python to ABI 52 and close both complete unchanged
async/sync Page network-response files with bounded Rust-owned terminal
response completion.

- [x] Pin `tests/async/test_page_network_response.py` and
  `tests/sync/test_page_network_response.py` at SHA-256
  `707b20a48b1b89529228697d5a87cb42a965a8afb9a485a5c951839c67dc2cba`
  and
  `82e3f5cfd0581f92085d1b56c06936fb890627dd505189b4894e36d7949e39fd`.
  Their 12 expanded rows are fully accounted: all four Chromium rows execute,
  while eight Firefox/WebKit rows remain explicit provider-gap `not-run`
  outcomes.
- [x] Preserve the first product-reaching ABI 51 diagnostic: navigation and
  fetch setup succeed, then all four selected rows fail only because
  `Response.finished` is absent, report SHA-256
  `08c1a8411006bac153e900a086961b54c23a9828dddff646548e2e7e77154895`.
  The earlier TodoMVC example report is setup-only evidence and is excluded
  from product outcomes.
- [x] Add the independently selectable `response-finished` capability in ABI
  52. Rust owns pending/success/failure/closed state, terminal subscriptions
  and wakeups, with at most 256 owners and 64 KiB retained state. Owner slots
  are reclaimed on drop; Page/session retirement closes pending owners.
  Async/sync Python only forwards the opaque native Response owner.
- [x] Final installed candidate and independent oracle pass all four Chromium
  rows through the same dedicated CDP Chrome; report SHA-256 values are
  `98a5f800bdeb271affa5947bd91722e615129835d384ac5cb19e0ae81fe52b00`
  and
  `807cc17edba597a72a058041ce73de864942c36dbbc2def5179f2c5445fafa06`.
  A NODE-281-through-NODE-284 candidate regression passes 8/8, SHA-256
  `04ebd36660f93213091f0ac17235007f68fe471c477f3c0c7c96305f4086a65b`.
- [x] Record candidate/oracle tranche ledger SHA-256 values
  `28ec4b14143f3c74959798890494949f1e6e3cafd85bc09da1be9375717c121d`
  and
  `ae2b24671287c846f9be434f9a0261780378578a32cbaae39db8570b10ef9197`.
  Cumulative candidate is 655 passed, seven skipped and 5,864 not run,
  SHA-256
  `82dad9ef03972b03a4174dad73b07409b59776839d2e9885a01e81fb35f6f0c7`;
  cumulative oracle is 636 passed, seven skipped and 5,883 not run, SHA-256
  `f05f91ebfaafaa1766eef99bda1e2123a87684ad544ce20c715bc5fd6d76b6e7`.
- [x] Build normalized-reproducible ABI 52 wheels with archive SHA-256
  `6eec71b2bcc8cc2fe658a0e28a5adab537370e74f20ddb62928ec521d23b102c`
  and
  `bd406d7ea915d9f95cb3d79c7ef13433ae8ef632c6b77d34059a82268b94b3aa`.
  Pass 98 installed Python units, 12 targeted Rust host tests, strict
  Ruff/Pyright/Clippy gates and the 108-package dependency proof. Fresh
  6,526/6,526 collection has SHA-256
  `73608b054e7dd88df4ca79accebffa6f69561faa30a0868da4d18f5e014c2c2f`.
- [ ] Continue the other 5,864 candidate Python rows and all 48,477 Core JS/TS
  invocations. This four-row Chromium checkpoint is not a full-suite claim.

NODE-283: Advance Python to ABI 51 and close the complete unchanged shared
SIGINT lifecycle file with a bounded Rust-owned timeout wait.

- [x] Pin `tests/common/test_signals.py` at SHA-256
  `7f3d9cfc43ca8c5431df25b25ef75f1b38b0f9bac881e893384f622a3dc73267`.
  Its async and sync Chromium rows install an application SIGINT handler,
  launch with `handle_sigint=False`, wait through `Page.wait_for_timeout`, and
  verify ordered Context/Browser/Playwright cleanup after the parent sends
  SIGINT.
- [x] Export the existing independent Rust `wait-for-timeout` leaf through
  ABI 51. Rust admits at most 16 concurrent waits and 12 KiB total retained
  state, and selects between session stop, disconnect and the bounded Tokio
  timer. Async/sync Python only validate milliseconds and forward the call.
- [x] Make macOS spawned test children import only the pinned upstream `tests`
  package, without exposing its source `playwright` package to the oracle.
  Install the same CDP-attach launch adapter in spawned oracle children and
  validate the explicit `chrome` channel in both lanes. Superseded import and
  channel diagnostics are excluded from the green ledger.
- [x] Final installed candidate and independent oracle pass both Chromium rows,
  report SHA-256 values
  `15327d9f5ab947fd6c9090f7f33c1be479c55355c6ab317e2cbd9985dae6f91e`
  and
  `a2b8d0ba5c8f9f3ebc78bcf4293042578af96805a4145305d008a0f7aa01300a`.
  Firefox/WebKit remain four explicit provider-gap `not-run` rows. A
  NODE-281-through-NODE-283 candidate regression passes 4/4, SHA-256
  `48997deecaa3332fcf8ba1a1cdafd7437fde07bac3b69b7cacd61322d063449d`.
- [x] Record candidate/oracle signal-tranche ledger SHA-256 values
  `2bd05bee87df53c356065d66262d3688a943f113566596671f4bbaf490e65c4a`
  and
  `1d3e8009024c507900cd1ae4d15ecf087d61d6ea58f4e895c62797199a2a8971`.
  Cumulative candidate is 651 passed, seven skipped and 5,868 not run,
  SHA-256
  `b64fcdc19f027fd37718a339b122ecb6ea240890e12b4c56c147baac4f3ae0e4`;
  cumulative oracle is 632 passed, seven skipped and 5,887 not run, SHA-256
  `48dd00b23f5df8ad5fb26bf3b4a74dc4ac57c55f9b5ac85f54ff38b69c7789e8`.
- [x] Build normalized-reproducible ABI 51 wheels with archive SHA-256
  `39bf27fbf7d6bdd9d6c1513f11b6c0b37ba2928b2a70c7f921f67593dd3dbfe1`
  and
  `54b6323011e22a19f6b6beceba01fab317fc1bfbc11869f73f305e88acc1f0f0`.
  Pass 96 installed Python units, the targeted Rust capacity test, strict
  Ruff/Pyright/Clippy gates, Rust formatting, the 108-package dependency proof
  and fresh 6,526/6,526 collection, SHA-256
  `d2b169846ada9162934a60b9e542e99f55096f43fa5a1a2d988cf2d935e372c8`.
- [ ] Continue the other 5,868 candidate Python rows and all 48,477 Core JS/TS
  invocations. This two-row Chromium checkpoint is not a full-suite claim.

NODE-282: Keep Python ABI 50 and close the complete unchanged shared event-
listener error file with a bounded sync-call propagation channel.

- [x] Pin `tests/common/test_events.py` at SHA-256
  `34a3a26326764909a9fd0e1b97282ea5ca689c4a8c45f142725d10c42ee519ce`.
  The initial installed-candidate diagnostic runs the response handler but
  fails because `page.goto()` does not raise its `Exception("fail")`, report
  SHA-256
  `f694bc243739cb29f7e83140749b89f64432a1dec489870f2617793a8199264d`.
- [x] Add one lock-protected pending-listener-error slot to the bounded sync
  loop. Wrapped Page handlers record only the first pending exception and
  rethrow it after the public call finishes, so a cleanup call still executes
  before surfacing an otherwise-idle listener failure.
- [x] Final installed candidate and independent oracle pass the Chromium row;
  report SHA-256 values are
  `b0cb31378655ab7131bda8d57255e28b8378f2ace1275933e61230aa173c4a42`
  and
  `940665a60c4023e1f2e05a32fa3045339f6052406ddc71df1a7f88b2ffab4fef`.
  Firefox/WebKit remain explicit provider-gap `not-run` rows. A two-file
  candidate regression reruns NODE-281 and NODE-282 as 2/2, SHA-256
  `04e6bf146f45099b4b56f3ab80f78daadeec5a580185b67ab7c79aeeeeec046e`.
- [x] Record candidate/oracle event-tranche ledger SHA-256 values
  `5d0c9e0aac75f181785220fab4962664e562c815b2f9ceed0933cd320a0de2a5`
  and
  `f86168a3b461572ae0a8e2bbe6a99ec5503c416fcb89e9551cac77be9ba3b846`.
  Cumulative candidate is 649 passed, seven skipped and 5,870 not run,
  SHA-256
  `3e75c9588a73cd741ac571800a430e290affb178aee3bed5ab0951b46987602d`;
  cumulative oracle is 630 passed, seven skipped and 5,889 not run, SHA-256
  `c895df37d1112f67e24f5e31b8d7bb2a13982261e792b3d45c966faf65e05b6e`.
- [x] Build normalized-reproducible ABI 50 wheels with archive SHA-256
  `d70c33a033f01ff57dddcf6cee6bca90b31203e62e9a67cf6877b669202a051c`
  and
  `5e09beec25d3982f14a70550475fb0bb6f0c6bcf50c6aeba92e364181678ea7d`.
  Pass 94 installed Python units, Ruff/Pyright, exact wheel-profile Clippy,
  Rust formatting, the 108-package dependency proof and fresh 6,526/6,526
  collection, SHA-256
  `5a8c37f74beaee9487a5028138ae35af821bdd18ffa731d776251550e5587f7a`.
- [ ] Continue the other 5,870 candidate Python rows and all 48,477 Core JS/TS
  invocations. This one-row Chromium checkpoint is not a full-suite claim.

NODE-281: Keep Python ABI 50 and close the complete unchanged shared thread-
lifecycle file without adding product code.

- [x] Pin `tests/common/test_threads.py` at SHA-256
  `8e42fa88bfa47b5f6ba501c2731d3d9c1812fddb28b4451aba923fb93bfe8c64`.
  The final installed candidate and independent oracle pass the Chromium row;
  report SHA-256 values are
  `1fdf1cf683d7ca5bba9f12db51f81bf2fb083d1d230520a25117f43c39ab254a`
  and
  `4b10bc1a7187c08b13e25a639cbd33baa45306edc04f0c6916913ddc3c48de59`.
  Firefox/WebKit remain explicit provider-gap `not-run` rows.
- [x] Preserve the upstream worker-thread boundary unchanged: construct and
  stop `sync_playwright`, attach Chromium, create a convenience Page and close
  the Browser from the worker thread. The existing bounded sync loop and Rust
  owners pass, so this checkpoint adds no Python or Rust implementation.
- [x] Record candidate/oracle tranche ledger SHA-256 values
  `ee364ba1464514d18d20d6160a14062b048c9c967bee4867760a86152e5aef09`
  and
  `c61fc526cfc53f6927d9322c0aa4c22944464186ae4516952da809674ff22a42`.
  Cumulative candidate is 648 passed, seven skipped and 5,871 not run,
  SHA-256
  `4febdfe050081de20ef509f153f4e77669eb2672d22be1a07853d9e2e1916345`;
  cumulative oracle is 629 passed, seven skipped and 5,890 not run, SHA-256
  `d9fb4a9faf432e4cc60c473b373ac9076afb98eb0dd4d9ee3ed62beb0d14b06e`.
- [x] Reuse the unchanged normalized-reproducible ABI 50 wheels SHA-256
  `0ae0b675aefe9479cc66b551a815e95a479be8419a09a839402a17d07badccec`
  and
  `db9345fce03a55a770371532d84e71e04824994cafd650ca8492adac96db0353`.
  The NODE-280 93-unit, 895-host-test, addon, strict lint and 108-package
  dependency gates remain applicable because product code did not change;
  fresh 6,526/6,526 candidate collection has SHA-256
  `c4871850fb0b392fbabcac9feef6d144d50dc68e0233597d2207ca22e0da04dd`.
- [ ] Continue the other 5,871 candidate Python rows and all 48,477 Core JS/TS
  invocations. This one-row Chromium checkpoint is not a full-suite claim.

NODE-280: Keep Python ABI 50 and close the complete unchanged async selector-
generator file with bounded Rust-owned test-id configuration and strict-error
diagnostics.

- [x] Pin `tests/async/test_selector_generator.py` at SHA-256
  `60ae2e5324fd30a820402d4feaa28dac2243caaef227a28ad9b1773e3d403b8d`.
  The final installed candidate and independent oracle pass the Chromium row;
  report SHA-256 values are
  `c38cfd4ee833a79dd43ae44e6574ea15d3cf10c3321846c088bdcc570f597537`
  and
  `881bcdbf424fab522b7eccd9ba0a4c9555b419b209efe29e025a659d6c1b6c2f`.
  Firefox/WebKit remain explicit provider-gap `not-run` rows.
- [x] Preserve two candidate diagnostics: v1 fails at the missing
  `Selectors.set_test_id_attribute`; v2 reaches hover and exposes the distinct
  `strict CSS selector` backend prefix. Final Rust accepts only the structured
  strict prefixes and generates at most five previews, 512 HTML UTF-16 units
  and 256 test-id UTF-16 units per preview.
- [x] Keep the mutable setting in the bounded Playwright-scoped Rust selector
  registry. Semantic test-id Locators take immutable snapshots; action-time
  strict diagnostics use the current setting. Python async/sync setters are
  thin forwarding methods.
- [x] Record candidate/oracle tranche ledger SHA-256 values
  `4fff7e510e7cc37945776930e3bc03a2e133cf00d5a1d600ea729b79884c4c2b`
  and
  `ecb0fc1580fd76e5dc05245fe9fe66b0d9014ac00e53230cb4a8e9dbd6079ff4`.
  Cumulative candidate is 647 passed, seven skipped and 5,872 not run,
  SHA-256
  `38ab5abac559594a16e9faedab66525f7a168e4bddcf765da1b294a30139a10d`;
  cumulative oracle is 628 passed, seven skipped and 5,891 not run, SHA-256
  `27c233a777873c1b97d4cb503768603de325c23c23c0d699b322bd130575a6a6`.
- [x] Build two normalized-reproducible ABI 50 wheels with archive SHA-256
  `0ae0b675aefe9479cc66b551a815e95a479be8419a09a839402a17d07badccec`
  and
  `db9345fce03a55a770371532d84e71e04824994cafd650ca8492adac96db0353`.
  Pass 93 installed Python units, 895 all-feature host tests plus 26 explicit
  live/diagnostic ignores, two addon units, strict release Clippy,
  Ruff/Pyright, the 108-package dependency proof and fresh 6,526/6,526
  collection, SHA-256
  `ea207f49af313c2b31f6c26d85ac6cd1e63d66f3afa7f24ad160b7c1b4e01499`.
- [ ] Continue the other 5,872 candidate Python rows and all 48,477 Core JS/TS
  invocations. This one-row Chromium checkpoint is not a full-suite claim.

NODE-279: Keep Python ABI 50 and close every expanded row from the complete
unchanged async legacy-text selector file while preserving provider gaps.

- [x] Pin `tests/async/test_selectors_text.py` at SHA-256
  `d8b841099bddcb972bf81846f050cac8880f65fe5c6f73b1c44a26d556d39332`.
  Final installed candidate and independent oracle pass both Chromium rows;
  report SHA-256 values are
  `214264e2a9babd5277c448a82d86ba11ace901635120cdd047eb4437a76595c3`
  and
  `d3204cf9be77ba876630627f7f9a81f88017bde7f54f64ebbe7b2f3f58828efa`.
  The two Firefox and two WebKit expansions remain explicit provider-gap
  `not-run` rows because the source has no Chromium-only marker.
- [x] Keep selector semantics typed and Rust-owned. Public quoted legacy text
  uses immediate text-node chunks; semantic `get_by_text` retains descendant
  text behavior; regex text is typed; and selector-chain tokenization now
  respects quoted, escaped and regex-contained `>>` boundaries.
- [x] Add thin async/sync `Page.query_selector_all`, typed non-strict-first
  `Page.query_selector`/`eval_on_selector`, and `to_have_id` adapters over
  existing bounded Rust owners. Locator evaluation remains strict.
- [x] Advance the selector protocol to 1.1 and Locator protocols to 1.24/2.7.
  Older 1.23/2.6 peers reject the new legacy-text variant instead of silently
  misdecoding it.
- [x] Record candidate/oracle tranche ledger SHA-256 values
  `ed9435fad8c7248c38d19f726a7afc1afe3c1d3d3f7a7125d42913bc1c049c87`
  and
  `0e3764ca45a20c458185b06f5ec83b4942eb723019dcf6f746aff19789d224c6`.
  Cumulative candidate is 646 passed, seven skipped and 5,873 not run,
  SHA-256
  `603945f33ceadde881b175508db8402092fe71f246b3a025ea2b9277227fde25`;
  cumulative oracle is 627 passed, seven skipped and 5,892 not run, SHA-256
  `d19ab2d5f567fd275b46cb93e37b07dd631db0175345208fe3365fd0a6c2e98a`.
- [x] Build two normalized-reproducible ABI 50 wheels with archive SHA-256
  `8c2739a837b625d8a8da4f394752dbf6e40f7d39a62a2b319baf3c1f9a063f85`
  and
  `45266c5d9c8ad7e2f3c4552cacb460365640447cd6131777030d83d25c134e04`.
  Pass 92 installed Python units, 891 all-feature host tests plus 26 explicit
  live/diagnostic ignores, two addon units, strict release Clippy,
  Ruff/Pyright, the 108-package dependency proof and fresh 6,526/6,526
  candidate collection, collection SHA-256
  `9d2d60c194ec015e2e141a29eac2479063bef342f22dbc80c4ed1afe8228f283`.
- [ ] Continue the other 5,873 candidate Python rows and all 48,477 Core JS/TS
  invocations. This two-row Chromium checkpoint is not a full-suite claim.

NODE-278: Advance Python to ABI 50 and execute every expanded row from the
complete unchanged async screenshot file without hiding non-Chromium provider
gaps.

- [x] Preserve the ABI 49 candidate diagnostic: all three Chromium rows fail
  at the missing `Page.screenshot` facade, report SHA-256
  `f38c3a716fae71f63efcc81352fdf4e3805a14e03a3e42ca228f3f89b44e0b59`.
  The independent official oracle passes all three, report SHA-256
  `d3f45897a07e19a2ac3aa2ac7a737bbaf67d1b3088bb12b8d819632142ca11e4`.
- [x] Keep screenshot semantics in bounded Rust owners. Page, Locator and exact
  retained ElementHandle capture share finite capture/output admission; mask
  setup admits at most 64 Locators and 256 matching elements per Locator and
  is cleaned up on both success and failure. Rust owns document-space element
  clipping and Chromium `captureBeyondViewport`; Python only validates public
  path/type/mask values and forwards native identities.
- [x] Run all three Chromium invocations from unchanged
  `tests/async/test_screenshot.py`. The installed ABI 50 candidate and oracle
  pass 3/3; candidate report SHA-256
  `ad7e0c2b043641fa39a4fcac775945a193c4fef63a507f17a4572150bd270c58`.
  The file has no Chromium-only marker, so its three Firefox and three WebKit
  expansions remain explicit provider-gap `not-run` rows rather than being
  called non-applicable.
- [x] Record candidate/oracle tranche ledger SHA-256 values
  `46b1f1fafba03863f5da38fb691e1afa37296b590990d5c750ee1c9153fd22d3`
  and
  `06c53a9fa7b23cd239f0bb9a8189bad35a6576cfdc7450b52daaa41c1364a6d3`.
  Update cumulative candidate to 644 passed, seven skipped and 5,875 not run,
  SHA-256
  `f898f89d210cfb1c73d37b5817b68688050fae9f273c54c92480f4e0375aaf5c`;
  cumulative oracle is 625 passed, seven skipped and 5,894 not run, SHA-256
  `aa53c89869c367c05976854317d9d0e50964b6cc7b0617602eba54af5a4b84b8`.
- [x] Build two normalized-reproducible ABI 50 wheels with archive SHA-256
  `21017fae60321b12ab56bcc1b000e01e18c8892caa48e085a5622fa84a1da9ce`
  and
  `6d8369d2bb334fb28f650f244172a269bb2f0f694e14c42c5f4b7f06cbc3dfd7`.
  Pass 88 installed Python units, 886 all-feature host tests plus 26 explicit
  live/diagnostic ignores, two addon units, strict Clippy/Ruff/Pyright, the
  108-package dependency proof and fresh 6,526/6,526 candidate collection,
  collection SHA-256
  `6980ab9cfc863becc420c497f98a2a9aec3622935dfd0b542092dcc32288d0cf`.
- [ ] Continue the other 5,875 candidate Python rows and all 48,477 Core JS/TS
  invocations. This three-row Chromium checkpoint is not a full-suite claim.

NODE-277: Advance Python to ABI 49 and execute every expanded row from the
complete unchanged async PDF file, including tagged document outlines.

- [x] Preserve the ABI 48 candidate diagnostic: capture/save pass, while the
  outline row fails only because `tagged` and `outline` are explicitly
  uninstalled, report SHA-256
  `59e0bff3910ee10e21c2d61a11f6ddd6bc5fb04a5f88f7e63defba159848b33b`.
  The independent official oracle passes all three Chromium rows, report
  SHA-256
  `647a7ca744aa2a3899d3f3756d1b7c5df38a213faf4f4e052b4b99948d3a6077`.
- [x] Keep option semantics in the existing Rust `page-pdf` owner. Python
  strictly validates and forwards two optional booleans; Rust maps them to
  `generateTaggedPDF` and `generateDocumentOutline` while preserving the
  bounded `ReturnAsStream` read/close path and 2 MiB output lease.
- [x] Run all three applicable Chromium invocations from unchanged
  `tests/async/test_pdf.py`. The installed ABI 49 candidate and oracle pass
  3/3; candidate report SHA-256
  `248379bc35eb7c2b78e6e3293ec7a7b9ada3a2f4327883765880e03153ea3355`.
  The file's six Firefox/WebKit expansions remain explicit upstream
  Chromium-only non-applicable `not-run` rows.
- [x] Record candidate/oracle tranche ledger SHA-256 values
  `75bdbdbf039c1238e80219a7a7aae0f6d828fa87077cef35b2c21a845a4142cf`
  and
  `c67b9b11e5b598023ef28f3d7f6ef5ca63cc9742cab089acf94985e33f11c653`.
  Update cumulative candidate to 641 passed, seven skipped and 5,878 not run,
  SHA-256
  `311867644fb8f62e11af82b1cdb08ce58392f1a4dfe2a03bc0bd7cc1ff387cd8`;
  cumulative oracle is 622 passed, seven skipped and 5,897 not run, SHA-256
  `895826486a6d1831cbabaf2eef041d822ab520836232682b680d69d1df01ccfe`.
- [x] Build two normalized-reproducible ABI 49 wheels with archive SHA-256
  `e837346a03487651337816b4adb8e93c6f48860b94252be8d00c8a33088d7e34`
  and
  `68ecf94ec7845f836f0b09dd5f8dffd8fab64ed0e64cbd49029f77fcba9cec28`.
  Pass 86 installed Python units, 883 all-feature host tests plus 26 explicit
  live/diagnostic ignores, two addon units, strict Clippy/Ruff/Pyright, the
  108-package dependency proof and fresh 6,526/6,526 candidate collection,
  collection SHA-256
  `bad14cc3e732be2f400aa34ce4a27f2b447b2f121193ccd3294da3a13c2a0007`.
- [ ] Continue the other 5,878 candidate Python rows and all 48,477 Core JS/TS
  invocations. This three-row Chromium checkpoint is not a full-suite claim.

NODE-276: Advance Python to ABI 48 and execute every expanded row from the
complete unchanged sync PDF file while keeping capture and output ownership in
bounded Rust.

- [x] Preserve the ABI 47 candidate diagnostic: both Chromium rows fail with
  missing `Page.pdf`, report SHA-256
  `677f6e364d147cf8c03837544076285a620fdac02cf561eb5eec9fd1962b3476`.
  The independent official oracle passes both rows, report SHA-256
  `036bffd3a26facd620b478f49ec81f98a70f01274cd51c1d925d370e1975d908`.
- [x] Add an independently selectable Rust `page-pdf` leaf. Rust owns exact
  Page/session lookup, `Page.printToPDF` dispatch, finite capture queuing,
  the Playwright v1.58 `ReturnAsStream` protocol, a bounded `IO.read` loop,
  raw/base64 chunk decoding, mandatory `IO.close`, a 2 MiB per-document bound,
  two concurrent retained outputs and optional path persistence. Python only
  validates that no uninstalled PDF options were requested and forwards
  Page/path values.
- [x] Run both applicable Chromium invocations from unchanged
  `tests/sync/test_pdf.py`. The installed ABI 48 candidate passes 2/2, report
  SHA-256
  `c36fb38329822687850d1900d4b24a4b10361d423011ea8efea3c9adf619ac3e`.
  The file's four Firefox/WebKit expansions remain explicit upstream
  `only_browser("chromium")` non-applicable `not-run` rows.
- [x] Record candidate/oracle tranche ledger SHA-256 values
  `18b846b9dc946359f730e4fa6e1f74faad79747975c9fa72621e113bc26b1e45`
  and
  `54635dc63be374697eebd7a068ed199ea8e0f5f0e3b00505d3c7f6b1d2c1270a`.
  Update cumulative candidate to 638 passed, seven skipped and 5,881 not run,
  SHA-256
  `801181534ae92df917d69d02f0d03708981c14ab8a171fefa7086e12acae3b02`;
  cumulative oracle is 619 passed, seven skipped and 5,900 not run, SHA-256
  `a9f7b78946734baef302d23a8cada3da5d3d707865957a41fcdb1e3041828a01`.
- [x] Build two normalized-reproducible ABI 48 wheels with archive SHA-256
  `48593cfdd4a22249e2eee14186faceb95235670657e52aa808680d6e9a2fe348`
  and
  `b9acf998197b093ec2597c996193e6b3256fc97dfa23e73f3299de85c59bd2af`.
  Pass 85 installed Python units, 882 all-feature host units plus 26 explicit
  live/diagnostic ignores, two addon units, strict Clippy/Ruff/Pyright, the
  108-package dependency proof and fresh 6,526/6,526 candidate collection,
  collection SHA-256
  `3d7e92c62715dbc8dafb3d15cb9af030e0876c2e778e80fde9b13ac97cf95645`.
  The earlier ABI 48 base64-return candidate pass is retained only as a
  superseded diagnostic and is excluded from the final ledger.
- [ ] Continue the other 5,881 candidate Python rows and all 48,477 Core JS/TS
  invocations. This two-row Chromium checkpoint is not a full-suite claim.

NODE-275: Advance Python to ABI 47 and execute every expanded row from the
complete unchanged async HTTPS-error file without hiding provider gaps.

- [x] Preserve the ABI 46 diagnostic: the negative/default-TLS Chromium row
  passes, while the positive row stops at unsupported
  `Browser.new_context(ignore_https_errors=True)`, report SHA-256
  `cb5e9506dbf789968653f5525dd58f74689c26dae80110fa055e9c68efa69bd5`.
- [x] Add an independently selectable Rust `ignore-https-errors` leaf with a
  fixed 16-Context registry. Install `Security.setIgnoreCertificateErrors`
  on every owned Page/OOPIF session before publication and remove Context
  policy on close; Python forwards only the strict boolean option.
- [x] Run both Chromium invocations from unchanged
  `tests/async/test_ignore_https_errors.py`. ABI 47 candidate and independent
  oracle pass 2/2, with report SHA-256 values
  `cdd4eb40b5c8c40ede8c84f73056c5faf229837c497bf889d66545612510bea8`
  and
  `1f3179fc8f7deee06eb07ea775d238c9896bad297dd74f0afa1732732c4bef59`.
  The file's four Firefox/WebKit expansions remain explicit provider-gap
  `not-run` rows.
- [x] Update the cumulative candidate ledger to 636 passed, seven skipped and
  5,883 not run, SHA-256
  `690f4b86c509932323cd613f2f40fede63a06e86133209bf98944421ae3d70d8`.
  The cumulative oracle ledger is 617 passed, seven skipped and 5,902 not run,
  SHA-256
  `647b129c3901df6eb3337e7d886e88f8cbc7322653a71b38d7e99ac1c65bd204`.
- [x] Build two normalized-reproducible ABI 47 wheels with archive SHA-256
  `4074ac94f00f9463cf337e67af4b73671cf9d152ea1f6582735c9ad3265d49e7`
  and
  `5f4a2a2d71a0c6290344bd35b2b5ac2305e7653e21ce466d202e7dc584cdba7c`.
  Pass 83 installed Python units, 879 all-feature host units plus 26 explicit
  live/diagnostic ignores, two addon units, strict Clippy/Ruff/Pyright, the
  108-package dependency proof and a fresh 6,526/6,526 candidate collection.
- [ ] Continue the other 5,883 candidate Python rows and all 48,477 Core JS/TS
  invocations. This two-row Chromium checkpoint is not a full-suite claim.

NODE-274: Keep Python ABI 46 and execute the complete unchanged async/sync
fill and context-manager files in candidate and independent-oracle lanes.

- [x] Run all five Chromium invocations from unchanged `tests/async/test_fill.py`
  and `tests/sync/test_fill.py`. Candidate and oracle both pass 5/5 through the
  existing Rust-owned Locator fill capability.
- [x] Preserve the initial context-manager diagnostics: candidate fails 4/4
  before execution because Page/Context/Browser lack context-manager protocols;
  oracle passes 2/4 because the CDP launch adapter exposes its pre-attached
  default Context. Neither diagnostic is merged into the green ledger.
- [x] Add thin async/sync Page, BrowserContext and Browser context-manager
  protocols whose exit path calls the existing close owner. Make the oracle
  CDP adapter hide only Contexts that predate the adapted launch, symmetric
  with candidate fixture behavior. The four complete unchanged Chromium rows
  then pass 4/4 in both lanes.
- [x] Record the combined nine-row candidate/oracle tranche ledgers, SHA-256
  `8c727149b0e6c9b2b8712845a3a55dc81413a0888a4cd2799193246f8f95e0ab`
  and
  `db9950cb4d39a64ddc86e4adaf9fedf40f25202ef8852172e4811254debb3651`.
- [x] Update the cumulative candidate ledger to 634 passed, seven skipped and
  5,885 not run, SHA-256
  `8bb2f5b5437477779c6eb6690da2f247d3f377422f62b1d8bd3ee0cafb5f3908`.
  The cumulative oracle ledger is 615 passed, seven skipped and 5,904 not run,
  SHA-256
  `b02be1e0e0f789d575725098853e060d596514ddc7031fd2c0060883a53d080d`.
- [x] Build two normalized-reproducible ABI 46 wheels with archive SHA-256
  `6f2f3e4668083b081608d8ce92e1cdbe1f6040ea8d225f4bcfff44c9a62bc3ed`
  and
  `51c25e0d628d2dac7acf07c596c6815637000b86efba9056d6eac504d6b9096e`.
  Pass 81 installed Python units, strict Ruff/Pyright, the 108-package
  dependency proof and a fresh 6,526/6,526 candidate collection.
- [ ] Continue the other 5,885 unexecuted Python rows and all 48,477 Core
  JS/TS invocations. This nine-row checkpoint is not a full-suite claim.

NODE-273: Advance Python to ABI 46 and execute the complete unchanged async
semantic-selector file through native Locator collection evaluation.

- [x] Add one thin `NativeLocator.evaluate_all` export over the existing
  Rust-owned typed recipe and bounded collection-evaluation owner. Python no
  longer needs to recover a selector string from a native Locator.
- [x] Preserve Rust ownership of selector compilation, exact collection
  resolution, utility-world execution, FFI admission and bounded value
  conversion; Python forwards only the public function and serializable value.
- [x] Run all three Chromium invocations in unchanged
  `tests/async/test_selectors_get_by.py` in isolated candidate and independent
  oracle lanes. Both pass 3/3, including regex/exact escaping, role accessible
  names and `include_hidden`.
- [x] Record candidate/oracle report SHA-256 values
  `cc1b66557ae5510345f4dae1163f86a4dce8af8c344d81530586fc0f3fdddf12`
  and
  `c0150e1af71e0e262d2a4edf5c5bfa3463aaef0272a5f6bac4aa00e975b32d02`.
- [x] Update the cumulative candidate ledger to 625 passed, seven skipped and
  5,894 not run, SHA-256
  `39fc2c66a96066e5d0275b146688a48a7dbb1e8a3aeaf37f3971d598cbee4e4b`.
  The cumulative oracle ledger is 606 passed, seven skipped and 5,913 not run,
  SHA-256
  `3715fb13273109c65e780df7b8435752f488c2649e6e7ca9c23f64a7cf3eab42`.
- [x] Build two normalized-reproducible ABI 46 wheels with archive SHA-256
  values
  `a665627bdb6bdda5ce0e8e2a72a37999b222c10ffa5e8ba44637f0f52da52900`
  and
  `e7a4c8933d3167ad24fea61e3e6354fa06a0df8cb6ccbb44662cee467712b20d`.
- [x] Pass 79 installed Python units, 877 all-feature Rust host tests plus 26
  explicit live/diagnostic ignores, two addon units, strict release Clippy,
  Ruff/Pyright, the 108-package dependency proof and a fresh 6,526/6,526
  candidate collection.
- [ ] Continue the other 5,894 unexecuted Python rows and all 48,477 Core
  JS/TS invocations. This three-row checkpoint is not a full-suite claim.

NODE-272: Advance Python to ABI 45 and execute the complete unchanged official
sync semantic-Locator file without moving selector or assertion semantics into
Python.

- [x] Add Page, Frame, Locator and FrameLocator `get_by_text`, `get_by_label`,
  `get_by_placeholder`, `get_by_alt_text`, `get_by_title`, `get_by_test_id`
  and `get_by_role` facades over typed bounded Rust selector recipes.
- [x] Add Rust-owned `to_have_attribute` matching: total deadline, serial
  retries, missing-element handling, exact/regular-expression comparison and
  `ignore_case` precedence remain native. Python only converts public values
  and assertion results.
- [x] Run all ten Chromium invocations in the unchanged
  `tests/sync/test_locator_get_by.py` in isolated candidate and independent
  oracle lanes; both pass 10/10. The pinned upstream source has no async twin.
- [x] Record candidate report SHA-256
  `95362db9ab9b86874567fcee6bda9871f5c23e849005c0e9d03023ffc07abc73`,
  candidate tranche ledger SHA-256
  `e159fec1a40bf0ab0c157e4595890c6caab343cb0a5a2aee7e19bce8fc776932`
  and oracle tranche ledger SHA-256
  `2b68866af60f6f3fb9bb2f6e880f37214655b54dfaa1d28304031baa4bd2b20f`.
- [x] Update the cumulative candidate ledger to 622 passed, seven skipped and
  5,897 not run, SHA-256
  `e37434aa74c4dea2f6f8001ae27d657e833cf0c197207b0186b5d133b9a40813`.
  The independent oracle ledger is 603 passed, seven skipped and 5,916 not
  run, SHA-256
  `d164ca2a7dd060d16ced2277062a21cb60b695aa36df8320a551355e64ffee4d`.
- [x] Build two normalized-reproducible ABI 45 wheels with archive SHA-256
  values
  `e0321e22e0ab05b57608f48d180689009d6113a55525a0b81a7a5776e04b71b0`
  and
  `0c30faaf48699e0b95426204f8c28e3e868abb5e94b2dccc004050ba61ac098b`;
  payloads differ only in permitted CycloneDX build identity.
- [x] Pass 78 installed Python units, 877 all-feature Rust host tests plus 26
  explicit live/diagnostic ignores, two addon units, strict release Clippy,
  Ruff formatting/lint, strict Pyright, the 108-package dependency proof and
  a fresh 6,526/6,526 candidate collection.
- [ ] Continue the other 5,897 unexecuted Python rows and all 48,477 Core
  JS/TS invocations. This ten-row checkpoint is not a full-suite claim.

NODE-271: Advance Python to ABI 44 and close the complete 163-row official
Chromium Locator tranche without moving browser semantics into Python.

- [x] Add a lazy Rust-owned `HostFrameLocator` recipe. Every nested iframe
  owner remains an immutable bounded Locator; each operation re-resolves the
  current frame through a temporary ElementHandle, disposes that handle, and
  preserves the existing selector-depth and timeout bounds.
- [x] Export thin async/sync `Page`, `Frame`, `Locator` and `FrameLocator`
  facades for `frame_locator`, `content_frame`, `owner`, `locator`,
  `first`/`last`/`nth`, and visibility assertions. Existing-Locator re-rooting
  remains a typed Rust operation and strict errors use Playwright-shaped
  Locator call logs.
- [x] Match the official Python public-handler arity adapter. Browser `Event`
  values still use the ordinary Rust-owned binding serializer; the thin
  Python wrapper invokes callbacks with no more arguments than their declared
  signature. Keep callback-start publication and final-result delivery as
  separate bounded binding phases.
- [x] Run the final installed wheel against every unchanged Chromium row in
  `tests/async/test_locators.py` and `tests/sync/test_locators.py`: 86/86 plus
  77/77, 163/163 total. Report SHA-256 values are
  `49d736923082374a0d3eab81bea3cbaa539cf364105579856bc513c4cd2dde77`
  and
  `2cbb448b0651490286df3d7d559211ffac1b012527c42ff9176a3d72f6d07e02`.
  The independent official oracle is also 163/163.
- [x] Record the complete Locator ledger as 163 passed and 6,363 not run,
  SHA-256
  `9b99936aa66ed493f322ab6e44adf7e5bad788780593caf6b46cf49d5ae01a2e`.
  The cumulative 6,526-row ledger is 612 passed, seven skipped and 5,907 not
  run, SHA-256
  `89eabed04a3a7804637229ff077cf0898cc1aa903c190611bfb2d1553650aefe`.
- [x] Build two normalized-reproducible ABI 44 wheels with archive SHA-256
  values
  `bfdbcb48279fbc5377e84dbc2d40b101c186cbb74b25fade4582646db848e100`
  and
  `ef00f6f69fe0dfed04818d0580e9d2e564c6b75c2e08cb2526f7a7aee1c778af`;
  payloads differ only in permitted CycloneDX build identity.
- [x] Pass 74 installed Python units, 875 all-feature Rust host tests plus 26
  explicit live/diagnostic ignores, two addon units, strict release Clippy,
  Ruff formatting/lint, strict Pyright, and the 108-package dependency proof.
- [ ] Continue the other 5,907 unexecuted Python rows and all 48,477 Core
  JS/TS invocations. Locator completion is not a full-suite claim.

NODE-270: Advance Python to ABI 43 and compose the existing Rust-owned Locator
geometry and screenshot capabilities so the four viewport-dependent official
rows can complete end to end.

- [x] Export `Locator.bounding_box` as a thin async/sync facade over the
  existing Rust geometry owner; Python converts only the bounded result shape.
- [x] Install `locator-screenshot` as an independent host/addon feature. Rust
  owns selector resolution, stability, scrolling, iframe clip translation,
  capture admission and the 2 MiB output ceiling; Python only validates the
  public option shape and copies the already bounded result.
- [x] Keep screenshot admission finite at two concurrent captures and retain
  the existing 64-Page / 8 KiB viewport registry introduced in NODE-269.
- [x] Pass all four unchanged targeted async/sync Chromium viewport rows,
  report SHA-256
  `d38a756b260f5434bbd38c41956f1c8148facdd7c4c781f507a166b78075c61d`.
- [x] Rerun all 163 Locator rows in clean processes: 77/86 async and 68/77
  sync, 145/163 total, exactly four gains and no losses from NODE-268. Report
  SHA-256 values are
  `04cc14bf1adfd352d8e3eb5cf1139b9b8e82e239229be596bf7b4d8d69e74720`
  and
  `437120717f7314a7bdd65df31720c3bcb81f033989f2d749c62118e2dfc66f7e`.
- [x] Record the complete Locator ledger as 145 passed, 18 failed and 6,363
  not run, SHA-256
  `1276b628d8b778e7c947bb06fb5f32f43ff98ee064bce3e958e97ff1d88e6169`.
  The cumulative 6,526-row ledger is 594 passed, 18 failed, seven skipped and
  5,907 not run, SHA-256
  `811c14419d0fee688717168bfb0118d07cfaa82c27d9d8c2e0f14650d01348fe`.
- [x] Build two ABI 43 wheels with SHA-256 values
  `14924b545aaa14e9a4096897d59cc7cce45cd1fd8c3b5d07ef2ac3a7ef8b11da`
  and
  `fcf9958339d7f54ec190d85227f957d057abc0c253c6e5f52cf44cad1f852fa9`;
  normalized payloads differ only in permitted CycloneDX build identity.
- [x] Pass 66 installed Python units, 635 selected Rust host tests plus 24
  explicit live/diagnostic ignores, two addon units, strict
  Clippy/Ruff/Pyright and the 108-package dependency proof.
- [ ] Continue the remaining 18 Locator discrepancies: 16 FrameLocator rows
  and two focus/blur callback-completion timing rows. All 5,907 unexecuted
  Python rows and all 48,477 Core JS/TS invocations remain open; no full-suite
  claim is permitted.

NODE-269: Advance Python to ABI 42 for an independently selectable Rust-owned
Page viewport capability without claiming the downstream Locator rows early.

- [x] Add a fixed 64-Page / 8 KiB viewport registry, CDP window/metrics
  override and session-initialization replay that composes with device
  emulation.
- [x] Expose thin async/sync `Page.set_viewport_size` facades and publish the
  two viewport resource limits in immutable capability metadata.
- [x] Prove the leaf remains independently selectable and does not reference
  its resource metadata when disabled.
- [x] Build normalized-reproducible ABI 42 wheels with SHA-256 values
  `f17b710f3fe0ff90cc9a24a3fc7b4359218214b54462f1e96e2a4d10794ab840`
  and
  `ba9b0698c6ce62759ccd804ec1f605e0fc8648436918804115acedadbd2e6d74`;
  pass 63 installed Python units and the strict Rust/Python/addon gates.
- [x] Preserve the four-row official report as a red diagnostic: viewport
  mutation succeeds, then two rows reach missing `Locator.screenshot` and two
  reach missing `Locator.bounding_box`. NODE-270 closes those downstream gaps.

NODE-268: Keep Python ABI 41 and close four mirrored Locator discrepancy
families without widening the Rust capability graph.

- [x] Preserve Playwright's exact capture-plus-`nth` rejection in the bounded
  Rust selector recipe instead of sending an invalid capture selector to CDP.
- [x] Render role Locators and action timeouts with Playwright-shaped
  descriptions and `Locator.click` call-log prefixes while retaining bounded
  diagnostic storage and Unicode selector text.
- [x] Translate only the admitted simple relative XPath `./TAG` subset used by
  `has=` into a typed `:scope>TAG` query; keep broader XPath explicitly open.
- [x] Pass all eight targeted official async/sync Chromium rows, report
  SHA-256
  `e009754dd6d9cc27279875d84079f7ed72f1aa09ce53158433c0b7ea46e7140f`.
- [x] Rerun all 163 Locator rows in clean processes: 75/86 async and 66/77
  sync, 141/163 total, exactly eight gains and no losses from NODE-267. Report
  SHA-256 values are
  `c403694a37a45224c4cca636f0a733ba5713e6480cb45e6c1aa1e0c94e42da95`
  and
  `8e6c90f179b35c843860d4d9f501c552d27ee606cb104c3e7f1dae2502fa7e64`.
- [x] Preserve but exclude two targeted setup-only attempts: the first lacked
  Pillow and the second supplied an unsupported `channel` launch option. The
  clean replacement selects and passes all eight target rows.
- [x] Record the complete Locator ledger as 141 passed, 22 failed and 6,363
  not run, SHA-256
  `cf145b50fa1a45bc018d043058c92d37b783b788f442713a817ba9593f96de56`.
  The cumulative 6,526-row ledger is 590 passed, 22 failed, seven skipped and
  5,907 not run, SHA-256
  `6180ffa49de77b48b2a61d13a8fe3f159654c4e5c4529d841082377112d0ca46`.
- [x] Build two ABI 41 wheels with SHA-256 values
  `0036073c4cce16844e7c5cd35e3d437d472f52e57959741da1e150cba2e3e6d8`
  and
  `1e905da9fcc741493b93739c2dfe28ad315bd91821c751f34ad3bd7d0f601b69`;
  normalized payloads differ only in permitted CycloneDX build identity.
- [x] Pass 60 installed Python units, four selector-model units, two addon
  units, 554 affected Rust host tests plus 22 explicit live/diagnostic
  ignores, strict Clippy/Ruff/Pyright and the 108-package dependency proof.
- [ ] Continue the remaining 22 Locator discrepancies: four viewport rows,
  16 FrameLocator rows and two focus/blur callback-timing rows. All 5,907
  unexecuted Python rows and all 48,477 Core JS/TS invocations also remain
  open; no full-suite claim is permitted.

NODE-267: Advance Python to ABI 41 for independently selectable Locator DOM
actions and reuse the existing bounded Handle owner for `evaluate_handle`.

- [x] Add the separate `locator-dom-actions` feature for Rust-owned
  `dispatch_event` and `select_text`; its 108-package Pwright dependency
  closure is identical to the base addon closure.
- [x] Resolve `Locator.evaluate_handle` strictly in Rust, execute through the
  existing Handle leaf and return an owned native Handle with a bounded,
  Rust-computed preview. Python only adapts the public Handle wrapper.
- [x] Pass all six targeted official async/sync rows for `dispatch_event`,
  `select_text` and `evaluate_handle`, report SHA-256
  `3578853c90e57c8e7b15dbc36287d2ccf347a5dda26abf7a7590fc24373a3dc1`.
- [x] Rerun all 163 Locator rows: 71/86 async and 62/77 sync, 133/163 total,
  exactly six gains and no losses from NODE-266. The valid report SHA-256
  values are
  `7c231d815995b070495dc4f4f30c060b934e500f363e142a4368bf6977c66d2c`
  and
  `0c5aa8d72106042bfbbea686dde9a55910a2e404ba1d55bdf13b85e19cddbd10`.
- [x] Keep the first sync attempt as invalid infrastructure evidence after a
  shared browser target closed; restart the dedicated profile only after
  verifying no other browser task owned its two blank targets, then replace
  it with the clean 77-row sync report rather than counting setup errors as
  candidate failures.
- [x] Record the complete Locator ledger as 133 passed, 30 failed and 6,363
  not run, SHA-256
  `89154dea59026809606c087f49786fdbf9ba3bb708f9dc78a72349e379108292`.
  The cumulative 6,526-row ledger is 582 passed, 30 failed, seven skipped and
  5,907 not run, SHA-256
  `1feb3bb3ec44812ee930defa5471cd8e8454e2d8c49e9502059273f1c360fe41`.
- [x] Build two ABI 41 wheels with SHA-256 values
  `b5532c550883212610bf0991c9a555e021f0ef2862c882a5cb9f84c0d70b8665`
  and
  `ecfb2d1df595cd00a0be48d650c4ed893c9ab0eefa86fb3d76784fd429386d74`;
  normalized payloads differ only in permitted CycloneDX build identity.
- [x] Pass 54 Python units, two addon units, 551 affected Rust host tests plus
  22 explicit live/diagnostic ignores, strict Clippy/Ruff/Pyright and the
  dependency proof.
- [ ] Continue the remaining 30 Locator discrepancies, all 5,907 unexecuted
  Python rows and all 48,477 Core JS/TS invocations. No full-suite claim is
  permitted at this checkpoint.

NODE-266: Export the existing Rust-owned Locator highlight leaf through an
independently selectable Python ABI 40 feature and rerun every Locator row.

- [x] Add `locator-highlight` without widening the 108-package base-wheel
  dependency closure; Python forwards only bounded description metadata to
  `HostLocator::highlight`.
- [x] Pass both targeted official highlight rows and all 58 installed Python
  units, strict Ruff/Pyright/Clippy, 551 affected Rust tests plus 22 explicit
  live/diagnostic ignores, and two addon tests.
- [x] Preserve the interrupted 86-row async report after the dedicated Chrome
  process restarted. Rerun its seven setup-affected rows and merge them with
  the checked JUnit replacement tool; do not recast infrastructure errors as
  candidate outcomes.
- [x] Rerun all 163 Locator rows: 68/86 async and 59/77 sync, 127/163 total,
  exactly two gains and no losses from NODE-265. The complete Locator ledger
  SHA-256 is
  `074b438d329801fd69ac700e7da53571364b560a55f15af3e75116db430c2b9e`.
- [x] Replace the prior complete Locator reports in the cumulative 6,526-row
  ledger: 576 passed, 36 failed, seven skipped and 5,907 not run, SHA-256
  `6976549c14fa4000f16617a0226b53753f54580907a5294aa413eea1d40f2cce`.
- [x] Build two repaired ABI 40 wheels; normalized payloads differ only in the
  permitted CycloneDX build identity fields.
- [ ] Continue the remaining 36 Locator discrepancies, all 5,907 unexecuted
  Python rows and all 48,477 Core JS/TS invocations.

NODE-265: Advance to Python ABI 39 for double-click, bounded file input and
Locator element-handle publication.

- [x] Keep double-click options, filesystem/payload admission, selector
  resolution and handle lifecycle in Rust-owned capabilities.
- [x] Rerun all 163 Locator rows: 67/86 async and 58/77 sync, 125/163 total,
  exactly 11 gains and no losses from NODE-264.
- [x] Record the cumulative checkpoint as 574 passed, 38 failed, seven skipped
  and 5,907 not run, SHA-256
  `2356b169e48248f6609fcbd097d9f3205dd87ad2b26ec45a89eb7149a84b7d6e`.

NODE-264: Make visible-selector composition a bounded Rust recipe and rerun
the complete Locator files.

- [x] Parse `visible=true` and `visible=false` as `FilterVisible` nodes over
  the current recipe, including registered-engine chains; reject invalid or
  initial filter parts instead of treating them as CSS.
- [x] Advance the backend-owned selector runtime to version 9 and include an
  element root in user-facing relative engines so a filtered Locator can
  continue with `get_by_text` without Python-authored selector strings.
- [x] Pass the four targeted async/sync visible rows, then rerun all 163
  Locator rows: 62/86 async and 52/77 sync, 114/163 total, with exactly four
  gains and no losses from NODE-263.
- [x] Replace the prior complete Locator reports without dropping any earlier
  source tranche. The cumulative 6,526-row ledger is 563 passed, 49 failed,
  seven skipped and 5,907 not run, SHA-256
  `281d5cfa976077f38c3c49be2493171edf21e56c41c62b26036b8a44ded1a309`.
- [x] Pass strict release Clippy, 1,007 affected Rust tests plus 36 explicit
  live/diagnostic ignores, 53 Python units, two addon tests and normalized
  two-wheel reproducibility.
- [ ] Continue the remaining 49 Locator discrepancies, all 5,907 unexecuted
  Python rows and all 48,477 Core JS/TS invocations.

NODE-263: Advance to Python ABI 38 for Rust-owned role/test-id leaves and
single-Locator evaluation.

- [x] Keep `get_by_role`, `get_by_test_id` and `Locator.evaluate` in immutable
  native recipes and bounded Rust evaluation owners; Python only validates
  public option shape and converts values.
- [x] Rerun both complete Locator files: 60/86 async and 50/77 sync, 110/163
  total, with three gains and no losses from NODE-262.
- [x] Record the cumulative 6,526-row checkpoint as 559 passed, 53 failed,
  seven skipped and 5,907 not run, SHA-256
  `d966961d4b507e49bc8f7ead28fa133d490083a0cdf05d77b7b6877110b0462c`.

NODE-262: Advance to Python ABI 37 for Rust-owned Locator waits, text/property
expectations and the remaining typed filter/or composition paths.

- [x] Keep retry deadlines, state polling, text/property reads and regex flags
  in Rust-owned native Locator recipes.
- [x] Rerun both complete Locator files: 58/86 async and 49/77 sync, 107/163
  total, with 24 gains and no losses from NODE-261.

NODE-261: Advance to Python ABI 36 for typed Locator filter/has/has-not,
visible and boolean composition.

- [x] Reuse one native query budget for derived recipes and reject cross-Page
  or cross-Frame combinations before browser work.
- [x] Rerun both complete Locator files: 45/86 async and 38/77 sync, 83/163
  total, with four gains and no losses from NODE-260.

NODE-260: Stabilize attached-browser network ownership and publish the first
complete-file Locator repair checkpoint.

- [x] Stop eager request-history/response-event activation for default or
  external Contexts borrowed through CDP attach. Keep exact Page waiters
  demand-driven, keep newly created owned Contexts active, and preserve eager
  activation for the default Context of a Rust-owned browser process.
- [x] Preserve the externally interrupted 50/86 attempt as
  `aborted-external-cdp-loss`; do not merge it into conformance evidence.
- [x] Re-run every async and sync Locator row in a separate pytest process.
  Replace the one async setup race with its isolated rerun, yielding stable
  candidate results of 43/86 async and 36/77 sync, with zero infrastructure
  errors in the merged evidence.
- [x] Publish the complete 6,526-row candidate ledger as 528 passed, 84
  failed, seven skipped and 5,907 not run, SHA-256
  `b617aeabdd66aa3f0f25b9d318d1aea3bb876e576b4ac7c134fd3f86c89d8216`.
- [x] Pass 47/47 Python units, 270/270 Node adapter tests, strict Rust Clippy,
  Ruff/Pyright, normalized two-wheel reproducibility and the complete 6,526
  collection.
- [ ] Add demand-driven response-listener/history activation for borrowed
  default/external Contexts before claiming that attached pre-existing Pages
  retain the eager ABI 27/29 behavior.
- [ ] Continue the remaining 84 Locator discrepancies and all unexecuted
  Python and Core JS/TS rows.

[Complete Locator diagnostic and repair evidence](python-official-conformance.md#complete-locator-diagnostic-and-repair-checkpoint-2026-09-28).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-259: Repair the first full-file Locator discrepancy cluster without
moving browser semantics into Python.

- [x] Route immutable native Locator owners through click, fill, press,
  checked, focus, blur, hover, clear, single-string select-option,
  scroll-into-view and sequential typing actions.
- [x] Add bounded `Locator.all()`, `Frame.locator()`, Python repr/str facades,
  and Playwright-shaped strict-mode wording while keeping recipe/action
  execution in Rust.
- [x] Convert 19 official rows from failure to pass: ten async and nine sync.
  Preserve every remaining failure instead of shrinking the selected files.
- [ ] Implement the next clusters: filter/has/has-not and regex text,
  FrameLocator/content-frame/owner, ElementHandle/evaluateHandle, remaining
  actions and assertions.

NODE-258: Execute the complete unchanged Python Locator files before selecting
another implementation slice.

- [x] Run all 86 async plus 77 sync Chromium invocations against the ABI 35
  candidate: 60 passed and 103 failed.
- [x] Run the same complete files through the independent official Playwright
  lane: 163/163 passed, proving every candidate failure is a discrepancy rather
  than an upstream/test-server failure.
- [x] Preserve complete candidate/oracle reports and replace prior selected
  Locator rows in full ledgers without losing the mixed sync Page drag row.
- [x] Use the failure clusters, not ad-hoc green filtering, to order NODE-259.

NODE-257: Export typed Rust-owned Locator recipes to Python and verify the
first composition slice.

- [x] Pin exactly four async and two sync upstream Chromium invocations in
  `locator-composition-manifest.json`, SHA-256
  `78e5b01d4582222168c51fea88add9a89987374914de5bac4711e2a7e4d4755f`.
- [x] Preserve the ABI 34 candidate's 4/4 async plus 2/2 sync missing-member
  failures outside the green ledger; run the same six rows in the independent
  oracle.
- [x] Advance ABI to 35; make root Locator creation, within composition and
  nth derivation retain immutable `HostLocator` recipes under Rust's existing
  query budget. Keep Python free of selector-chain encoding and polling state.
- [x] Pass final candidate and oracle 4/4 async plus 2/2 sync, installed unit
  tests 39/39, Node adapter tests 270/270, strict Rust/Python gates, dependency
  proof, normalized wheel reproducibility and full 6,526 collection.
- [x] Prove exactly six `not-run` to `passed` transitions in each ledger:
  candidate 505/7/6014, SHA-256
  `791513dec404b04ed5521aa031524c418703dc78f2f59adced1016c0efed07a7`;
  oracle 486/7/6033, SHA-256
  `efdd8cd287445c9dadd84942ebd64f4e2bb9f565f06928bda3c4eeaa3423d775`.
- [ ] Continue remaining Locator strictness, assertions and actions, all 6,014
  candidate Python rows and the complete Core JS/TS corpus.

[Python ABI 35 typed Locator evidence](python-ffi-compatibility-design.md#abi-35-typed-locator-composition-checkpoint-2026-09-28).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-256: Close the async Python Locator description metadata slice without
inventing browser or Rust state.

- [x] Pin exactly the three upstream async Chromium none/simple/special-
  character description invocations in `locator-description-manifest.json`,
  SHA-256
  `d578cddad1cbb56d29d098b4fb1c19a20bd1d24a739809a25bdacb03d6591374`.
- [x] Preserve the initial three `AttributeError` candidate failures and the
  independent oracle's 3/3 report.
- [x] Keep description identity immutable and Python-facade-owned; add no
  browser command, retained native value, ABI, feature, package or dependency
  edge.
- [x] Pass final candidate 3/3, installed unit tests 38/38, strict
  Ruff/Pyright, normalized wheel reproducibility and full 6,526 collection.
- [x] Prove exactly three `not-run` to `passed` transitions in each ledger:
  candidate 499/7/6020, SHA-256
  `c5e89f60ce3e252855f80dd5ca8bcd9444cb089f62b61cb60eee04586d29cc67`;
  oracle 480/7/6039, SHA-256
  `100d1c8ef02411cbfd506042600f9b7c90ea54f53b74478c571fc276a7a151f6`.
- [ ] Add Rust-owned typed Locator chaining before selecting the two chained-
  description rows; continue all 6,020 candidate Python rows and the Core
  JS/TS corpus.

[Python ABI 34 Locator description evidence](python-ffi-compatibility-design.md#abi-34-locator-description-checkpoint-2026-09-28).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-255: Execute the first official Locator click/evaluate-all/count slice and
keep count-expectation retry ownership in Rust.

- [x] Pin the unchanged async/sync `test_locators.py` source hashes and exactly
  ten Chromium invocations in `locator-execution-manifest.json`, SHA-256
  `ed03f8d21b9d27cd400b3a3cb98203a20a35fdc4c7f57aa6e03ec00c92411e09`.
- [x] Preserve the invalid `--browser-channel` setup diagnostic and the 4/5
  candidate report exposing the placeholder assertion outside the green
  ledger; neither is behavior-passing evidence.
- [x] Add typed Rust count comparison, total-deadline retry and bounded 10 ms
  sleep; Python and Node forward expected count, timeout and negation only.
- [x] Advance Python ABI 33 to 34 for `locator_expect_count`; add no capability
  feature, package or dependency edge.
- [x] Pass 5/5 async plus 5/5 sync in both installed-wheel candidate and
  independent official oracle lanes, 36/36 Python unit tests and 270/270 Node
  adapter tests.
- [x] Build two repaired wheels with reproducible normalized payloads, collect
  all 6,526 Python rows, and prove exactly ten `not-run` to `passed`
  transitions per ledger: candidate 496/7/6023, SHA-256
  `0562df16dd47427a0aa6bd6e6071b833c4a29dc88c5998d8e88aff5a23276649`;
  oracle 477/7/6042, SHA-256
  `14912795ba9cbb4efc8a6c3713506dc039094c497ab0f14806f5cd8ee160ba8b`.
- [ ] Continue remaining Locator composition, strictness, assertions,
  actionability and providers; 6,023 candidate Python rows and all unexecuted
  Core JS/TS rows remain open.

[Python ABI 34 Locator expectation evidence](python-ffi-compatibility-design.md#abi-34-locator-execution-and-count-expectation-checkpoint-2026-09-28).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-254: Close the basic async/sync Locator facade slice without moving
browser behavior out of Rust.

- [x] Pin unchanged async/sync `test_locators.py` sources and exactly 34
  Chromium read/state/collection/fill/check/focus/press/evaluate invocations in
  `locator-basic-manifest.json`.
- [x] Preserve the initial 34 missing-method failures and the intermediate
  16/17 report outside the green ledger; the latter exposed missing Rust
  Playwright `:text("...")` parsing.
- [x] Add thin async/sync Locator methods that only forward selector, options
  and values to existing Rust `LocatorRead`, action and evaluation owners.
- [x] Parse quoted `:text()` into a typed exact text leaf in the bounded Rust
  query recipe; keep selector semantics out of Python.
- [x] Correct the NODE-253 native capability handshake so it explicitly lists
  `layout-selectors`, 256 parts and 256 KiB; keep ABI 33 because no native
  method changed.
- [x] Pass 34/34 final candidate and 34/34 independent oracle, 34/34 installed
  unit tests, normalized wheel reproducibility and full 6,526 collection.
- [x] Prove exactly 34 `not-run` to `passed` transitions in each ledger:
  candidate 486/7/6033, SHA-256
  `2a956ea3e610f1a8ddad4b92cce0dde918a86aaa3e46a0eb915a5525cc925437`;
  oracle 467/7/6052, SHA-256
  `7a5412b7212fdf7c4e3a6828d411febc65e922dc4be5bfa046210e67e2b3563b`.
- [ ] Continue the remaining Locator composition/strictness/actionability APIs,
  Firefox/WebKit, 6,033 candidate `not-run` rows and the incomplete 48,477-row
  Core JS/TS corpus.

[Python ABI 33 basic Locator evidence](python-ffi-compatibility-design.md#abi-33-basic-locator-facade-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-253: Add bounded layout-selector execution and close the remaining
Chromium layout rows in async/sync `test_queryselector.py`.

- [x] Pin unchanged async/sync sources and exactly the two
  `test_should_work_with_layout_selectors[chromium]` invocations in
  `selector-layout-manifest.json`.
- [x] Preserve the initial candidate `selector script threw` diagnostics
  outside the green ledger; run final installed-wheel candidate and independent
  official-oracle lanes unchanged, 2/2 in each.
- [x] Add independently selectable `layout-selectors` host/addon features with
  no new Pwright package edge. Keep parsing, clause validation, geometry score
  formulas and result ordering in Rust-generated bounded browser programs.
- [x] Bound admission to the existing 64 KiB selector source ceiling, at most
  256 structural parts and a 256 KiB emitted program; support the official
  left/right/above/below/near forms and tested top-level `+` composition.
- [x] Keep ABI 33: the existing internal native browser-program count export
  serves both registered and layout plans; Python gains no parser or geometry
  implementation.
- [x] Build two normalized-payload-reproducible wheels, pass 33/33 installed
  unit tests, strict Ruff/Pyright/Clippy, feature-cold compilation, dependency
  proof and full 6,526/6,526 collection.
- [x] Rebuild unique ledgers and prove exactly two outcome transitions:
  candidate 452/7/6067, SHA-256
  `295722cd54450259098ff79cd59a5f47cb59926f2d4403f8292d47f481f373df`;
  oracle 433/7/6086, SHA-256
  `263f3100d25d77d1de6b705e6b4cf152ac439857da9f5caa769f1bf055ed6a98`.
- [ ] Generalize arbitrary nested CSS/custom-pseudo layout grammar, add
  Playwright retry polling and positive `Page.query_selector` handle
  publication; continue Firefox/WebKit, all 6,067 candidate `not-run` rows and
  the incomplete 48,477-row Core JS/TS corpus.

[Python ABI 33 layout evidence](python-ffi-compatibility-design.md#abi-33-layout-selector-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-252: Execute registered selector engines through a bounded Rust-owned
program and close the first six official custom-engine rows.

- [x] Pin unchanged async/sync `test_queryselector.py` sources at SHA-256
  `da8882e3b969ad83a159bec555c05971939d6b419e4c5b9be16165b60aa28708`
  and `6799dd725cfb7c99a1e90ba0cfc5dc4a83943a9fb673a22a4fb1f00f78f9bda1`.
- [x] Fix exactly the base registration function plus the path and
  main/isolated-world functions in two manifests; run all six Chromium
  expansions unchanged in candidate and independent-oracle lanes.
- [x] Keep registry and execution ownership in Rust: at most 64 engines,
  64-byte names, 1 MiB per source, 8 MiB accounted name/source payload plus
  fixed 64-entry structural overhead, 256 selector parts, and a 2 MiB emitted
  program ceiling. Install each distinct referenced engine once.
- [x] Preserve the ABI 32 unknown-engine diagnostics and the ABI 33 strictness
  and final-custom-part `queryAll` diagnostics outside the green ledger.
- [x] Route `$eval`-shaped single lookup through the custom engine's `query`,
  `$$eval` through `queryAll`, and choose main versus utility world from
  `content_script` without adding a parser or JavaScript runtime to Python.
- [x] Build two normalized-payload-reproducible ABI 33 wheels, pass 33/33
  installed unit tests, strict Ruff/Pyright/Clippy, feature-cold compilation,
  dependency proof and full 6,526/6,526 collection.
- [x] Rebuild unique ledgers: candidate 450/7/6069, SHA-256
  `52fe9d0c64ecf31fb34098e468053265f79eddd68a8cb29dcddab2f75e45c675`;
  oracle 431/7/6088, SHA-256
  `e38018a3c4207b9fdb0995bf8b121d2c455510c49a9440275b2c329cefde8bd3`.
- [ ] Add positive registered `Page.query_selector` handle publication,
  typed Locator filters/composition over registered plans and retry polling;
  continue Firefox/WebKit, all 6,069 candidate `not-run` rows and the
  incomplete 48,477-row Core JS/TS corpus.

[Python ABI 33 custom-engine evidence](python-ffi-compatibility-design.md#abi-33-registered-selector-engine-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-251: Route Python query-selector validation through the existing Rust
selector parser and close the official async/sync registration-error rows.

- [x] Pin unchanged async/sync `test_queryselector.py` sources at SHA-256
  `da8882e3b969ad83a159bec555c05971939d6b419e4c5b9be16165b60aa28708`
  and `6799dd725cfb7c99a1e90ba0cfc5dc4a83943a9fb673a22a4fb1f00f78f9bda1`.
- [x] Preserve the candidate diagnostic showing DOM `querySelector` leaked a
  CSS `SyntaxError` for `neverregister=ignored`; do not merge it as green.
- [x] Add ABI 32 native `validate_selector` over the existing bounded Rust
  `LocatorQuery`; Python forwards Frame ID and selector and performs no parser
  or error-string reconstruction.
- [x] Pass the unchanged Chromium rows 2/2 candidate and 2/2 independent
  oracle while keeping four Firefox/WebKit expansions `not-run`.
- [x] Build two reproducible-payload ABI 32 wheels, pass 32/32 installed unit
  tests, strict Ruff/Pyright/Clippy, feature-cold compilation, dependency proof
  and full 6,526/6,526 collection.
- [x] Rebuild unique ledgers: candidate 444/7/6075, SHA-256
  `7b3fb579c03cf79a5869613890e06506903142ae8e4627de877e2f8bc4f0d6f3`;
  oracle 425/7/6094, SHA-256
  `89d888ddfe687e446614a35d5a24467e7b3123678242d1191129d93fb0d9b727`.
- [ ] Implement registered-engine browser execution and continue the 6,075
  candidate `not-run` rows plus the incomplete Core JS/TS corpus.

[Python ABI 32 selector-validation evidence](python-ffi-compatibility-design.md#abi-32-selector-validation-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-250: Add independently selectable, bounded global selector registration
through Python ABI 31 and close the first official registration discrepancy.

- [x] Pin the unchanged `tests/async/test_selectors_misc.py` source at SHA-256
  `44746147b5adaf40bdd879468b34d9fb2d31b4c64d3a397c9c94a47961b41b0f`
  and select exactly the Chromium duplicate-registration invocation.
- [x] Keep registration ownership in Rust: 64 engines, 64-byte names, 1 MiB
  per source and 8 MiB total, with duplicate/predefined/invalid-name errors and
  explicit teardown release. Python retains no registry and bounds `path=`
  reads before copying into the native call.
- [x] Keep `selector-registration` independently composable: feature-cold,
  feature-selected and exact wheel profiles compile; the dependency proof
  remains at the same 108-Pwright-package base closure.
- [x] Preserve the initial ABI 30 `Playwright.selectors` setup failure and the
  first ABI 31 shim-collection failure outside the green ledger, then pass the
  unchanged final-wheel candidate 1/1 and independent oracle 1/1.
- [x] Build two reproducible-payload ABI 31 wheels, pass 32/32 installed unit
  tests, strict Ruff/Pyright/Clippy, four Rust registry tests and full
  6,526/6,526 collection.
- [x] Rebuild unique ledgers: candidate 442/7/6077, SHA-256
  `80a4c3855a5a064ef26c467eae02b50b960e93e0faff250fda1693aa8f0a1445`;
  oracle 423/7/6096, SHA-256
  `cddd8fff7571cd00a132fc91ac1c3a722d9bbf4aba1807efc1c55390a788ab7b`.
- [ ] Implement and differentially verify custom selector-engine execution;
  continue all 6,077 explicitly not-run candidate invocations and the
  incomplete 48,477-row Core JS/TS corpus.

[Python ABI 31 selector-registration evidence](python-ffi-compatibility-design.md#abi-31-selector-registration-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-249: Prove the async binding of the ABI 30 collection selector path with
the exact upstream test rather than inferring parity from the sync result.

- [x] Pin the complete unchanged `tests/async/test_selectors_misc.py` source at
  SHA-256
  `44746147b5adaf40bdd879468b34d9fb2d31b4c64d3a397c9c94a47961b41b0f`.
- [x] Run the exact Chromium invocation unchanged through the installed ABI 30
  candidate wheel and the independent official oracle; both pass 1/1.
- [x] Keep Firefox/WebKit expansions as explicit `not-run` provider gaps.
- [x] Rebuild unique 6,526-row ledgers: candidate 441/7/6078, SHA-256
  `39d6bda545c2f7ce3a21252ee63903b8624732de1fbb29aaaaab6133259d0c86`;
  oracle 422/7/6097, SHA-256
  `318ff5c4cfdb01fdeb833c027fed4ead7765dc32fad7d881f8512a01f78e7be8`.
- [ ] Continue all 6,078 explicitly not-run candidate invocations and the
  incomplete Core JS/TS corpus.

[Python ABI 30 async selector evidence](python-ffi-compatibility-design.md#abi-30-async-selector-regression-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-248: Add bounded Rust-owned collection selector evaluation through Python
ABI 30 and close the first official `internal:and` discrepancy.

- [x] Pin the complete unchanged `tests/sync/test_selectors_misc.py` source at
  SHA-256
  `940c7576586bc22381f260de40349d8ac3632ddb05ff0415b1af5ed3beeaf532`.
- [x] Keep Python thin: async/sync `Page.eval_on_selector_all` forwards Frame
  identity, selector, function source and argument to the existing Rust
  locator collection-evaluation owner.
- [x] Parse `internal:and=<JSON selector>` into typed `Selector::And` and
  preserve bounded selector recipe admission; reject malformed or nested-chain
  payloads rather than silently interpreting them as CSS.
- [x] Decouple the CDP event item bound from aggregate retained bytes: admit
  8,192 event items while retaining the prior 512 MiB payload ceiling plus one
  maximum-size ingress overlap.
- [x] Keep `network-events + concurrent-close` composable: native-close
  selection takes the maximum of both declared runtime object capacities
  instead of replacing the 1,088-object network bound with 256.
- [x] Run the exact applicable Chromium invocation unchanged: installed-wheel
  candidate 1/1 and independent oracle 1/1. Keep Firefox/WebKit rows explicitly
  `not-run` because those providers do not exist yet.
- [x] Build two reproducible-payload ABI 30 wheels, pass 29/29 installed unit
  tests, pass async/sync listener regressions, and recollect 6,526/6,526.
- [x] Rebuild unique 6,526-row ledgers: candidate 440/7/6079, SHA-256
  `65bd9d4261400421fa079ceea93780052d25c11b19e92a3ad8d2ea25fa75862b`;
  oracle 421/7/6098, SHA-256
  `c4f10b1c972140a3414e48902f610e4a99f8b18384daac925bad89bd9e6db116`.
- [ ] Continue all 6,079 explicitly not-run candidate invocations and the
  incomplete Core JS/TS corpus.

[Python ABI 30 selector evidence](python-ffi-compatibility-design.md#abi-30-collection-selector-evaluation-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-247: Complete Python ABI 29 response-listener coverage for the official
sync binding and harden bounded reconnect recovery.

- [x] Pin the complete unchanged `tests/sync/test_listeners.py` source at
  SHA-256
  `24c14d60da2cc007ca29c68de83982644b3a82320249145953e0a7125fa88658`.
- [x] Run its applicable Chromium invocation unchanged: candidate 1/1 and
  independent oracle 1/1. Preserve the full-file Firefox/WebKit provider
  errors outside the green ledger.
- [x] Preserve live network subscription lanes while an authoritative
  reconnect snapshot is incomplete, then reconcile adapter sessions and
  release only Pages absent from the completed snapshot.
- [x] Keep recovery single-shot and bounded: one authoritative reconstruction,
  one epoch advance, and a 2,048-entry CDP replay window sized for the declared
  16-Page/128-session/256-world profile.
- [x] Build two reproducible-payload final wheels, pass 28/28 installed tests,
  and recollect 6,526/6,526 from the installed wheel.
- [x] Re-run the async listener regression 1/1 on the same final wheel and pass
  the topology, network-lane, Rust Clippy, dependency, Ruff and Pyright gates.
- [x] Rebuild unique 6,526-row ledgers: candidate 439/7/6080, SHA-256
  `4da01fe20cf788ee1a12f28509708166ed31b5f3eb3cefd78672ba4cddc3ffee`;
  oracle 420/7/6099, SHA-256
  `f654d55f1bb21a0382a2bca2636438070a0c14e3b92cdcd1d7c2e643b6f9ea90`.
- [ ] Continue all 6,080 explicitly not-run candidate invocations and the
  incomplete Core JS/TS corpus.

[Python ABI 29 sync evidence](python-ffi-compatibility-design.md#abi-29-sync-listener-completion-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-246: Add bounded Rust-owned Page `response` listener publication through
Python ABI 29 without folding callbacks into `network-events`.

- [x] Add an independent `response-events` feature with exactly the existing
  `network-events` Pwright package closure.
- [x] Activate response interest before Context publication and preserve the
  shared subscription when `request-history` is selected at the same time.
- [x] Bound response publication at 16 Contexts, 256 queued notices and the
  existing shared 16 MiB network byte budget; release notices on Page/session
  retirement and wake blocked consumers on close.
- [x] Keep Python thin: one watcher converts Rust `(page_id, response)` notices
  and preserves `on`/`remove_listener` callback identity.
- [x] Recover sticky topology event loss once, with an authoritative
  same-connection snapshot and exactly one monotonic epoch advance; keep
  materialized Page membership independent of transient command-session
  routing during reconnect.
- [x] Pin the complete unchanged `tests/async/test_listeners.py` source and run
  its applicable Chromium invocation: candidate 1/1 and oracle 1/1. Keep the
  Firefox/WebKit provider errors explicit rather than converting them to skips.
- [x] Build two reproducible-payload final wheels, pass 28/28 installed tests,
  and recollect 6,526/6,526 from the installed wheel.
- [x] Rebuild unique 6,526-row ledgers: candidate 438/7/6081, SHA-256
  `382e3c5a4fa98f8bd22e9503066d21a0236ded7f7f823a10b8e7ee3d17e439c2`;
  oracle 419/7/6100, SHA-256
  `a0d41d0fe578361d80998cc60cdc25b7c07edb44e455061eccf64640785ec13f`.
- [ ] Continue all 6,081 explicitly not-run candidate invocations and the
  incomplete Core JS/TS corpus.

[Python ABI 29 evidence](python-ffi-compatibility-design.md#abi-29-response-listener-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-245: Add the complete Playwright v1.58.0 device catalog and bounded
Context-scoped device emulation through Python ABI 28.

- [x] Embed all 143 descriptors as immutable Rust package data, with semantic
  equality to upstream source SHA-256
  `ceb47c6d0366b273151121ade0217f1e96a4043594c8188fcc12c0c9b30f9353`.
- [x] Keep `device-emulation` independently selectable and dependency-neutral;
  add no Node driver, bundled browser or Python protocol implementation.
- [x] Bound settings at 16 live Contexts and 64 KiB, validate dimensions and
  user-agent bytes before retention, apply them before Page publication, and
  release the exact Context lease on close or failed creation.
- [x] Add thin async/sync `Playwright.devices`, browser-type lookup and
  `Browser.new_context(**device)` adaptation while Rust owns all CDP behavior.
- [x] Pin and run the complete unchanged official device-descriptor file:
  candidate and independent oracle each record one Chromium pass plus the same
  two upstream Firefox/WebKit skips.
- [x] Build two reproducible-payload final wheels, pass 26/26 installed tests,
  and recollect 6,526/6,526 from the installed wheel.
- [x] Rebuild unique 6,526-row ledgers: candidate 437/7/6082, SHA-256
  `c02c13e38b6b3dae4d83ec8ad280088dabd7432c30b70ecfeb83e83bf9e5cfc3`;
  oracle 418/7/6101, SHA-256
  `a60219448e4b669fd5c3520186e93579b2e3fea6692492d483c0845a7fd3c644`.
- [ ] Continue all 6,082 explicitly not-run candidate invocations and the
  incomplete Core JS/TS corpus.

[Python ABI 28 evidence](python-ffi-compatibility-design.md#abi-28-device-descriptor-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-244: Add bounded Rust-owned `Page.requests()` history through Python ABI
27 and close both official Chromium bindings.

- [x] Keep `request-history` independently selectable while reusing the exact
  `network-events` package closure and all four normalized request lifecycle
  tags; add no Python protocol client or Node driver.
- [x] Retain the newest 100 Requests per Page in Rust across 16 Page slots,
  while preserving user-held evictions through a separately bounded 256
  owners per Page, 4,096 owners globally and 64 MiB event-byte budget.
- [x] Activate history at BrowserContext scope before future Page publication,
  refresh the empty owned-Context route snapshot, and compensate partial
  subscription activation exactly.
- [x] Keep `Request.response()` and `Response.text()` usable for all 199
  retained Requests under a 256-entry observation lane with actual body-byte
  charging; keep the unrelated general command capacity at 32.
- [x] Add thin async and sync `Page.requests`, `Request.response`,
  `Response.text`, and string `Page.route`/`Route.fulfill` facades; Rust keeps
  request, response, body and route authority.
- [x] Pin and pass the unchanged async/sync Chromium last-100 rows: candidate
  2/2 and independent oracle 2/2.
- [x] Build two reproducible-payload final wheels, pass 25/25 installed tests,
  and recollect 6,526/6,526 from the installed wheel.
- [x] Rebuild unique 6,526-row ledgers: candidate 436/5/6085, SHA-256
  `2fb546a564e528dfc3788466aae16bb6fc556163e950eff2f26df2bd90e4042e`;
  oracle 417/5/6104, SHA-256
  `9ba7a572ab3c66a18b0c409ba09ac4b32c23db36f84d9c73c3be6db7b6040a4c`.
- [ ] Continue all 6,085 explicitly not-run candidate invocations and the
  incomplete Core JS/TS corpus.

[Python ABI 27 evidence](python-ffi-compatibility-design.md#abi-27-request-history-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-243: Add independently selectable, bounded Rust-owned
`Page.console_messages()` history through Python ABI 26.

- [x] Record the newest 200 Chromium console messages per Page in Rust, with
  16 preallocated Page slots, 3,200 global items, a 16 MiB byte budget,
  oldest-record eviction and exact Page/session teardown.
- [x] Keep `console-history` separate from `console-events`: both reuse the
  same CDP observation/parser, while installing history does not install
  callback publication or a Python-owned event queue.
- [x] Add thin async and sync `Page.console_messages()` facades and public
  value wrappers. `type`, `text`, `location`, `page`, and `worker` are exposed;
  retained argument `JSHandle`s remain explicitly unsupported.
- [x] Pin and pass the unchanged async/sync Chromium newest-200 rows:
  candidate 2/2 and independent oracle 2/2.
- [x] Build two reproducible-payload final wheels, pass 23/23 installed tests
  and recollect 6,526/6,526 from the installed wheel.
- [x] Rebuild unique 6,526-row ledgers: candidate 434/5/6087, SHA-256
  `9445e7a0c6d424cee6b604a8fc43804ddb4304ab4d97f1eac7371e3398e4d41e`;
  oracle 415/5/6106, SHA-256
  `354fadad72ee68e599644973aa648e941c4619499f3b0a12b30049441a080773`.
- [ ] Continue all 6,087 explicitly not-run candidate invocations and the
  incomplete Core JS/TS corpus.

[Python ABI 26 evidence](python-ffi-compatibility-design.md#abi-26-console-message-history-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-242: Add bounded Rust-owned `Page.page_errors()` history through Python
ABI 25 and close both official Chromium bindings.

- [x] Retain the newest 200 Page errors in Rust, with 16 preallocated Page
  slots, 3,200 global items, 16 MiB of error bytes, fixed-storage accounting,
  oldest-record eviction and Page/session teardown.
- [x] Raise the selected Page-error lifecycle publication/revision capacity to
  512 so the unchanged 301-error sync test cannot exhaust the prior 256-entry
  queue before its watcher drains.
- [x] Add thin async and sync `Page.page_errors()` facades, the exact native
  stub, ABI/type assertions and an AAA delegation/conversion test.
- [x] Pin and pass the unchanged async/sync Chromium history rows: candidate
  2/2 and independent oracle 2/2.
- [x] Build two reproducible final wheels, pass 22/22 installed tests and
  recollect 6,526/6,526 from the installed wheel.
- [x] Rebuild unique 6,526-row ledgers: candidate 432/5/6089, SHA-256
  `6c3d2dad4271da574f924a305cadcb8aeb801e4650079249e524df02be0e1293`;
  oracle 413/5/6108, SHA-256
  `59aad3d302b5c05d06a3a3fe8898ee881076658ab8b0bb9edc5de2ef91e5e00d`.
- [ ] Continue all 6,089 explicitly not-run candidate invocations and the
  incomplete Core JS/TS corpus.

[Python ABI 25 evidence](python-ffi-compatibility-design.md#abi-25-page-error-history-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-241: Expose the existing Rust Page garbage-collection leaf through Python
ABI 24 and close both official Chromium bindings.

- [x] Add an independently selectable `request-gc` addon feature that forwards
  only the bounded Page ID to `pwright-node-host/request-gc`; add no Pwright
  package, Python protocol client, task, queue, timer, or retained state.
- [x] Add thin async and sync `Page.request_gc()` facades, an exact native stub,
  ABI/profile assertions, and an AAA delegation test.
- [x] Pin the unchanged async and sync upstream files and exact functions in
  `page-request-gc-manifest.json`.
- [x] Pass 1/1 async and 1/1 sync in both repaired-wheel candidate and
  independent official-oracle Chromium lanes.
- [x] Build two final wheels with reproducible payloads, pass 21/21 installed
  tests, and recollect 6,526/6,526 from the installed wheel.
- [x] Rebuild the ledgers without duplicate rows: candidate is 430/5/6091,
  SHA-256
  `fce03db46415b6bed7e1d97484aec3abbcbfa9b3fa051a52d1ef73eccf659241`;
  oracle is 411/5/6110, SHA-256
  `d0bf59a7f917f8c19d6be9064ebc41e295825d8825459e4fda29e7b717f3fb71`.
- [ ] Continue all 6,091 explicitly not-run candidate invocations and the
  incomplete Core JS/TS corpus.

[Python ABI 24 evidence](python-ffi-compatibility-design.md#abi-24-page-request-gc-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-240: Repair terminal Page-route settlement and execute the complete
official Python Page shard in isolated candidate and oracle lanes.

- [x] Preserve a bounded terminal receipt when Page retirement removes a
  pending intercepted request, so the first later `continue_`, `abort`, or
  `fulfill` is a successful no-op and a duplicate action still fails.
- [x] Account the fixed 64-receipt capacity in the Rust resource plan and cover
  retirement plus single-consumption behavior with focused AAA tests.
- [x] Run all 125 expanded `tests/async/test_page.py[chromium]` rows, one pytest
  process per row, against the repaired installed wheel and an independent
  official package on the same task-owned headless Chrome.
- [x] Record candidate 124 passed / one upstream FIXME skip and oracle 124
  passed / the same skip, with zero per-test status differences. Candidate
  report SHA-256 is
  `709000c785522a979b7f56b344216a9ed0a4f8b9d783ffc725dea8e15c9ba9f3`;
  oracle report SHA-256 is
  `d8058d713c4cc1fcefbbab295b4eb9aa5647dc98eb9f8453a8e877319be7dade`.
- [x] Rebuild both 6,526-row ledgers without duplicate rows: candidate is
  428/5/6093, SHA-256
  `a8b7ce8ed00ed75676cad32d68c6487685839b8fae29ba8f1b79e5cbd0689980`;
  oracle is 409/5/6112, SHA-256
  `c0858c03e07687bd7786d5f713b548fcbc763b9028b41af50579a5e3384c7755`.
- [x] Build two repaired ABI 23 wheels with reproducible payloads, pass 20/20
  installed-wheel tests, and recollect 6,526/6,526 from the installed wheel.
- [ ] Continue all 6,093 explicitly not-run candidate invocations and the
  incomplete Core JS/TS corpus; this Page-file result is not full-suite
  completion.

[Python full-Page evidence](python-official-conformance.md#full-page-isolated-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-239: Make official Python execution hang-resilient without losing corpus
rows.

- [x] Expand selected files/functions through the pinned 6,526-row inventory
  and preserve deterministic inventory order and requested browser filters.
- [x] Run each expanded row in its own pytest subprocess with both pytest's
  signal timeout and an outer process deadline.
- [x] Validate exact JUnit identity, synthesize explicit crash/deadline rows,
  aggregate every selected outcome, and checkpoint one report per case.
- [x] Add `--resume-isolated` so interrupted exhaustive runs reuse only valid
  exact per-case reports rather than rerunning or silently trusting drift.
- [x] Prove isolation with a 15-second route-close hang followed by an unchanged
  title test that passes in the next subprocess; aggregate result is one failed
  and one passed row, SHA-256
  `4899c51e6a63307f5ff89884869b50ab054437264c6e64fd2b273568c984c885`.
- [x] Execute the complete 125-case Page shard through the isolated runner,
  classify every independent discrepancy, and replace only validated rows.
- [ ] Extend resumable process isolation to the complete Python corpus and the
  Core JS/TS runner; 6,093 candidate Python invocations remain not run.

[Python isolated-runner evidence](python-official-conformance.md#full-no-weakened-assertion-runner).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-238: Add bounded headless `Page.pause()` timeout restoration through
Python ABI 23.

- [x] Add separately selectable `page-pause` addon and host features over the
  existing timeout-settings leaf without adding a Pwright package.
- [x] Keep headless detection, Page/Context timeout snapshots, temporary zero
  overrides and drop-safe exact restoration in Rust; headed Inspector pause is
  explicitly unsupported.
- [x] Add thin async/sync `Page.pause`, `Page.get_by_text` and Locator click
  composition while parsing the typed internal-text selector in Rust.
- [x] Pin the two exact unchanged upstream pause functions and pass 2/2 in the
  installed repaired-wheel candidate and independent official-oracle lanes on
  the same task-owned headless Chrome.
- [x] Replace exactly the two failed Page rows in the initial ABI 23 checkpoint;
  NODE-240 later supersedes that merged report with fresh full-file candidate
  and oracle executions.
- [x] Pass 20/20 installed-wheel tests, 6,526/6,526 collection, dependency,
  strict Clippy/Ruff/Pyright/format gates and repaired-wheel payload
  reproducibility.
- [x] Add per-case pytest-process isolation before using timeout continuation
  for exhaustive shards; a signal timeout can contaminate later async teardown.
- [ ] Continue all 6,093 explicitly not-run candidate invocations; this is not
  full-suite completion.

[Python ABI 23 evidence](python-ffi-compatibility-design.md#abi-23-headless-page-pause-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-237: Add bounded installed-Chromium persistent Context launch through
Python ABI 22.

- [x] Add separately selectable `persistent-context` addon and host features;
  keep attach sessions process-authority-free.
- [x] Accept only an empty or Pwright-marked profile and reject nonempty
  unmarked state before launching a browser.
- [x] Keep executable/channel selection, ephemeral loopback CDP, connection,
  process lease and bounded graceful/forced shutdown in Rust.
- [x] Publish the default Context's stable `base_url` before its topology
  identity is exposed and release every settings/process resource on close.
- [x] Pin the exact unchanged upstream persistent-Context/base-URL row and pass
  1/1 in repaired-wheel candidate and independent official-oracle lanes using
  the same installed `channel=chrome` configuration.
- [x] Pass 19/19 installed-wheel tests, 6,526/6,526 collection, exact five-crate
  dependency delta, strict format/lint/type/Clippy gates and repaired-wheel
  payload reproducibility.
- [ ] Execute and classify the other 21 collected persistent-Context
  invocations; this one-row checkpoint is not the complete surface.
- [x] Resolve the two unchanged debugger-only `page.pause()` discrepancies in
  ABI 23's task-owned headless lane.
- [ ] Continue all 6,093 explicitly not-run candidate invocations.

[Python ABI 22 evidence](python-ffi-compatibility-design.md#abi-22-persistent-chromium-context-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-233: Add bounded Rust-owned Page/Locator drag-and-drop through Python ABI
19.

- [x] Add an optional `drag-and-drop` host feature over the existing retained
  mouse/drag runtime and locator bounding-box capability.
- [x] Keep Page/Frame ownership validation, locator scroll/geometry, finite
  point validation and the bounded step count in Rust; keep Python to public
  option conversion and wrapper identity.
- [x] Add thin async/sync `Page.drag_and_drop`, `Page.locator` and
  `Locator.drag_to` facades without installing a Python input engine.
- [x] Pin four upstream files and eight exact unchanged test functions; pass
  6/6 async plus 2/2 sync in final-wheel candidate and independent oracle
  lanes.
- [x] Replace exactly four existing Page rows, add four previously not-run
  Locator/sync rows and rebuild both 6,526-row ledgers without duplicate
  outcomes: candidate 412/7/5/6102 and oracle 329/4/6193.
- [x] Prove the feature adds exactly one Pwright package, pass 469 Rust host
  tests with ten explicit live-test ignores, 15/15 installed-wheel tests,
  strict Clippy/Ruff/Pyright/format gates and repaired-wheel payload
  reproducibility.
- [ ] Add full Playwright actionability/stability/hit-target semantics and
  force mode before claiming the complete drag surface.
- [ ] Continue the seven recorded candidate failures and 6,102 not-run Python
  invocations; this is not full-suite completion.

[Python ABI 19 evidence](python-ffi-compatibility-design.md#abi-19-drag-and-drop-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-232: Add bounded `Request.sizes()` settlement through Python ABI 18.

- [x] Reuse the selected Rust network-events owner instead of adding another
  browser observer or Python request registry.
- [x] Pass the fixed two-case final-wheel candidate and independent oracle
  lanes, then replace exactly those Page rows.
- [x] Record the ABI 18 candidate 404/11/5/6106 and oracle 321/4/6201 ledgers
  plus reproducible repaired wheels.
- [ ] Continue the remaining complete-corpus work; ABI 19 supersedes this
  checkpoint.

NODE-231: Add bounded async Page request routing in Python ABI 17.

- [x] Add a separately selectable Rust `page-routes` leaf over `Fetch.enable`,
  `Fetch.requestPaused`, `Fetch.continueRequest` and `Fetch.failRequest`.
- [x] Keep 64 registration and 64 pending-call caps plus explicit pattern,
  URL, method, resource-type, header and POST-data byte limits in Rust.
- [x] Preserve last-registration priority for multiple string patterns and
  treat explicit Chromium retired-session receipts as terminal success.
- [x] Keep Python thin: retain callback/wrapper identity, await handlers and
  dispatch already bounded native descriptors; own no CDP route registry.
- [x] Pass four focused Rust AAA tests and all five fixed unchanged-upstream
  async Chromium Page-route functions in final-wheel candidate and independent
  oracle lanes.
- [x] Replace exactly those five Page cases and rebuild both complete ledgers:
  candidate 402/13/5/6106 and oracle 319/4/6203.
- [x] Pass final repaired-wheel payload reproducibility, installed-wheel 13/13,
  Ruff, strict Pyright, formatting, workflow parsing, exact dependency proof,
  and feature-cold plus selected-feature strict Clippy.
- [ ] Add regex/predicate matching, `times`, continue overrides,
  fulfill/fallback/unroute, BrowserContext routing and sync routing.
- [ ] Continue the 13 recorded candidate failures and 6,106 not-run Python
  invocations; this checkpoint is not full-suite completion.

[Python ABI 17 evidence](python-ffi-compatibility-design.md#abi-17-page-request-routing-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-230: Add a bounded Rust Page-error event leaf in Python ABI 16.

- [x] Observe native `Runtime.exceptionThrown` events in a separate optional
  frame observer; install no Python browser observer or polling loop.
- [x] Bound error name, message and stack before lifecycle-queue admission and
  publish them through the existing ordered fence.
- [x] Match Chromium Error descriptions, custom `name`, primitive thrown
  values, object and Window messages in four focused Rust tests.
- [x] Pass all five fixed unchanged-upstream Page-error functions in candidate
  and independent oracle lanes.
- [x] Replace exactly those five Page cases and rebuild both complete ledgers:
  candidate 397/18/5/6106 and oracle 314/4/6208.
- [x] Finish strict ABI 16 final-source packaging, reproducibility, lint,
  Clippy and dependency gates.
- [ ] Continue the 18 recorded candidate failures and 6,106 not-run Python
  invocations; this checkpoint is not full-suite completion.

[Python ABI 16 evidence](python-ffi-compatibility-design.md#abi-16-page-error-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-229: Select Rust-owned Page lifecycle publication in Python ABI 15.

- [x] Reuse the bounded native Page-lifecycle registry and publication fence;
  do not add a Python polling or browser-observation engine.
- [x] Commit and acknowledge each lifecycle notice before calling the thin
  Python scheduler, then release the exclusive drain to prevent callback
  re-entry from contending with native publication.
- [x] Compose observed Page/Context close with network-adapter retirement so
  the final Python wheel can select both `page-events` and `network-events`.
- [x] Expose `load`, `domcontentloaded`, `close`, `popup`, `opener`,
  `expect_popup`, `wait_for_event`, `on`, `once` and listener removal while
  preserving one public Page wrapper per native identity.
- [x] Pass the fixed six-case unchanged-upstream lifecycle manifest 6/6 in
  candidate and 6/6 in the independent official oracle.
- [x] Replace exactly those six cases in the 125-case Page report: 103 pass /
  21 fail / one upstream skip; rebuild both 6,526-record ledgers without
  duplicate rows.
- [x] Pass strict final-source packaging, payload reproducibility, Ruff,
  Pyright, feature-cold and selected-feature Clippy, Rust formatting and exact
  dependency gates for ABI 15; the installed final wheel passes 13/13 locally.
- [ ] Continue the 23 recorded candidate failures and 6,106 not-run Python
  invocations; this checkpoint is not full-suite completion.

[Python ABI 15 evidence](python-ffi-compatibility-design.md#abi-15-page-lifecycle-event-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-228: Add thin Python Page bindings over the Rust binding owner in ABI 14.

- [x] Preserve native bounded registration, invocation publication, result
  serialization and cleanup; Python owns only callable invocation and wrapper
  identity.
- [x] Pass all 12 fixed unchanged-upstream Page-binding rows in candidate and
  independent oracle lanes.
- [x] Replace those rows in the canonical Page report and rebuild both full
  6,526-record ledgers.
- [ ] Keep wider BrowserContext binding scope and remaining full-corpus work
  explicit; ABI 15 supersedes this checkpoint.

[Python ABI 14 evidence](python-ffi-compatibility-design.md#abi-14-page-binding-checkpoint-2026-09-27).

NODE-227: Select Rust-owned Handles/wait-for-function in Python ABI 13 and
complete the fixed tag and core-Handle selections.

- [x] Publish true Rust `PendingHandle` owners through PyO3 without exposing a
  remote object ID; explicit dispose waits for the Rust cleanup receipt.
- [x] Add direct Handle arguments, `json_value`, element classification and
  bounded truthy `wait_for_function` to thin async/sync facades.
- [x] Compose Page script/style tags from Frame Handle evaluation; keep path
  reads and public overload normalization in Python only.
- [x] Pass 18/18 fixed unchanged-upstream tag rows and 7/7 fixed JSHandle rows
  in candidate and independent official oracle lanes.
- [x] Replace exactly the 18 previously failed tag cases in the 125-case Page
  JUnit report and rebuild both 6,526-record ledgers without duplicate rows.
- [x] Verify Python 13/13, Rust wait-for-function 6/6, strict Pyright/Ruff,
  selected strict Clippy, exact dependency closure and repaired-wheel payload
  reproducibility.
- [ ] Add nested Handle graphs, properties and the wider ElementHandle surface.
- [ ] Continue the 41 recorded candidate failures and 6,106 not-run Python
  invocations; this checkpoint is not full-suite completion.

[Python ABI 13 evidence](python-ffi-compatibility-design.md#abi-13-handle-wait-for-function-and-tag-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-226: Add Python ABI 12 and complete Page/BrowserContext init scripts.

- [x] Generalize the bounded Rust registration scope from Page-only to typed
  Page or BrowserContext ownership.
- [x] Install Context sources into existing Pages plus future Page/OOPIF
  sessions through the shared initialization plugin.
- [x] Add thin async/sync Python source/path normalization and PyO3 methods.
- [x] Expose the same BrowserContext method through the Node facade and native
  addon.
- [x] Pass the complete unchanged async and sync official Python files 9/9
  each in both candidate and oracle lanes.
- [x] Merge all 18 outcomes into each 6,526-record ledger and preserve every
  remaining failure/not-run result.
- [x] Build two repaired ABI 12 wheels and verify payload reproducibility;
  pass Python 13/13 and Node 269/269 local tests.
- [x] Run the Rust host with every feature enabled and preserve exact static
  file-input diagnostics for `ActionTimeout` and `FrameDetached`; strict
  all-feature Clippy and the focused diagnostic tests pass.
- [ ] Continue until every remaining Python and Core invocation is executed
  and classified; this checkpoint is not full-suite completion.

[Python init-script FFI evidence](python-ffi-compatibility-design.md#abi-12-page-and-browsercontext-init-script-checkpoint-2026-09-27).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-225: Correct and verify Python Page-fill discrepancy handling.

- [x] Match Playwright's invalid input-type capitalization and non-input label
  wording in the Rust-selected browser program.
- [x] Reject a non-string Python fill value as Playwright `Error` with
  `expected string, got number` before PyO3 conversion.
- [x] Rerun the complete unchanged 125-case async Page file: 67 pass / 57 fail
  / one upstream skip, up from 64/60/1.
- [x] Run the corrected three nodes through the independent official oracle:
  3/3 pass without changing upstream assertions.
- [x] Add an oracle-only launch adapter that uses the official package's own
  `connect_over_cdp` and rejects unsupported launch options.
- [ ] Add a truly headless isolated oracle lane for debugger-only
  `page.pause()`; do not merge the interrupted visible-Chrome diagnostic.
- [ ] Eliminate the remaining 57 complete-Page-file failures and execute every
  remaining Python invocation.

[Python FFI design and evidence](python-ffi-compatibility-design.md).
[Official Python ledger](python-official-conformance.md).

NODE-223: Add an independently selectable Rust client-certificate capability
and establish exact official Node candidate/oracle evidence.

- [x] Add a bounded Rust PEM/PFX decoder and an exact-origin identity store with
  at most 64 entries and 16 MiB per certificate/key/PFX material value.
- [x] Keep certificate decoding lazy so malformed material for an unmatched
  origin does not affect requests; cache each matched TLS configuration once.
- [x] Keep the Node facade thin: validate official public shapes, read paths or
  copy buffers, then select the optional native profile.
- [x] Pass the exact malformed-input validation row 3/3 and seven direct-request
  behaviors 21/21 in both candidate and independent oracle lanes.
- [x] Use one common source-only official fixture runtime in both lanes to bind
  TCP/TLS servers to the verified Tailscale IP and use full MagicDNS URLs.
- [x] Pass 96 Rust release tests across nine request crates, Node 268/268,
  strict types, targeted strict Clippy, the exact dependency proof and a real
  Tailscale-bound PEM/PFX mutual-TLS smoke.
- [x] Merge the official outcomes into the canonical Core ledgers: candidate
  1,214 expected / one skipped / 31 unexpected / 47,231 not-run; oracle 884
  expected / 47,593 not-run.
- [x] Add Python ABI 11 and run the fixed official async/sync global-request
  client-certificate selection: eight pass and four upstream macOS WebKit
  skips in both candidate and oracle lanes.
- [x] Build repaired self-contained Python wheels twice and verify identical
  payloads after normalizing only CycloneDX build identity fields.
- [ ] Implement the official HTTP/SOCKS proxy certificate paths, browser and
  persistent-context sharing/interception, and multiple
  identities for one normalized origin.
- [ ] Execute and classify every remaining Core and Python invocation; this is
  selected official evidence, not a full-suite completion claim.

[Canonical design and evidence](api-request-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-221: Expose the browser-independent request owner through a thin optional
Node profile and establish official oracle/candidate evidence.

- [x] Add a dedicated `api-request` native profile without selecting a browser
  capability at runtime; keep seven public verbs and response conversion in a
  thin Node facade.
- [x] Keep request execution, redirect/retry/cancellation, Basic auth and the
  bounded cookie store in Rust.
- [x] Import/export storage-state cookies through the Rust store, retain origin
  JSON in the native context under an 8 MiB boundary, and support file
  round-trips in the language facade.
- [x] Accept at most 64 official request-scoped resolver addresses in Rust
  while preserving the URL hostname for HTTP Host and TLS SNI.
- [x] Pass complete unchanged Library `global-fetch` 198/198 and cookie 60/60
  candidate/oracle matrices.
- [x] Pass complete unchanged Bidi `global-fetch` 396/396 and cookie 120/120
  candidate/oracle matrices.
- [x] Record every invocation in the canonical 48,477-record ledger; this
  checkpoint reached 1,193 expected / one skipped / 31 unexpected / 47,252
  not-run before NODE-223 added certificate transport evidence.
- [x] Reject the two official malformed client-certificate shapes before
  optional native capability selection and pass the exact validation row 3/3
  in both candidate and oracle lanes.
- [x] Pass 91 Rust release tests, 266 Node tests, strict types, targeted strict
  Clippy and a real native seven-verb HTTP smoke.
- [x] Add Python ABI 10 with thin async/sync APIRequest facades and pass the
  complete unchanged global-fetch matrices: 231/231 candidate, 138/138 async
  oracle and 93/93 sync oracle.
- [x] Merge those outcomes into the 6,526-record Python ledgers: candidate
  320/62/1/6,143 and oracle 237/6,289.
- [ ] Implement proxy and complete query/form/multipart facade support and
  execute their remaining official rows/files. Direct Node PEM/PFX transport
  is tracked under NODE-223.
- [ ] Add the BrowserContext shared-cookie adapter for both language bindings.
- [ ] Execute and classify all remaining Core and Python invocations; this
  checkpoint is not a full-suite completion claim.

[Canonical design and evidence](api-request-design.md).
[Full upstream ledger](upstream-playwright-conformance.md).

NODE-220: Match the pinned request stack's bounded Happy Eyeballs scheduling in
the optional Tokio connector.

- [x] Preserve the first DNS result's address-family priority and alternate
  IPv6/IPv4 results without exceeding the existing 64-answer cap.
- [x] Start one attempt every 300 ms, keep prior attempts pending, select the
  first success and preserve the first completed error if all fail.
- [x] Drop every losing attempt on success or cancellation without spawning
  detached tasks; bypass DNS and interleaving for IP literals.
- [x] Inject attempt and delay futures into deterministic AAA scheduler tests.
- [x] Pass 88 combined release tests, doctests, rustdoc and all/pedantic strict
  Clippy for all eight API-request crates.
- [ ] Expose API request through thin profiles and run the unchanged official
  Happy Eyeballs cases before claiming a public result.

[Canonical design and evidence](api-request-design.md).

NODE-219: Make redirect and handled-auth decisions from bounded final headers
without retaining response bodies that Playwright does not return.

- [x] Add final-head and complete events to the runtime-free HTTP/1 codec while
  preserving its existing complete-response API as a wrapper.
- [x] Store raw Set-Cookie values when the final head arrives, before body
  consumption or redirect request recomposition.
- [x] Drop the pending connection, decoder and fixed 32 KiB buffer for followed
  redirects and handled Basic challenges instead of consuming their bodies.
- [x] Continue reading, decompressing and enforcing the configured body cap for
  every terminal returned response.
- [x] Pass 83 combined release tests, doctests, rustdoc and all/pedantic strict
  Clippy for all eight API-request crates.
- [ ] Expose the capability through thin Node/Python profiles and run unchanged
  official differential cases before claiming public compatibility.

[Canonical design and evidence](api-request-design.md).

NODE-218: Put HTTP Basic credential semantics in an optional Rust leaf rather
than in either language facade or the concrete connector.

- [x] Add a base authorization-provider port and propagate it through the
  browser-independent request context without selecting an implementation.
- [x] Bound username/password at 4 KiB and origin at 64 KiB; zeroize the stored
  encoded credential and redact it from Debug output.
- [x] Match optional origin case-insensitively but exactly by scheme/host/port,
  preserving arbitrary bounded nonmatching strings like the pinned source.
- [x] Support default `unauthorized` challenge retry and `always` preemptive
  send without overriding an explicit Authorization header.
- [x] Evaluate preemptive policy once before redirects and retain the inserted
  header across the chain, matching Playwright 1.58.0 source behavior.
- [x] Pass 78 combined release tests, rustdoc and all/pedantic strict Clippy for
  all eight API-request crates.
- [x] Move handled 401 challenge decisions before full body consumption.
- [ ] Expose the option through thin Node/Python profiles before official
  differential runs.

[Canonical design and evidence](api-request-design.md).

NODE-217: Keep global cookie state in an optional Rust leaf and expose only a
store port to request ownership and transport policy.

- [x] Add bounded deterministic global cookie ownership with structured
  import/export and an injected clock.
- [x] Parse raw Set-Cookie values without losing embedded `=`, normalize domain
  case, and implement host-only/domain/path/secure/expiry matching.
- [x] Compose Cookie only when a caller header is absent; persist redirects and
  recompose the next request from the selected store.
- [x] Bound entries at 1,024, one cookie at 4 KiB and aggregate text at 1 MiB;
  evict oldest entries deterministically.
- [x] Pass 66 combined release tests, rustdoc and all/pedantic strict Clippy for
  all seven API-request crates.
- [x] Move Set-Cookie/redirect handling before full body consumption.
- [ ] Add the BrowserContext cookie adapter and bounded origin storage-state
  support.
- [ ] Add thin Node/Python profiles and unchanged official differential runs
  before claiming public compatibility.

[Canonical design and evidence](api-request-design.md).

NODE-216: Add concrete network and decompression leaves without merging them
back into the API-request owner or transport policy.

- [x] Add a Tokio TCP connector with cancellation, 64-address DNS cap, one
  owned stream and reset/refused/other error classification.
- [x] Add a Rustls connector that independently selects verified WebPKI roots
  or request-scoped `ignoreHTTPSErrors` and advertises only HTTP/1.1.
- [x] Verify the insecure config with a complete in-memory self-signed TLS
  handshake; do not start a listener or background server.
- [x] Add optional bounded gzip/x-gzip, deflate and Brotli final-response
  decoding, including empty stream compatibility and expansion rejection.
- [x] Recheck decoder output independently in transport policy and preserve raw
  `Content-Encoding` response headers.
- [x] Pass 53 combined release tests, rustdoc and all/pedantic strict Clippy.
- [x] Implement bounded Happy Eyeballs in the Tokio connector.
- [ ] Implement proxy, client certificates, BrowserContext/origin storage and
  thin language profiles before claiming public compatibility.

[Canonical design and evidence](api-request-design.md).

NODE-215: Keep redirect and retry semantics in a Rust transport-policy plugin,
while keeping DNS/socket/TLS/proxy ownership behind injected ports.

- [x] Add one-exchange `ApiConnector` and `ApiConnection` contracts with no
  detached operation ownership.
- [x] Drive the bounded HTTP/1 codec through one fixed 32 KiB read buffer and
  project exact final URL/status/status text/raw headers/body.
- [x] Implement 301/302/303/307/308 location resolution, POST/303 method
  rewriting, content-header cleanup, Cookie removal and custom Host updates.
- [x] Distinguish `maxRedirects: 0` from recursive budget exhaustion and retry
  only classified resets after 250 ms exponential backoff.
- [x] Bound and truncate internal logs without failing successful responses;
  reject pre-cancelled work before connector invocation.
- [x] Pass eight release tests, rustdoc and all/pedantic strict Clippy.
- [ ] Implement concrete Tokio socket and Rustls connectors, decompression,
  proxy/credentials and cookie ports before any public support claim.

[Canonical design and evidence](api-request-design.md).

NODE-214: Preserve Playwright raw response headers in a separately selectable,
runtime-free HTTP/1 wire codec before choosing a connection stack.

- [x] Encode exact path/query/body bytes and inject `Host` plus single-exchange
  `Connection: close` only when callers did not supply them.
- [x] Reject non-Latin-1 request header values before transport and retain
  response header spelling, duplicates and order exactly.
- [x] Incrementally decode fixed, chunked, informational, HEAD/204 and
  close-delimited responses across every tested fragment boundary.
- [x] Enforce request-head, cumulative response-head/trailer, response-body and
  chunk-line bounds before unbounded growth; reject truncation and pipelining.
- [x] Pass 17 base plus 11 codec release tests, rustdoc, direct dependency
  boundary checks and all/pedantic strict Clippy.
- [ ] Add the separately selectable connector, TLS and transport-policy crates;
  this codec alone performs no network I/O and changes no official outcome.

[Canonical design and evidence](api-request-design.md).

NODE-213: Keep complete request construction in the shared Rust capability so
Node/Python and future transports do not duplicate semantics.

- [x] Add explicit encoded/repeated query, raw bytes, serialized JSON,
  repeated form and ordered text/file multipart input types.
- [x] Install user-agent/accept/encoding defaults, then apply context and
  request headers with Playwright's case-insensitive original-name policy.
- [x] Preserve user content type, derive missing content type and replace
  content length with the exact serialized byte count.
- [x] Serialize forms incrementally under the request-body cap and enforce
  query/body field-count bounds before transport dispatch.
- [x] Generate deterministic context-owned WebKit-shaped multipart boundaries,
  infer missing file MIME types and reject collisions after four bounded scans.
- [x] Pass sixteen release-profile AAA tests and all-target strict Clippy.
- [ ] Add the optional bounded HTTP transport and cookie/storage owners before
  exposing these inputs through Node or Python.

[Canonical design and evidence](api-request-design.md).

NODE-212: Build APIRequestContext as an optional browser-independent Rust
capability, not a Node fetch shim or an implicit Chromium dependency.

- [x] Pin Playwright 1.58.0 status, timeout-zero, redirect, disposal,
  response-retention and BrowserContext/global cookie ownership semantics.
- [x] Add `pwright-api-request` with no concrete HTTP, CDP, browser or language
  runtime dependency and an injected async transport port.
- [x] Validate and bound request method, URL, headers/body, response metadata,
  body, per-response log and aggregate retained bytes before registration.
- [x] Own the complete public deadline, including admission wait; cancel the
  exact request on timeout and the entire tree on context disposal.
- [x] Make response/context disposal idempotent and reclaim exact registered
  body/log bytes; preserve the context disposal reason for in-flight failures.
- [x] Pass eleven release-profile AAA tests and all-target strict Clippy.
- [ ] Add an optional bounded concrete HTTP transport with decompression,
  redirects, reset-only retries, proxy, TLS and credentials.
- [ ] Add independent global cookie/storage ownership and a BrowserContext
  cookie adapter over the same base capability.
- [ ] Add complete request serialization and response APIs, then thin
  default-off Node and Python bindings.
- [ ] Execute every applicable unchanged official APIRequest/APIResponse test
  in oracle and candidate lanes and merge outcomes into both full ledgers.
- [ ] Close package/ABI/dependency/memory/cleanup/CI gates before claiming
  public support.

[Canonical design and current evidence](api-request-design.md).

Quality-hardening checkpoint after NODE-191:

- [x] contain synchronous/rejected session-watch failures, close the native
  session, preserve cleanup errors and publish one local disconnect;
- [x] require native ABI 1 plus `NativeEnvironment` before profile markers and
  smoke-load real default/terminal artifacts in a killable child process;
- [x] make public timeout zero disable ordinary CDP response timers and align
  Page/Frame/JSHandle runtime source conversion with Locator;
- [x] add Node 20/22/24 package lanes, default native load and terminal
  standalone/combined profile gates to CI;
- [x] centralize CDP test-client call recording behind an opt-in bridge test
  dependency and prove the ordinary production bridge closure excludes it;
- [x] centralize the stateful fake's Runtime error/response/fallback precedence
  without merging its method defaults or gates with the canned bridge mock;
- [x] define and enforce canned-mock strict mode across every configurable
  response slot while keeping fixed built-in responses/no-op methods usable;
- [x] share persistent optional response storage across the canned mock and
  stateful fake without merging their defaults, scripts, gates, or DOM behavior;
- [x] extract the Page/OOPIF session-initialization plugin contract, preserve
  aggregate compatibility, and move two host implementations to the leaf;
- [x] extract synchronous Frame event, URL, and ownership projection contracts,
  preserve aggregate compatibility, and move host implementations to the leaf;
- [x] extract bounded Frame lifecycle/navigation-wait observation into an exact
  ten-package leaf, preserve aggregate compatibility, and move the Node host
  lifecycle adapter to the leaf;
- [ ] replace the aggregate Chromium dependency with capability-owned leaves
  and a narrow Node assembly. The transitional 236/251 package closures and
  39 aggregate-coupled host files remain NODE-004 and prevent a feature-minimal
  distribution claim.

NODE-191: Consume the retained serialized payload graph in one bounded,
non-retargeting Rust/CDP file-input mutation and settle all native ownership.

Pinned Playwright 1.58.2 first retargets and validates the input, awaits
`onBeforeInputAction`, then serializes `filePayloads` as a utility call
argument. Its payload branch invokes `injected.setInputFiles` in the creation
Frame's utility world; it does **not** use Chromium's
`DOM.setFileInputFiles`. The injected helper reconstructs `File` objects with
browser `atob`, assigns one `DataTransfer.files`, dispatches bubbling/composed
`input` followed by bubbling `change`, and returns text for a late non-file
target. `_setInputFiles` ignores that return. NODE-191 must preserve those
facts without acquiring or retargeting the element again.

- [x] Add default-off standalone/combined
  `retained-serialized-payload-mutation` profiles. The addon profile composes
  NODE-190; host, composition and backend features select only the new
  mutation/settlement leaf. Older artifact identities and resolved dependency
  closures must remain byte-for-byte unchanged.
- [x] Validate the move-once graph as the package-produced file-payload subset
  before native I/O: one serialized root array, at most 64 object entries,
  fixed `name`/`mimeType`/`buffer`/`lastModifiedMs` fields, valid wrapper IDs,
  base64 alphabet/padding, at most 70 MiB base64 and 64 KiB metadata. This is
  an internal protocol proof, not a second user-input validation or a generic
  evaluation codec. It also bounds JSON escaping below the selected 72 MiB
  large-command lane.
- [x] Move, never clone, that validated graph into exactly one
  `Runtime.callFunctionOn` request. Route it through the retained element's
  original utility-world client/group but use the already-retargeted object ID.
  The browser function may implement only the pinned file-payload subset of
  `parseEvaluationResultValue`; it must not call `retarget`, inspect
  `multiple`/`webkitdirectory`, resolve a selector, re-adopt a Handle or call
  `DOM.setFileInputFiles`.
- [x] Match the injected mutation itself: a late non-HTMLElement, non-INPUT or
  non-`type=file` target is a successful no-op because Playwright ignores the
  helper's returned error string; ordinary files preserve `name`, `mimeType`,
  optional `lastModifiedMs` and browser-side base64 decoding; events are
  synchronous and ordered `input` then `change` with the pinned flags. Browser
  exceptions and raw CDP failures remain typed native failures for the later
  public error-rewrite leaf.
- [x] Give mutation its own one-call backend wire/scratch admission and one
  cancellation-safe composition owner/cleanup continuation. Keep NODE-190's
  96 MiB retained-graph owner live until the graph has moved into the started
  operation; keep the validated target/group and FFI running owner through CDP
  settlement and terminal group release. A dropped public waiter may continue
  already-started mutation/cleanup but may never submit an unpolled mutation.
- [x] Extend the same parked FFI result rather than admitting a second FFI
  command: atomically take the validated target and serialized graph from the
  `PayloadImported` record, run mutation under the existing Node deadline, then
  re-park only long enough for the Node-thread callback to observe the typed
  result. Add a success-only terminal registry commit that removes the exact
  entry and releases its 1 MiB FFI permit; do not encode successful completion
  as `abort` and do not rely on preparation GC.
- [x] Keep terminal ordering explicit. The mutation future must settle the
  object group before reporting success; the callback must recheck exact
  session/ticket authority, consume the typed result, commit
  `PayloadImported -> Mutated`, and immediately terminally remove that same
  entry before resolving the internal coordinator. Failure/revocation removes
  the entry while pre-admitted cleanup continues independently.
- [x] Prove the backend with fake CDP: exact world/group/retargeted object ID,
  one large `Runtime.callFunctionOn`, no `DOM.setFileInputFiles`, no retarget
  source, move-once graph use, malformed/capacity rejection before native I/O,
  ignored helper return, exception propagation and zero residual leases.
- [x] Prove composition/host/addon ownership with AAA tests: unpolled drop sends
  no command; in-flight waiter drop completes mutation and group release;
  cleanup failure poisons new admission; graph and validated owner are each
  consumed once; foreign-session/out-of-order calls cannot revoke the rightful
  ticket; success leaves zero preparations, roots, serialized owners, running
  FFI work or native groups.
- [x] Differentially verify actual Chrome against pinned Playwright for empty,
  one-file and multiple-file payloads; byte content, name, MIME and modified
  time; `input`/`change` order and flags; late `type` change no-op; listener
  throws; detach/navigation races; and cancellation. Run standalone and
  combined artifacts, release Rust tests/strict Clippy, package/types/Worker,
  dependency proofs, the extended offline matrix and a maximum admitted payload
  memory gate.
- [x] Keep public `ElementHandle.setInputFiles`, Playwright stack/error rewriting
  and generic evaluation transport out of NODE-191. Unpaired UTF-16 strings,
  arbitrary serialized wrapper tags/live handles and generic wire-order parity
  remain explicit later gates; this leaf may claim only the package-produced
  file-payload subset plus browser mutation and cleanup.

The implementation order is backend schema/mutation, cancellation-safe
composition, host ownership, same-ticket addon handoff, terminal registry
commit, package coordinator, then offline and real-browser gates. No public
method is exposed before every ownership and differential item above is green.

Current NODE-191 checkpoint: every implementation, ownership and differential
item above is exercised. Both artifacts pass 392/392 offline
retained-input/oracle cases, 167/167 package cases and 42/42 ordinary
real-Chrome raw-binding cases; each also passes the separate
50 MiB minus one byte maximum-payload gate. Pinned, in-process Playwright
instrumentation supplies the same post-prevalidation pause for empty/multiple,
listener-throw, late-type, timeout, Frame-detach and navigation comparisons.
The matrix records rather than hides two compatibility differences: Playwright
resolves when the main document navigates during that pause while Pwright fails
closed, and the current channel strips caller `lastModifiedMs`. Public
`ElementHandle.setInputFiles` remains a later design task that must choose those
policies and add the public error/stack facade explicitly.

NODE-190: Bound and transfer the serialized value graph into Rust before any
browser mutation.

- [x] Add standalone/combined `retained-serialized-payload-import` profiles;
  compose only the existing serialization leaf, the host import owner and the
  addon's optional JSON value dependency.
- [x] Reserve one full 96 MiB Rust owner plus one ActiveCommand before copying;
  admit only one retained graph per session and leave capacity-rejected
  `Serialized` tickets retryable.
- [x] Import own enumerable string-keyed JSON-compatible data synchronously,
  bounded to depth 64, 8,192 nodes, 8,192 entries and 72 MiB total UTF-8
  strings. Reject raw BigInt, native cycles and unsupported values.
- [x] Gate `Serialized -> PayloadImported` by exact NativeSession/ticket,
  preserve the retained validated target and existing 1 MiB FFI admission, and
  reduce direct Node roots from eleven to zero only after successful import.
- [x] Keep the Rust graph move-once for NODE-191; abort/session teardown drops
  it and returns the dedicated owner slot without requiring JavaScript GC.
- [x] Verify both artifacts with the 482-case extended offline matrix, 166
  package cases, focused serializer/dispatch tests, Worker loading, exact
  dependency/type checks, strict release Clippy and both 34/34 real-Chrome
  binding lanes.
- [ ] Consume the graph with the same validated target in a non-retargeting
  Rust/CDP file-input mutation; dispatch `input` then `change` and prove
  terminal cleanup/publication.
- [ ] Add generic live-`JSHandle` adoption and exact Playwright transport/error
  behavior, including unpaired UTF-16 code units and wire-key ordering, before
  claiming the serializer/import pair is a general evaluation codec.
- [ ] Expose and differentially verify public `ElementHandle.setInputFiles`
  only after mutation, event and error-order semantics are closed.

[Canonical contract and current verification](knowledge/pwright-node-handles.md#node-190-bounded-rust-ownership-for-the-serialized-payload-graph).

NODE-189: Preserve Playwright's late utility call-argument serialization after
input instrumentation without copying payload bytes into Rust prematurely.

- [x] Pin the upstream boundary: `Buffer.toString('base64')` is early server
  materialization; `serializeAsCallArgument` is a later synchronous traversal
  after retained DOM prevalidation and awaited input instrumentation.
- [x] Add standalone/combined `retained-call-argument-serialization` addon
  profiles selecting only `retained-input-instrumentation`; preserve every
  host/backend/composition dependency graph.
- [x] Gate `Instrumented -> Serialized` by exact NativeSession and same ticket,
  retaining the validated native owner, public call identity, one deadline and
  one 1 MiB FFI admission.
- [x] Use a pinned Node language adapter for getters, `toJSON`, cycles and
  special values while Rust owns phase/root authority. Retain the result as an
  eleventh root and reuse base64 string leaves without a Rust byte copy.
- [x] Differentially cover ordinary graphs, cycles, special values, throwing
  object/array getters, `toJSON`, thenables and instrumentation-microtask order;
  cover rejection/revocation and exact-session ownership in the binding lane.
- [x] Verify 398/398 selected offline file-input cases, 165/165 package cases,
  34/34 focused serializer/dispatch cases under both artifacts, exact profile
  boundaries, types, strict Clippy, Worker loading and both 32/32 live lanes.
- [ ] Add Rust-owned live-`JSHandle` adoption for the generic `{ h }` branch;
  do not claim full evaluation-serializer compatibility from value-only payload
  coverage.
- [ ] Consume the serialized root in a non-retargeting Rust/CDP mutation leaf,
  dispatch `input` then `change`, and settle terminal cleanup/publication.
- [ ] Add Playwright error transport and public differential
  `ElementHandle.setInputFiles` only after mutation semantics are closed.

[Canonical contract and current verification](knowledge/pwright-node-handles.md#node-189-late-call-argument-serialization-on-the-retained-owner).

NODE-188: Model Playwright's awaited `onBeforeInputAction` boundary as an
independently selectable Node composition phase.

- [x] Pin the upstream order: retained DOM prevalidation and retargeting finish
  before `onBeforeInputAction`; path access, late payload serialization and DOM
  mutation begin only afterward. Public handle upload supplies no point or box.
- [x] Add standalone/combined `retained-input-instrumentation` addon profiles
  selecting only `retained-target-prevalidation`; preserve every older profile
  and the complete host/backend/composition graph.
- [x] Gate `Prevalidated -> WaitingInputAction -> Instrumented` in Rust with
  exact-session checks, while retaining the same validated owner, ten roots,
  one parked 1 MiB FFI admission and one Node deadline.
- [x] Add a frozen internal instrumentation port. Its default implementation is
  an awaited no-op; it receives a restricted progress view, retained `call@N`,
  the original receiver root and undefined point/box. Do not invent a public
  listener-registration API before tracing/debugger/recorder/screencast exist.
- [x] Abort the exact ticket on listener rejection, deadline or retirement;
  prevent foreign sessions from checking out or cancelling the owner phase.
- [x] Verify 275/275 selected offline lifecycle/Worker cases, 164/164 package
  cases, exact addon-only dependency deltas, types, standalone/combined strict
  Clippy, both artifacts and both 30/30 real-Chrome binding lanes.
- [x] Implement late utility-world call-argument serialization from the
  retained validated owner, preserving Buffer mutation timing and the same
  progress through NODE-189's independently selected leaf.
- [ ] Before selecting any concrete listener plugin, define a Rust-backed
  server-SdkObject/attribution adapter and normal-then-last listener registry;
  do not mistake the current Node receiver identity token for that full API.
- [ ] Add mutation/event completion, terminal cleanup/publication, Playwright
  error transport and public differential `ElementHandle.setInputFiles`.

[Canonical contract and current verification](knowledge/pwright-node-handles.md#node-188-awaited-input-action-instrumentation-boundary).

NODE-187: Consume the retained marked target in DOM prevalidation without
opening an ownership gap or widening existing profiles.

- [x] Add explicit standalone/combined `retained-target-prevalidation`
  features and artifact identities; leave every older profile exact.
- [x] Generalize the existing retained-preflight composition owner so it can
  wrap `OwnedMarkedElement` and preserve typed marker cleanup.
- [x] Resume and re-park the same 1 MiB FFI ticket under the same Node deadline,
  keep ten roots and advance only `Projected -> Prevalidated`.
- [x] Admit eight independent backend preflights, eight 16 KiB composition
  owners and eight cleanup continuations; keep marker-call admission nested.
- [x] Verify 52 composition cases with one live case ignored, 666 host cases
  with 24 ignored, 68 addon cases, 271 offline lifecycle cases, 163 package
  cases, both Worker/artifact profiles, exact boundaries, types, strict
  standalone/combined Clippy and both 28/28 live lanes.
- [x] Integrate the awaited `onBeforeInputAction` boundary on this same progress
  through the internal default-off instrumentation leaf.
- [ ] Integrate late utility-world serialization, mutation and terminal
  cleanup/publication on this same progress.
- [ ] Map the full Playwright language-error surface and expose/differentially
  verify public `ElementHandle.setInputFiles`.

[Canonical contract and current verification](knowledge/pwright-node-handles.md#node-187-same-owner-retained-dom-prevalidation).

NODE-186: Preserve marked-target ownership across exact Playwright carrier
projection without widening admission.

- [x] Return a move-only retained marked element from the Rust host and keep it
  inside the parked selected FFI result after ordered publication.
- [x] Run the actual synchronous projection kernel only after `Marked`, recheck
  after observable language operations and commit `Projected` with native truth
  flags.
- [x] Extend only the selected logical root maximum from six to ten for the four
  raw projected values; keep earlier profiles at six and make no JS-byte claim.
- [x] Keep one 1 MiB FFI ticket and the existing eight-call native owner bound;
  abort/drop continues through pre-admitted cleanup.
- [x] Verify 68 addon tests, 666 host tests with 24 ignored, 270 offline lifecycle
  cases, 162 package cases, both Worker/artifact profiles, types/boundaries,
  strict standalone/combined Clippy and both 27/27 live lanes.
- [x] Consume the retained marked owner directly in DOM prevalidation; never
  release and re-adopt it between phases.
- [ ] Continue with `onBeforeInputAction`, late utility-world serialization,
  mutation/cleanup and public differential `ElementHandle.setInputFiles`.

[Canonical contract and current verification](knowledge/pwright-node-handles.md#node-186-retained-marked-owner-and-same-ticket-carrier-projection).

NODE-185: Put materialization and marking under one selected public-call
envelope without widening admission.

- [x] Allocate monotonic, overflow-checked `call@N` identities per exact
  NativeSession scope after server schema validation; preserve them in Rust and
  remove the JS-supplied marker ID.
- [x] Await the instrumentation checkpoint, revalidate the retained source,
  then start one Node-event-loop timer before materialization with no Rust clock.
- [x] Race materialization and same-ticket marking under the same progress and
  revoke the exact preparation through the deadline-specific N-API edge.
- [x] Keep 32 preparations, six roots, 16 KiB registry/phase checks and one
  1 MiB FFI admission unchanged; document the separate unbounded V8 timer scope.
- [x] Verify 68 addon tests, strict standalone/combined Clippy, 269 offline
  lifecycle cases, 162 package cases, types/boundaries and final 26/26 live
  lanes on both rebuilt artifacts. Retain the combined Worker timeout and
  earlier overloaded shared-Chrome failures as evidence.
- [ ] Compose projection, retained DOM prevalidation, `onBeforeInputAction`,
  late utility serialization and mutation/cleanup on that same progress.
- [ ] Expose and differentially verify public `ElementHandle.setInputFiles`;
  the green internal 26-case binding lane is not public API parity.

[Canonical contract and current verification](knowledge/pwright-node-handles.md#node-185-session-call-identity-and-one-node-deadline).

NODE-184: The internal Node preparation now has a real same-admission native
target-marking phase.

- [x] Select typed progress at initial admission, retain the exact running
  revoker in the Node entry and upgrade the bound original Handle only after
  `Materialized` at native dispatch.
- [x] Keep the session/source registration with typed completion, park the same
  FFI ticket before lifecycle publication, revalidate after callback reentry and
  commit `Marked` only after successful typed completion.
- [x] Add standalone/combined artifact profiles and exact dependency/identity
  checks without changing existing selections or admitting another command.
- [x] Verify strict selected Clippy, 162 package contracts, Worker loading and
  24/24 real-Chrome raw binding cases including abort, late disposal and foreign
  session rejection with final owned-resource retirement.
- [ ] Establish connection-wide call identity and the public deadline before
  materialization with Node timer delivery and typed cancellation reason.
- [ ] Compose projection, retained DOM prevalidation, instrumentation, late
  serialization and mutation on the same ticket; then make the pinned public
  differential lane green.

[Canonical contract and current verification](knowledge/pwright-node-handles.md#node-184-same-admission-node-target-marking-handoff).

NODE-183: The default-off Rust host `retained-target-marking` selection now
owns persistent exact-utility-world marker caching. This completes the host
ownership prerequisite, not Node preparation promotion or retained upload.

- [x] Add exact-Frame utility-only snapshots with independent retirement
  receipts, no browser I/O/readiness wait and no main-world requirement.
- [x] Coalesce same-world initialization; separately admit 64 current worlds,
  eight retiring predecessors and eight calls, with snapshot-only persistent
  owners and cleanup retained through native acknowledgement or exact proof.
- [x] Prune stale routes outside locks and seal marker admission before native
  drains; add the Rust-only original-Handle marking phase without a new timeout.
- [x] Verify standalone/combined host and full composition tests with strict
  Clippy; the same assembled Chrome case passes both final host selections
  with all owned resources retired. The older retained host regression passes.
- [x] Complete default regressions, standalone/combined dependency proofs,
  runtime-JS/addon boundary checks and formatting; exact results are canonical.
- [x] Connect original NativeHandle Weak upgrade, session revocation and typed
  same-admission Node progress through `Marked`; NODE-184 adds explicit addon
  profiles and raw browser evidence.
- [ ] Put that phase under one public pre-materialization deadline and
  connection-wide call identity before automatic chaining.
- [ ] Compose marking, projection, DOM prevalidation, instrumentation and late
  serialization/mutation, then verify the public retained upload API.

[Canonical contract and current verification](knowledge/pwright-node-handles.md#node-183-host-owned-utility-world-marker-cache).
Earlier checkpoints retain their historical scope and verification status.

NODE-182: Persistent native target marking is a default-off composable primitive,
not yet a selected Node capability.

- [x] Separate private exact-world marker state from adopted per-call groups;
  preserve call-ID event differences, original-node identity and native flags.
- [x] Pre-admit independent state/call owners and cancellation-safe cleanup;
  retain result/exception allowance through target release and poison on failure.
- [x] Exercise nine real Chrome calls, eight ordered DOM events and final
  all-class resource retirement. This does not establish public upload parity.
- [x] Complete default and standalone/combined Rust tests, strict Clippy,
  resolved dependencies, formatting, package/types, 21 offline script cases
  and final real Chrome marker checks in both selections.
- [ ] Add bounded safe-host world caching/replacement/shutdown with independent
  retirement receipts; add typed same-admission Node handoff and publication.
- [ ] Preserve one pre-materialization deadline plus Node event-loop timer
  delivery and connection-wide call identity. Do not restart timeout at native
  entry or force expiry before synchronous language errors can be observed.
- [ ] Compose carrier projection, DOM prevalidation, instrumentation, late
  serialization and upload, then run public differential verification.

[Canonical contract and verification](knowledge/pwright-node-handles.md#node-182-persistent-native-target-marking).

NODE-181: Inert original NativeHandle binding is implemented without adding a
strong pin, root, capacity, feature or budget. It is a preparation prerequisite,
not a native phase or public retained upload.

- [x] Capture Weak state from the actual NativeHandle after existing admission
  and propagate it through all records/snapshots without upgrade or an earlier
  disposal/getter gate; retain the actual 16 KiB registry-layout check and check
  the same overlap sum against both existing 1 MiB entry admission and independent
  16 KiB phase scratch, including temporary survival after callback revocation.
- [ ] Establish one operation deadline before materialization, connection-wide
  protocol IDs and separately admitted persistent exact-world remote-marker
  ownership before selecting initial typed native progress.
- [ ] Integrate late native authority/Weak upgrade, real marking and subsequent
  native phases; current preparation remains unit-result Materialized.
- [x] Execute the twenty distinct binding cases against frozen-source final
  artifacts: standalone 20/20, combined 20/20, standalone repeat 20/20. All sixty
  passes fully retire owned cleanup with no failures/cancellations/skips; scope
  remains preparation through Materialized and explicit abort.
- [ ] Separately prove later native Weak upgrade, GC, marking and complete
  public upload behavior; these are not implied by the completed live gate.

Exact completed offline/Rust/Clippy/dependency verification belongs to the
canonical record; historical entries retain their original scope.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-181-original-native-wrapper-binding).

NODE-180: Exact-session host revocation hooks now precede native drains. This
implements the session-registry prerequisite from NODE-179, not the eventual
Node/native upload handoff. No package/feature or browser pass is added.

- [x] Add eight weak invocation slots under the existing retained-file-input
  selection, with explicit 4 KiB storage and one fixed scope/lease; check exact
  session/source metadata and generation-safe registration under host state.
- [x] Seal/take then revoke outside locks before operation and Handle waits,
  including teardown fallback; move default session and Handle-owner tracker
  wakeups outside their admission locks.
- [ ] Bind the original NativeHandle weakly, select the initial typed progress
  result and attach one registration only at the first native handoff. The 32
  initial language preparations remain unchanged and unregistered.
- [ ] Integrate marking, projection, prevalidation, instrumentation and late
  serialization/mutation under one deadline; verify actual Node assembly,
  browser cleanup and public retained upload.

Exact final test/dependency evidence belongs to the canonical checkpoint; no
pending gate is recorded as passed here. Historical checkpoints below retain
their original scope.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-180-session-bound-revocation-before-native-drains).

NODE-179: Added same-admission typed FFI parking/resumption and a synchronous
Rust raw-carrier projection diagnostic. These are prerequisites, not an actual
Node/native handoff; preparation still stops at Materialized with its original
unit-result admission. Exact verification belongs in the canonical checkpoint.

- [x] Keep parked results inside the original entry without a running token;
  resume under exact-budget/shutdown checks without retyping or readmission.
- [x] Add weak entry revocation, running-future-before-permit destruction and
  deterministic ownership/reentry tests; preserve raw carrier projection order.
- [x] Seal global FFI admission before lock-free shutdown wakeups; verify the
  regression and actual-future destruction in default and selected builds.
- [ ] Register bounded weak revokers with the exact session before parked native
  ownership, and revoke before owner waits on close or raw peer termination.
- [ ] Bind the original native wrapper weakly and define the initial typed
  progress result, publication fences and explicit projected-root admission.
- [ ] Budget persistent exact-world utility marker state and implement actual
  observable marking before carrier reads; a caller flag is not a receipt.
- [ ] Integrate NODE-178 prevalidation, instrumentation, late utility
  serialization and exact-retargeted mutation under one operation deadline
  originating before materialization, then execute actual-ticket/browser gates.

No native marking, new production ticket phase or public retained upload is
claimed. Historical checkpoints below retain their original scope.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-179-same-admission-ffi-handoff-and-raw-carrier-projection).

NODE-178: Implemented standalone default-off retained native DOM prevalidation,
one-shot result/exception admission, exact retargeted receiver ownership and
cancellation-safe group release. Host/addon selection and public upload remain
open; no new browser result or addon rebuild is claimed.

- [x] Split native precheck from eager payload normalization/mutation, requiring
  an already-adopted settlement-release owner and retaining its exact route.
- [x] Test query/ready/release cancellation, admission refusal and cleanup
  failure without claiming discarded executor jobs prove native reclamation.
- [ ] Integrate observable original-target marking and raw carrier projection.
- [ ] Design parked-owner revocation/promotion across onBeforeInputAction and
  late utility serialization under one operation deadline and original permit.
- [ ] Preserve the exact validated receiver during mutation; do not invoke an
  existing helper that retargets again or normalizes raw payloads early.
- [ ] Execute live binding/native/public-upload gates once dedicated Chrome
  is available under the existing browser and shared-service authority rules.

[Canonical contract and verification](knowledge/pwright-node-handles.md#node-178-owned-native-file-input-prevalidation).

NODE-177: Implemented selected two-checkpoint registration gating around server
schema and an empty instrumentation await, using exact owner-ticket plus creation
Page membership rather than world/Handle/Frame runtime presence. The thin driver
schedules setImmediate and resolves undefined at Materialized, without new roots,
dependencies, classes, budgets or native pins. Twenty new offline dispatch cases
pass per rebuilt artifact; exact results route to the canonical record. No new
browser result or public retained upload is claimed.

Next design constraint: preserve mark target -> carrier projection -> DOM
checks/retarget -> onBeforeInputAction -> late utility serializer -> browser
atob/File. Eager PreparedFileInput conversion is not a valid integration step.
Keep actual native authority/promotion, float deadlines, full instrumentation,
error transport, path/stream support and the browser gate explicit.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-177-deferred-registration-gates-before-materialization).

NODE-176: Server-direction schema and raw payload helper are implemented;
both selected artifacts pass 76 new offline cases plus regressions. Remaining integration requires
real deferred target admission/revalidation, instrumentation/error transport,
float deadlines, bounded typed decoding and same-permit execution promotion.
Public retained upload and the full browser discrepancy gate stay incomplete.
[Canonical contract and current evidence](knowledge/pwright-node-handles.md#node-176-server-direction-validation-and-raw-payload-materialization).

NODE-175 (2026-09-17): Implement and verify the synchronous Rust
ArgumentsReady to ChannelReady validator, replacing root six under existing
revocable phase admission. Cover schema order payloads (name, optional mimeType,
buffer), localDirectory, directoryStream, localPaths, streams, timeout; optional
fields; observable map/raw results; raw boxed valueOf results; inherited-setter
assignment; and Buffer identity without copying or serialization. Use frozen
primitive-only ports and normal non-isUnderTest semantics. Use three
no-capture/ZST native callback specializations with static typed dispatch and
current Env/path/ports per invocation, never captured scoped N-API values or
registry/native owners. This avoids new Rust callback-Box allocation and its
construction-failure leak; do not count N-API/V8 callback backing or arbitrary
escaped JS graphs within administrative RSS bounds.

Keep WritableStream as a missing capability: observe the first `_object`
read and fail closed with the expected-channel error until a genuine producer/
brand exists. Do not claim valid-stream or whole-protocol parity. Root,
FFI/scratch, metadata, feature, dependency and raw-class budgets stay unchanged.
Offline verification is recorded in the canonical checkpoint; actual-ticket/
browser/public gates remain open. Written browser gates are not passes; shared
CDP remains blocked without restart permission and no new browser pass is claimed.

Remaining: public API error prefix/stack; deferred server bytes/path preparation;
float deadline semantics; typed move-only FfiPreparation promotion, renewed
native-source authority and submission; public ElementHandle.setInputFiles;
actual-ticket/browser and full discrepancy gates. [Canonical status and evidence](knowledge/pwright-node-handles.md)
own completion claims; earlier tasks below remain historical.

NODE-174 (2026-09-17): Production creation-Page association now uses a bounded
canonical session Context/Page index, published membership and exact weak
Handle-owner provenance. Native Handle publication checks immutable Page proof;
all Rust-derived Handle producers, including generic descendants and batches,
inherit the binding. Preparation captures it before language phases, and
`assembleArguments(ports)` removes late caller-selected settings. The facade
forwards creation Page ID only; Rust captures canonical policy or lookup error
after FFI admission, deferring the error through normal source/argument
materialization before host execution. No JS cell/cache/lookup can reorder
closed-Page conversion, and idle constructors allocate no policy binding during
pre-commit lifecycle preparation. The facade initializes cached import-time raw
debug inputs once per selected connection; Rust owns undefined-only fallback
and inspector exclusions. These replace NODE-173's binding/debug gaps below.

Preserve separate admission for fresh scoped publications: cells plus bindings
share 1024 slots, with fixed administration and sixteen bounded session indexes
totaling 784 KiB. Close/retained metadata must not pin native authority or make
closed index receipts free. Existing root/preparation/synchronous-phase limits
and older artifact paths remain; no feature/profile/dependency/class is added.

Remaining: public default setters and arbitrary JS default state; channel
validation/error phases; deferred Buffer serialization/path work; native float
deadline mapping, promotion/submission and public ElementHandle.setInputFiles;
then actual-ticket, browser and discrepancy gates. [Canonical NODE-174 status](knowledge/pwright-node-handles.md#node-174-canonical-creation-page-timeout-binding)
owns exact evidence, not these routing notes. Earlier tasks retain historical
scope and are not fresh browser-pass claims.

NODE-173 (2026-09-17): Implement the Rust post-conversion argument phase:
converted spread, options spread, then live timeout resolution with no extra
await. Raw typed-settings assembly advances Ready to ArgumentsReady while
replacing the sixth root; the six-root/32-preparation/512 KiB synchronous-phase
ceilings stay unchanged. Separately admit Context/Page timeout metadata under
1024 cells at 512 bytes plus 16 KiB administration (528 KiB, one scope and
1025 leases). Keep the factory alive through the final cell lease without JS
roots, session/native authority or task ownership.

Preserve falsy-options normalization, separate numeric check/return reads,
debug-before-default ordering, live defaults after getters and repeated
options reads at parent fallback. The new safely published module-local
settings class is internal; only the normal two-level Page/Context shape and
typed numeric optional defaults are implemented. Creation-Frame association
is caller-selected, not automatically bound or validated.

Remaining: production facade binding and default/debug configuration; public
setter/arbitrary JavaScript default-state compatibility; actual-ticket N-API
integration; channel validation/error phases; deferred Buffer serialization
and paths; native timeout-domain mapping; promotion/submission and public
`ElementHandle.setInputFiles`, then browser/discrepancy gates. No new feature,
profile, dependency package or server is introduced.
[Canonical checkpoint and verification status](knowledge/pwright-node-handles.md#node-173-rust-owned-argument-assembly-and-live-timeout-settings)
own exact oracle/candidate evidence; written tests are not passed gates.
Earlier entries below preserve their original completion scope.

NODE-172 (2026-09-17): Implement the Rust synchronous preflight kernel and
reentrant language-phase boundary: awaited observable owner lookup, context
outside async conversion, array classification/size policy, then ordinary
`{ payloads }` async-return assimilation. Weak scope/registry phase guards use
pure `RootSnapshot` cloning under registry borrows: weak source metadata plus
an `Rc<NodeReference>`. N-API reference lookup occurs afterward, and its temporary
Rc drops before language callbacks. Initial `creation_phase` and later phases
pre-admit scratch before N-API allocation, recheck liveness after callbacks and
reject revoked replacement. Verify the independent registry-wide limit of 32
synchronous construction/replacement phases, each with a 16 KiB administrative
allowance (512 KiB aggregate). Scratch must survive callback abort/finalization
until synchronous return, not arbitrary Promise waits, without holding an FFI
Command/task or native authority. This increases selected resource totals;
32 entries, 16 scopes, the 16 KiB table, 1 MiB per preparation and shared 64 MiB
FFI pool remain unchanged. Registered 3/4/5/6-value arrays total at most 192;
arbitrary V8 graphs, scoped handles and opaque allocator backing are not
byte-bounded by these counts or allowances.

Remaining: actual-ticket N-API verification, public closed/cancelled phase
alignment, path stat/read after typed classification, options/channel getter
ordering, deferred byte materialization, promotion/native submission and public
`ElementHandle.setInputFiles`, followed by browser and broader discrepancy gates.
Paths currently fail explicitly at the parent boundary. Stateless selected
diagnostics and their differential harness cover the shared kernel, not a
completed session-bound API. No new features, dependencies or artifact profiles
are introduced. [Canonical checkpoint and verification status](knowledge/pwright-node-handles.md#node-172-rust-owned-preflight-and-reentrant-language-phases)
own evidence; pending gates are not passes. Earlier entries below preserve
their original completion scope.

NODE-171 (2026-09-17): Implement the selected Node-thread preparation registry,
module-local weak exact-session ticket, pure Handle provenance metadata and
close/Drop/environment revocation before FFI drain. Explicit capacities are
32 entries, 16 live session scopes, 16 KiB fixed administration and 1 MiB per
entry from the existing FFI Command budget. Receiver/files/options are rooted
only after admission; no getters, Buffer conversion or native work run yet.
Two new retained-file-input artifact profiles select this prerequisite while
the thirteen older profiles remain unchanged.

Remaining: phase-correct public ownerFrame/context observations, channel/options
validation and exception ordering, deferred Buffer materialization, separate
path stat/read phases, ticket promotion, native submission and public
`ElementHandle.setInputFiles`, followed by browser/differential/Worker gates.
Do not claim immediate raw peer-close reclamation without watcher delivery:
later publication, explicit close, wrapper Drop or environment shutdown can
still be required. Fixed administrative bytes do not bound retained closed
wrappers, weak tombstone allocations or user V8 graphs.
[Canonical checkpoint and exact verification](knowledge/pwright-node-handles.md#node-171-session-bound-node-preparation-registry).
Earlier entries below preserve their original completion scope.

NODE-170 (2026-09-17): Implement the host's revocable preparation-to-execution admission split without widening any budgets. Next: the session-bound Node root registry, explicit abort/close hooks, public ownerFrame/context observations, phase-specific exception handling, delayed Buffer serialization/copy, fractional timeout and path observation/materialization. Public retained upload and browser parity remain open.
[Canonical contract and evidence](knowledge/pwright-node-handles.md#node-170-revocable-ffi-preparation-before-execution).

NODE-169 (2026-09-17): Implement the host-only exact retained upload submission path and characterize pinned upstream ingress offline. Remaining work is the phase-accurate Node preparation ticket/rendezvous, delayed byte materialization, separate path stat/read phases, public producers and selected addon artifact, followed by native/differential/Worker gates. The public Node upload method is still absent.
[Canonical contract and evidence](knowledge/pwright-node-handles.md#node-169-safe-host-retained-upload-and-pinned-ingress-oracle).

NODE-168 (2026-09-17): Implement the default-off exact-node group owner and
coherent utility-world snapshot prerequisite; validate cancellation, malformed
native results, numeric reuse, cleanup/error and resource boundaries. The
[canonical checkpoint](knowledge/pwright-node-handles.md#node-168-group-owned-retained-adoption-and-deferred-upload-ingress)
records current evidence. Remaining upload work includes the Rust preparation
ticket, phase-accurate Node rendezvous, deferred byte copying, stat-versus-read
path split, public producer APIs, combined artifact and full differential gates.
Do not call the Node upload API implemented merely because its owner exists.

NODE-167 (2026-09-17): Implement and validate current-DOM ownerFrame as the
first retained-upload prerequisite; [the canonical checkpoint](knowledge/pwright-node-handles.md)
records exact current evidence. Remaining upload work is one bounded
preparation ticket, preflight-before-getter order, Node conversion rendezvous,
independently group-owned exact-node adoption, mutation/cleanup continuation,
selector producers and combined artifact proof. Do not equate ownerFrame or a
borrowed PayloadTarget with a completed upload API. Full-corpus, preview,
Worker/fault/concurrency and unaddressable-owner design tasks remain open.

NODE-166 (2026-09-17): Implemented the Node getProperties vertical slice:
pre-admitted native batches, complete addressable property-response cleanup,
independent sibling ownership, atomic publication and failure-safe language
wrappers. Also repaired exact native-world receipts after confirmed Page.close.
[The canonical checkpoint](knowledge/pwright-node-handles.md) owns green/red
verification status. This supersedes the implementation-missing batch notes
below, not the full compatibility goal. Remaining: preview/toString,
ownerFrame/adoption, retained ElementHandle actions/uploads, broader Worker,
fault/concurrency cases and the complete discrepancy corpus; retain the
unaddressable-object-group and typed compensation/retirement design tasks.

NODE-165 (2026-09-17): Implemented the addressable-ID acquisition repair:
retained-result evaluation explicitly awaits raw CDP settlement, preserves
exception RemoteObjects, and compensates distinct result/exception IDs after
parse or JavaScript failure while retaining evaluation admission. The shared
parser remains pure; invalid, absent-for-retained, or oversized IDs report
unresolved Cleanup rather than fabricated release success. This supersedes the
addressable-ID implementation gap noted in NODE-164, not every native-ownership
gap. [Canonical verification and follow-ups](knowledge/pwright-node-handles.md)
separate newly tested behavior from pending gates.

Next design tasks are retained-to-settlement Runtime.getProperties and admission
across the entire property pipeline, a separately admitted pre-owned object
group for unaddressable IDs, Node getProperties sibling/batch ownership, and
the still-incomplete full compatibility corpus. No package, feature, portable
schema or public Node API is added in this slice.

NODE-164 (2026-09-17): The priority raw-native cross-build audit gap now has a crate-wide module-local TypeTag implementation using the actual N-API argument/receiver checks. Portable HandleGetProperty also uses identity-preserving snapshots, pre-I/O route admission and shared publication, while abandoned Cleanup outcomes remain failures. This supersedes those implementation gaps; [the canonical checkpoint](knowledge/pwright-node-handles.md) records exact verification and remaining gates. Node getProperties, batch ownership and the full-corpus compatibility tasks remain open.

Historical NODE-164 audit priority (addressable-ID acquisition implementation superseded by NODE-165 above): close the malformed RemoteObject parse-before-owner cleanup gap, with native-release ownership and failure-injection evidence recorded in [the canonical checkpoint](knowledge/pwright-node-handles.md).

NODE-163: Implement and verify single-result Handle producers, preserving Promise-valued properties instead of awaiting them. Keep the full producer lane visible: the five getProperties cases must remain red until atomic batch admission, distinct sibling owner identities, deferred batch publication and N-API/JS rollback are implemented. Canonical evidence and the batch design are in knowledge/pwright-node-handles.md. Broader corpus, preview, adoption, Worker and retained-action/upload gates remain open.

Priority audit follow-up: raw native addon classes still rely on stable content-derived napi-rs type tags. Design module-local validation before any mixed-build native pointer cast, including NativeHandle and NativeSession; public wrapper-argument isolation is not proof of this separate boundary. Do not invoke a known incompatible Rust layout as a test. The existing call allowance is now shared through delayed caller/native completion, with no extra lease or call slot.

NODE-162: Native string-expression paths for JSHandle.evaluate and Frame/Page evaluation with Handle arguments, global evaluation scope and disposed-target conversion ordering pass 30 Handle tests, 38 evaluation regressions and module isolation. Ordinary/combined Rust hosts and strict profile checks pass; evidence and ignored tests are in the canonical Handle record. Handle-targeted/property producers, previews, ownerFrame/adoption, retained actions/uploads and Handle fault/Worker/concurrent-disposal gates remain next; full corpus discrepancies are still open.

NODE-161: Owned nested Handle arguments now reach Frame/Page by-value evaluation, Frame Handle creation and Handle by-value evaluation. Continue with string-expression arguments, Handle-targeted/property producers, canonical previews, ownerFrame/adoption and retained actions/uploads. Add Handle-specific N-API failure/Worker teardown and concurrent-disposal conformance before claiming full ownership/parity coverage.

NODE-160: First public opt-in Handle path is implemented: Page/Frame.evaluateHandle, JSHandle.jsonValue/function-evaluate/dispose and ElementHandle identity. Next design owned nested-reference conversion, other Handle/property producers, string evaluation, preview formatting and retained ElementHandle operations, then rerun the unchanged 18-case upload lane. Do not infer full compatibility from the focused passing lane.

NODE-159: Safe-host Frame Handle creation now stages native results and publishes in the existing driver, with pre-admitted compensation for orphaned results and queue shutdown. Next retain wrappers and wire Node producers/operations, then extend safe ownership to Handle-targeted/property-batch producers and verify the public discrepancy lane.

NODE-158: Added the opt-in staged Frame.evaluateHandle leaf: native execution and local publication are separate, busy publication retains its result, and stale sources require explicit compensation. Next pre-admit host queue and orphan-cleanup ownership before native I/O, integrate driver publication, then extend other Handle producers and public Node wrappers. Do not retry user JavaScript.

NODE-157: Integrated bounded native release receipts into the existing host driver and shutdown drain, with exact source-world parent-retirement evidence. Concurrent-close no longer installs an unused control outbox. Next coordinate raw creation publication with unrelated lifecycle transactions, then implement persistent Node wrappers and public Handle producers/operations.

NODE-156: Implemented and natively verified an opt-in Handle release/commit receipt with exact route-resource identity and retained admission on busy boundaries. Next wire admitted native completions into the existing host driver, including shutdown draining; only then remove the temporary host-local cleanup mutex and open persistent Node wrapper work.

NODE-155: Implemented explicit queue-free runtime disposal reservations and prepare_or_defer with retained admission on busy boundaries. Next integrate exact native-release identity receipts and short commits with the shared host lifecycle driver; the legacy Handle native-await transaction and public Node Handle gate remain unchanged.

NODE-154: Rust pending-result ownership is implemented with synchronous ticket transfer and session revocation independent of descriptor collection. Five owner tests and three native checks pass after sequencing Handle cleanup; retained descriptors keep exactly 65 storage leases until dropped. Next coordinate mixed lifecycle transactions/creation publication, then implement persistent wrapper publication/disposal and public Node Handle APIs.

NODE-153: Harden the existing in-process Handle continuation against partial cleanup failure: attempt every abandoned result, retain failed native routes for explicit retry and report failure. Pending Node result/publication ownership and retained-wrapper shutdown still remain the next implementation gate.

NODE-152: Native host Handle profile/leaf installation and bounded route metrics are implemented. Native creation/disposal/64-route capacity/Page retirement and the 8 existing uploads pass. Next add a resource-owned publication result whose shutdown does not depend on JS GC. Public Node Handle APIs and the 18-case red baseline remain open.

NODE-151: Added the retained-node payload backend seam and an 18-case public Handle lane. Oracle passes; candidate is red on missing Handle producers. Next install/publish bounded native Handles and current ownerFrame/lifecycle, then compose upload ownership and cancellation. Do not emulate a Handle with a Locator.

NODE-150: Page/Frame upload wrappers and native-scoped query composition pass 43/43 x3 standalone and 43/43 combined. Full corpus remains 53/57 matching, with the click mismatch now reversed. Continue handle/directory/timing coverage; raw-click controls indicate fresh-document timing sensitivity shared with upstream, not a verified production fix.
[Page/Frame upload contract and verification](knowledge/pwright-file-input.md#node-150-page-and-frame-selector-taking-upload-apis).

NODE-149 checkpoint: selected/combined upload verification is complete for the documented 32-case surface. Full shared upload now matches; overall 57-case corpus remains non-green (53 matches). Next: remaining Page/Frame/handle/directory/timing surfaces and the retained click/download/history gaps.

NODE-149: Selected N-API ingress, isolated profiles and public Locator upload are implemented and their differential/Worker gates pass. Next close remaining API surfaces, directory mutation and slow-preparation timing parity.
[Selected Node upload boundary and verification](knowledge/pwright-file-input.md#node-149-selected-node-buffer-ingress-and-public-locator-uploads).

NODE-148: Implemented selected safe-host buffered/path upload composition, shared exact-world resolution, deadline/cancellation and shutdown drain. Next implement Node Buffer ingress, thin public wrappers and a selected addon artifact; native directories, worker-thread proof and public differential gates remain open.
[Selected safe-host upload contract](knowledge/pwright-file-input.md#node-148-selected-safe-host-upload-integration).

NODE-147: Implemented the opt-in large outbound transport prerequisite and selected buffered backend path. Next: public source/Buffer ingress, shared resolver ownership, cancellation/Page/Frame/Worker integration and isolated Node artifact; native directories and full differential acceptance remain open.
[Large outbound admission and remaining gates](knowledge/pwright-file-input.md#node-147-independently-admitted-large-outbound-commands).

NODE-146: Implemented the single-use payload continuation and opt-in CDP settlement prerequisite. Still wire public operation cancellation/deadline publication, exact Page/Frame retirement and worker shutdown/drain, portable capability/codec, memory-payload preparation, native directory completion, selected FFI/message budgets and public differential cases. These low-level continuations do not finish the Node upload action.
[Upload continuation and remaining gates](knowledge/pwright-file-input.md#node-146-single-use-upload-continuations).

NODE-145: Implement and verify the actual Chromium buffered-file mutation slice. Keep native file/directory commands with cancellable directory event waits, large-message limits, full operation cleanup, memory-payload preparation and Node/public differential integration open. Low-level live tests are not substituted for those acceptance gates.
[Remaining file-input work](knowledge/pwright-file-input.md#node-145-chromium-payload-mutation).

NODE-144: Complete the separately selectable admitted blocking-executor slice and verify capacity, cancellation, runtime shutdown, panic cleanup and shared production-plan composition. Keep Node operation/Page lifecycle integration, payload MIME conversion, browser upload and differential gates open. Executor unit tests are not public upload completion.
[Remaining file-input implementation work](knowledge/pwright-file-input.md#node-144-admitted-blocking-executor).

NODE-143: Implement path/stat/MIME services and admitted path/descriptor retention with actual Unix file reading and native directory routing. Keep executor admission, browser upload/cancellation, transport peak budgets and public Node wiring open. File-source preparation alone does not complete setInputFiles.
[Remaining upload work](knowledge/pwright-file-input.md#node-143-filesystem-preparation-and-pinned-mime).

NODE-142: Complete the bounded source-content ownership subtask with real-file, source-growth, budget-before-read and abandoned-result tests. Keep the larger materializer task open: relative paths/stat/MIME, native-directory retention and executor/handle admission are not implemented by this content leaf. No public upload completion is claimed.
[Remaining file-input implementation tasks](knowledge/pwright-file-input.md#node-142-ledger-admitted-content-ownership).

NODE-141: File-upload preparation policy is implemented and source/oracle-characterized. Next add explicit filesystem/MIME and byte owners, exact Locator/native upload with directory-event cancellation, selected transport/FFI admission and public bindings. Keep path, memory-file, directory, lifecycle and shared-corpus gates open; model tests are not upload completion.
[File-input policy, evidence and remaining work](knowledge/pwright-file-input.md).

NODE-140: Broader verification remains non-green. Preserve arrival-close failures; investigate bounded Page/request capacity attribution and raw input misses without retries or weakened assertions. File upload/download remain unsupported, and history is unimplemented with a failing upstream reference. Equal outputs do not prove intended behavior when both sides miss an interaction.
[Expanded verification and retained failures](knowledge/testing-strategy.md#node-140-broadened-node-ffi-verification-remains-non-green).

NODE-139: Implement the Page-lifetime publication boundary and verify it independently from native navigation phase differences. Stable committed-document close cases already passed on NODE-138, so they are regression controls rather than a claimed red/green reproduction. Full compatibility, borrowed/default-context concurrent close paths and complete peak accounting remain open.
[Page lifetime publication contract and verification](knowledge/pwright-frame-navigation.md#node-139-page-lifetime-through-navigation-publication).

NODE-138: Actual Node close-driver integration is implemented behind concurrent-close, including bounded admission, post-transaction completion and stop-time native drain. The final rebuilt combined artifact passes the original controlled/transparent close gates, three controlled repeats and 27 lifecycle regressions. The then-observed Context-close Frame.goto error difference was investigated in NODE-139 and involved different native document phases; it is not a proven same-phase semantic defect. Full compatibility and complete peak accounting remain open.
[Node close-driver integration and final verification](knowledge/pwright-close-cascade.md#node-138-opt-in-node-close-driver-integration).

NODE-137: Implemented the native-proven local retirement API and borrowed Handle cleanup, including an explicit notification owner that can outlive an attachment lock scope. Next wire bounded close jobs into the actual Node driver, drain written work before shutdown waits and rerun real Node/Chrome differential gates.
[Native local retirement and post-lock notification contract](knowledge/pwright-close-cascade.md#node-137-native-proven-local-retirement-and-post-session-lock-notification).

NODE-136: Joined capacity ownership and consuming native completions are implemented. Next integrate route/Handle/network/native-world retirement, full peak ownership and real publication into the short commit, then select and drain concurrent close scheduling in Node. Keep the controlled ancestor-dispatch browser gate open.
[Joined close capacity and native completion contract](knowledge/pwright-close-cascade.md#node-136-joined-close-capacity-and-consuming-native-completions).

NODE-135: Implemented runtime-locked local publication/disposal to prevent transport invalidation between local commit phases. Ten new tests pass; runtime ordinary/selected/combined totals are 113/178/196 and default host 416 (8 ignored). Next finish stable commit scratch/graph ownership and compose native, topology, routes, Handles, network and publication before switching Node scheduling.
[Runtime-locked publication/disposal contract and evidence](knowledge/pwright-close-cascade.md#node-135-runtime-locked-local-publication-and-disposal).

NODE-134: Implemented runtime-owned preallocated outbox backing with 14 new runtime/two core tests. Runtime ordinary/selected/combined passes 113/168/186; core passes 75, strict lint, doctest and Node boundaries pass. Next finish graph/table and topology/Handle/network peak ownership, compose current-state close commits and wire/drain the Node scheduler. Controlled ancestor-dispatch remains open.
[Preallocated runtime outbox contract and evidence](knowledge/pwright-close-cascade.md#node-134-preallocated-runtime-control-outbox-storage).

NODE-133: Implemented exact native Page destruction for retained paired/utility worlds, including late events after route removal. Selected/combined composition pass 397/486 (16 ignored each), default host 416 (8 ignored), Node unit 84 and two rebuilt addon profiles pass 28 focused Chrome cases. Strict lint, types, dependency boundaries and formatting pass. Next finish full close peak ownership/commit and Node scheduler integration, then rerun the controlled ancestor-dispatch gate; no new commit/push.
[Native Page world-retirement contract and evidence](knowledge/pwright-close-cascade.md#node-133-retained-worlds-follow-exact-native-page-destruction).

NODE-132: Implemented attachment-bound native close/topology commit scope with under-lock identity validation and borrowed native keys. Final combined/authority release suites pass 474/394 (16 ignored each), including all 12 new scope cases in the combined run; three strict Clippy configurations, dependency/Node boundaries, formatting and diff checks pass. Next integrate pre-admitted runtime/routes/Handles/network/native-world/publication into this scope and wire host scheduling; full goal remains active, with no addon/browser compatibility claim or new commit/push.
[Native close commit-scope contract and evidence](knowledge/pwright-close-cascade.md#node-132-attachment-bound-native-close-commit-scope).

NODE-131: Implemented deferred runtime commit and traversal/notification collection-payload peak admission. Runtime tests pass 113/154/172 (ordinary/selected/combined), including 13 new cases; host regressions pass 416/464. Strict lint, doctest, dependency/Node boundaries and formatting pass. Next admit long-lived runtime/outbox storage plus topology/Handle/network scratch, compose latest-state retirement/publication, select native scheduling in the host and rerun the controlled browser gate. No full compatibility claim or new commit/push.
[Deferred runtime close commit contract and evidence](knowledge/pwright-close-cascade.md#node-131-deferred-runtime-disposal-notifications).

NODE-130: Implemented the route phase of short close commits: deletion-only survivor checks, preserved provider identities, navigation revocation, subscription pruning and post-lock readiness wake. All 13 new tests pass; selected/combined release suites pass 385/462 (16 ignored each). Next join it with reserved topology/runtime/publication and native scheduler receipts; no full host integration or browser pass yet.
[Deletion-only close route contract and evidence](knowledge/pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-129: Implemented the native half of concurrent close coordination: fixed admission, retained native futures/ancestor joins, payload ownership, fair completions and guarded initiation. Verification passes 416 composition tests (16 ignored), 38 native-leaf tests and six ordinary-leaf tests; strict lint and dependency boundaries pass. Next derive and compose the complete retirement/publication peak budget, wire serialized current-state commits into the host, rebuild both addons, and rerun the controlled browser gate. Full goal remains active; no new commit/push.
[Bounded native scheduler contract and evidence](knowledge/pwright-close-cascade.md#node-129-bounded-native-close-scheduler).

NODE-128: Completed opt-in Context ancestor authority with 22 new identity/admission/native-peer tests (394 selected composition passes, 16 ignored). Next compose it with native Page phases, topology/runtime/publication reservations and full peak budgets in the bounded coordinator, then pass the original controlled dispatch gate. No new commit/push.
[Native ancestor authority contract and evidence](knowledge/pwright-close-cascade.md#node-128-exact-native-context-ancestor-authority).

NODE-127: Completed native Page-close command/receipt phase separation and 15 new production-CDP peer tests (36 selected total). Next bind exact Context/epoch/ancestry retirement authority, compose the storage reservations into bounded native scheduling and pass the controlled NODE-123 dispatch gate. Overall compatibility remains unfinished; no new commit/push.
[Native close phase contract and evidence](knowledge/pwright-close-cascade.md#node-127-native-command-and-destruction-wait-phases).

NODE-126: Deferred publication admission/preparation is implemented as the third storage prerequisite after topology and runtime reservations. Next compose exact native intents/ancestor receipts, complete peak ownership budgets and serialized commits, then rerun the red NODE-123 controlled dispatch gate.
[Deferred publication contract and evidence](knowledge/pwright-close-cascade.md#node-126-deferred-close-publication-capacity).

NODE-125: Completed opt-in runtime waiting/outbox admission and latest-state disposal preparation, including closed-transport priority, partial-admission rollback and lifetime-handoff interaction. Tests pass 142 selected, 159 combined, 113 ordinary. Next implement bounded native close intents and exact ancestor receipts; derive commit-scratch/publication/native-future budgets, rebase publication and serialize short commits; then pass NODE-123 controlled dispatch without releasing child retirement first. No additional commit or push.
[Runtime reservation contract and evidence](knowledge/pwright-close-cascade.md#node-125-opt-in-runtime-disposal-and-outbox-reservations).

NODE-124: Completed opt-in topology transition reservations and 27 new ownership/capacity/latest-state tests (76 selected total; 49 ordinary). Next implement runtime disposal/control-message reservations without a long-held lifecycle gate; then wire bounded native intents, exact ancestor receipts and serialized publication. Keep the NODE-123 controlled ancestor-dispatch gate red until the actual scheduling change passes; no additional commit or push.
[Topology reservation contract and evidence](knowledge/pwright-close-cascade.md#node-124-opt-in-topology-transition-reservations).

NODE-123: Fixed Context interruption semantics and the later reply-producer/request-queue-loss settlement window. Final rebuilt profiles each pass 50 close-related cases plus two extra six-case confirmation repeats; retain the intermediate peer-repeat failure. A controlled red probe proves ancestor disposal remains blocked behind pending Page confirmation despite host/FFI admission. Next: implement additive retirement-capacity reservations without long-held mutation gates; compose bounded native intents, authoritative ancestor receipts and serialized latest-state publication; then pass the controlled dispatch gate without releasing Page retirement first. Preserve ordinary/v1 paths, failed-parent continuation and full discrepancy scope. No additional commit or push.
[Close cascade requirements](knowledge/pwright-close-cascade.md).

NODE-122: Implemented selected native close confirmation, explicit call/source/error budgets, independent cursor, continuation retirement and Rust terminal-settlement fencing. Both final artifacts pass the 48-case close/lifecycle/connection matrix, and strict Rust/type/dependency checks pass. The extra Frame-navigation gate remains 27/29: retain the arrival-boundary close reproducer, isolate exact native receipts/cleanup sequencing next, and do not manufacture success or change deadlines. The overall Node compatibility goal remains active; no additional commit or push.
[Selected confirmation](knowledge/pwright-page-close.md#node-122-selected-native-destruction-confirmation).

NODE-121: Transparent tracing of the original mutation/close-reentry scenarios passes 16 samples per profile; no natural failure was captured. Pinned 1.58.2 source nevertheless proves ordinary close waits for actual closure, unlike the Node selection's finite absence-check policy. Next implement separately admitted native close confirmation, preserving ordinary/v1 polling, native ownership and cancellation/publication gates; do not keep repeating unchanged probes or claim the unidentified timeout fixed.
[Transparent trace and next confirmation contract](knowledge/pwright-browser-close.md#node-121-transparent-original-scenario-tracing).

NODE-120: Retained native driver failures now carry bounded phase attribution. Both profiles pass a controlled Page.enable response-timeout test and full 18-case listener corpus. A native Page-close absence-check exhaustion also recurred; neither natural failure is declared fixed. Next capture the naturally failing method/session and close-confirmation timeline before changing deadlines.
[Close attribution contract and evidence](knowledge/pwright-browser-close.md#node-120-attribute-retained-native-driver-failures).

NODE-119: Ready/linkage source integration is implemented. Both final profiles pass the full 7-case lifecycle and 18-case listener corpora, 39 prior regressions, Frame-tree 18/18 and lookup 41/41 plus three boundaries. A combined mutation repetition failed in browser.close with a driver-response timeout; two subsequent planned repetitions passed. Diagnose that failure without assuming it is NODE-118's absence-check issue. The overall Node compatibility matrix remains open.
[Native ready-source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-119-native-ready-and-popup-source).

NODE-118: Ready/popup wrapper conversion is implemented and unit-tested. Next join authoritative PageReady/PopupLinked effects to native groups, preserve exact visibility across full snapshots, pre-admit source capacity, and run the whole differential corpus. Do not count conversion alone as completed creation-event support.
[Creation-conversion contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-118-precommit-creation-wrapper-conversion).

Also retain the NODE-118 native close-confirmation stability task: two initial
default corpus cases exhausted the existing 20 checks with 10-ms pauses. Final
corpora and three repetitions per scenario/profile passed without changing checks;
that is not a root-cause fix. Preserve those failures while investigating an
appropriate injected completion/deadline policy and its cancellation/resource proof.

NODE-117: External Page-close delivery now matches the Worker-confirmed upstream scenario in both profiles. Ready/creation and popup linkage remain next; admit wrapper identities before publication and preserve dependent-event ordering. The overall Node compatibility goal remains active.
[External-close source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-117-pre-admitted-external-close-source).

NODE-116: Explicit Page/Context close sources now have real Node delivery. Next join external committed disposal and ready/linkage sources, retaining exact planned views and pre-admission. Default live evidence is 14/18 listeners and 4/7 lifecycle-event cases; creation/popup/external Page close remain unfinished.
[Explicit-close source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-116-explicit-close-source-integration).

NODE-115: Node terminal dispatch is implemented; ordinary ready/linkage and explicit-close source joining remains next. Preserve exact planned collection visibility when composing those sources. Do not mark the overall Node compatibility goal complete.
[Terminal dispatcher contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-115-environment-thread-terminal-dispatch).

# Pwright Rust v2 design tasks

NODE-114: Implemented and exercised the terminal native source. Next join ready/linkage and explicit-close receipts to exact planned visibility, then integrate one environment-owned FFI dispatcher. The current terminal entry point is an experimental partial composition, not completion of the Node compatibility goal.
[Terminal source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-114-pre-admitted-native-terminal-source).

NODE-113: Implemented the bounded group/cursor primitive. Next join driver and explicit-close receipts to planned collection views, pre-admit terminal groups at creation, then route every FFI fence through one environment-owned callback port. Do not stage groups in the selected host while legacy whole-fence delivery remains.
[Group contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-113-bounded-dependent-publication-groups).

NODE-112: Explicit Page/Context close effects now have a two-phase, opt-in native input.
Next integrate ready/linkage, explicit close and saturation-safe terminal inputs
into one bounded dependent journal, then join intermediate visibility and the
exclusive publication drain to unlocked Node delivery.
[NODE-112 close receipt contract and verification](knowledge/pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: The borrowed driver-effect input is implemented and tested. Next, preserve the
native prepared Page/Context close effects currently discarded by direct service
commits, with pre-admission and post-commit publication. Then integrate bounded
dependent groups and unlocked Node delivery; final-state diffing is insufficient.
[NODE-111 source contract and verification](knowledge/pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-110 characterizes listener mutation, async gates, reentrant close and
synchronous/asynchronous failure channels; it fixes the supported Browser
notification's synchronous exception channel. Next: native dependent cause
groups, per-group completion/failure receipts, intermediate visibility and
bounded foreign delivery. Five missing-event scenarios remain RED.
[NODE-110 contracts, fix and verification](knowledge/pwright-node-lifecycle-events.md#node-110-listener-failure-boundaries-and-scoped-exception-reporting).

NODE-109 implements and tests the exclusive, fixed-watermark Rust queue owner.
Remaining: native lifecycle cause journal, atomic callback-state transactions,
listener throw/reentry characterization, unified FFI fence delivery and bounded
foreign port admission. Do not mark Node lifecycle-event compatibility complete
from this lower-level ownership primitive.
[NODE-109 contract and verification](knowledge/pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-108 adds precise bounded retirement receipts and revalidates existing
retirement handling: CDP 91, probe suites 19 each, fresh native layouts 6/6 each,
controlled Node/upstream retirement 6/6 default and 10/10 combined. No startup
stall reproduced in these fresh traces; previous failures remain unexplained.
Continue the admitted Rust event-step owner/native cause journal and listener
failure/reentry design. Use exact receipts for another reproduced attach stall;
do not replace the existing typed SessionDetached path based on aggregate counts.
[NODE-108 receipts and verification](knowledge/pwright-chromium-topology-events.md#node-108-bounded-native-lifecycle-identity-receipts).

NODE-107 handoff: Rust tests 304/352 and strict checks pass, but the rebuilt
combined live gate is incomplete due to initialization timeouts. The real-Rust
combined probe reproduces four observer-layout timeouts, narrowing outstanding
commands to Page.enable / Network.enable on other targets. Next characterize
retirement and observer interactions without restarting shared Chrome or extending
deadlines. The admitted event step owner/journal/foreign dispatch remains open.
[Evidence and outstanding work](knowledge/pwright-node-lifecycle-events.md#combined-profile-reproduction-and-incomplete-live-gate).

NODE-107 completes validated cross-revision view rebasing, including foreign
domain rejection, metadata/order preservation, hidden additions, full-capacity
replacement and final-owner release. Next admit the one-shot fixed-watermark
step owner and native cause journal, characterize listener failures/reentry,
and then route every foreign fence through it. Preserve NODE-106's unexplained
discovery timeout; neither this primitive nor a rerun marks that issue fixed.
[NODE-107 contracts and evidence](knowledge/pwright-node-lifecycle-events.md#node-107-validated-cross-revision-visibility-rebasing).

NODE-106 completes the immutable single-revision visibility building block and
layout measurements, with 14 new Rust tests (284 default / 332 combined passed).
Selected readers use the view but full fences remain unchanged. Next design and
admit old/new/pending revision overlap and the one-shot step owner, characterize
throwing listeners/reentry, then wire all publication fences and typed notices.
Do not mark lifecycle callbacks or the full goal complete from getter tests.
Live regressions passed apart from an initial combined borrowed-discovery
readiness timeout (6/7); unchanged repeat passed 7/7 and targeted case 1/1.
Keep that intermittent timeout unresolved. Both lifecycle candidates remain
0/7 versus the freshly verified 7/7 reference. No task-owned runtime remains.
[Implementation status](knowledge/pwright-node-lifecycle-events.md#node-106-immutable-single-revision-visibility-foundation).

NODE-105 completes reference characterization and a reproducible RED event gate:
7/7 upstream; 0/7 on both candidates, with final state equality. Page/Context
close callbacks need intermediate Rust-published state, not final-state replay.
Next prototype/admit the stepwise Rust publication representation, characterize
listener failure/reentry, then integrate all command/watch fences and typed
Page/popup/close notifications. Existing Page-event/opener capabilities are reuse
points. No production implementation/rebuild or commit/push in this checkpoint;
full goal remains active.
[Design and required verification](knowledge/pwright-node-lifecycle-events.md).

NODE-104 completes seven creation/discovery collection-publication observations:
corrected reference, default and combined each pass twice. No production patch or
addon rebuild was needed; initial failures were ordering and FFI-lane harness
assumptions, preserved separately from final evidence.
Next characterize and implement bounded Rust-owned Context page, Page/Context
close and popup events with exact handler-visible state/order and cleanup.
Current getters alone do not prove event compatibility. Overall goal stays active;
no staging, commit or push.
[Checkpoint and limits](knowledge/pwright-frame-publication.md#node-104-creation-and-discovery-publication-characterization).

NODE-103 implements the NODE-102 producer/getter integration and resolves all
six NODE-101 early-removal discrepancies on both rebuilt profiles. Rust batches
provide atomic publication; creation guards and retirement pools pre-admit work.
Host release tests pass 270/318, collection differential 7/7 per profile, and
the scalar/Context/enumeration/Worker regressions pass. The additional synchronous
baseURL TypeError mismatch is fixed in Rust; getter identity remains intact.
Next characterize creation/discovery visibility while Node is blocked, then
continue close/crash event ordering and remaining API discrepancy gates.
No staging, new commit or push. The overall Node-compatible Rust goal remains active.
[Exact checkpoint and verification](knowledge/pwright-frame-publication.md#node-103-rust-collection-producers-and-published-node-reads).
NODE-102 next-step/artifact statements below are historical, now superseded.

NODE-102 implements and verifies collection storage/fence primitives: explicit
pre-activation reservations, charged old/current overlap, bounded identities and
capacity, terminal/discard reclamation, no strong queue/host retention. Eighteen
new Rust tests pass; host default/combined totals are 263/311.
Next integrate native creation/discovery/retirement producers and selected Node
collection getters with atomic Page-close membership publication, then rebuild
and rerun the unchanged NODE-101 red gate. Storage tests are not browser parity.
No staging, new commit, push or installed addon replacement; goal stays active.
[Code, proof and concrete next integration steps](knowledge/pwright-frame-publication.md#node-102-bounded-rust-collection-revision-storage).

NODE-101 completes characterization of Context/Page collection publication:
six reproducible early-removal discrepancies on both NODE-100 artifacts; only
the live control matches. Upstream passes the corrected seven-case lane twice.
Next implement the Rust-owned published membership projection, with pre-admitted
storage/terminal work, fixed fences and uncached retained descriptors, then turn
this unchanged red gate green and rerun resource/Worker/enumeration regressions.
Creation/discovery timing and public close-event ordering remain open gates.
No production change, native rebuild, staging, commit or push in this checkpoint.
[Evidence and concrete implementation requirements](knowledge/pwright-frame-publication.md#node-101-context-and-page-collection-publication-discrepancy).

NODE-100 completes the bounded Browser.isConnected publication audit and fix:
Rust owns readiness, published state and duplicate suppression; default/combined
differential lanes pass 5/5 and Page.isClosed regressions 8/8 each. Rust host
tests pass 245/293, Node units/types/boundaries and strict Clippy pass.
Next characterize Browser.contexts() / BrowserContext.pages() under blocked Node
shutdown. Native-index timing is only a hypothesis until independently tested.
The full goal stays active; no staging, new commit or push.
[Contract and verification](knowledge/pwright-browser-close.md#node-100-rust-owned-browser-disconnect-publication).

NODE-099 implements Rust-owned Page.isClosed with pre-admitted terminal queue
slots, fixed Node publication fences and allocation-free native teardown. Both
final rebuilt profiles pass the original 8/8 gate and Page enumeration 7/7.
Final hardening rejects a previously allocated unadmitted queue; the combined
Rust gate passes 286 tests and strict Clippy; default passes 238. Worker teardown
passes 3/3 on both final artifacts; Node units/types and boundaries pass.
Next audit Browser.isConnected at the same event-loop boundary, then continue
Page event/API work. The full goal is not complete; no staging, commit or push.
[Contract and verification](knowledge/pwright-page-close.md#node-099-rust-owned-page-close-publication).

NODE-098 establishes the Page.isClosed contract with eight real-browser oracle
cases, including crash, borrowed disconnect and a blocked Node event loop.
Reference passes 8/8 twice; default/combined candidates fail 0/8 solely because
the API is missing. This is a red test/design checkpoint, not implementation.
Next prove bounded terminal close publication in Rust, then expose the getter
and rerun this unchanged gate. Do not publish the native close flag directly.
No Rust rebuild, new commit or push in this checkpoint.
[Contract, evidence and implementation gate](knowledge/pwright-page-close.md#node-098-pageisclosed-reference-contract-and-failing-candidate-baseline).

NODE-097 isolates Page-scoped auto-attach by a second client as a native paused-
iframe retirement trigger. Pinned Playwright and Rust show the same timeout and
explicit recovery; the direct matrix passes combined/default 11/11. Keep the
old uncoordinated stress failure, but do not treat this native signature alone as
a Rust discrepancy or alter production timeouts. Next implement Page.isClosed
from Rust-owned lifecycle state, then continue broader API/Worker coverage.
[Evidence and scope](knowledge/pwright-chromium-topology-events.md#node-097-page-auto-attachment-from-a-second-client).

NODE-096 adds bounded timing/native-retirement evidence and a four-combination
borrowing-peer disconnect control. It corrects the earlier interpretation:
one matching load arrives AFTER deadline-triggered stopLoading, not before.
The controlled combined peer probe passes 4/4; churn overlap still fails.
Next isolate peer attachment during paused OOPIF initialization and native
replacement. Rust production code and deadlines are unchanged; no new commit
or push. [Checkpoint](knowledge/pwright-chromium-topology-events.md#node-096-navigation-timing-and-borrowing-peer-control).

NODE-095 proves exact detach events can arrive while Rust initialization still
waits for native replies. A controlled baseline fails both new pre-receipt
retirement cases. Selected bounded pending-call retirement is implemented;
rebuilt combined/default attachment cases pass 10/10 and 6/6, Worker controls
3/3 each and queued Contexts 2/2 each. Native regressions pass 86 CDP, 375
composition and 274 host tests. Headers overlap passes 35/35, but two child
runs pass only 6/8 each. Next investigate navigation timeout timing relative to
exact main-loader load with no native calls pending, plus replacement
iframe/evaluate progress. Preserve wider API/Worker work; the goal is not complete.
[Checkpoint](knowledge/pwright-chromium-topology-events.md#node-095-native-detach-before-an-initialization-response).

NODE-094 reproduces queued disposed-Context root-detach failure and corrects the
selected Playwright fallback, preserving strict-mode behavior and borrowed
Context ownership. The expanded old-artifact baseline is 1/2; rebuilt default
and combined controls pass 2/2 each. Rust regressions pass 374 composition and
274 host tests. Repeated overlap passes headers 35/35 but child headers 6/8,
including candidate navigation/disconnect timeouts after Pwright builds finish.
Next inspect outstanding native commands and event-driver ordering at those
timeouts; do not increase deadlines or infer closure from arbitrary CDP errors.
Full compatibility remains open.
[Checkpoint](knowledge/pwright-chromium-topology-events.md#node-094-queued-attachments-from-disposed-contexts).

NODE-093 resumes implementation after the user's explicit go-ahead. Typed
header-initialization retirement is corrected after a 7/8 failing baseline;
combined attachment 8/8 and headers-only 6/6 now pass. The root-detach race
remains open; no full compatibility claim or new commit/push is made.
[Checkpoint](knowledge/pwright-extra-http-headers.md#node-093-typed-header-initialization-retirement).

NODE-092 addresses the demonstrated second-stage -32602 child-detach race in
Rust. Native tests pass 79; the initial paired retired/live Worker lane passes
2/2. The separate concurrent disconnect timeout and other compatibility gaps
remain open. [Checkpoint](knowledge/pwright-chromium-topology-events.md#node-092-child-disappears-between-resume-and-detach).

NODE-092 public status: default 2/2, combined 2/2 plus repeat, serial child headers
8/8 and attachment 7/7. The deliberately overlapping run still fails: headers
35/35, child headers 4/8. Next inspect strict raw target_detach callers in Page
attachment/cleanup and native-world retirement with exact native ownership gates;
do not treat all -32602 strings as idempotent cleanup.

Final NODE-092 native/host regressions pass 79 CDP, 373 composition and 274 host
tests; Node 47, strict types and dependency boundaries pass. All build/test handles
are terminal and owned browser fixtures are cleaned up. Goal remains active:
the failing concurrent gate and full API coverage are not complete.

NODE-091 implements native session-close watches for header fanout after a 4/6
baseline exposed Page/Context rollback discrepancies. Follow up on blocked-driver
receipt/detach ordering, crashes/replacement, Worker shutdown, initializer native
error preservation and Page.isClosed. The foundation commit remains local; no
push. [Evidence](knowledge/pwright-extra-http-headers.md#node-091-native-session-closure-during-header-fanout).

Final NODE-091 gates: combined 8/8 plus an 8/8 repeat; headers-only 8/8;
header regression 35/35; attachment 7/7; host 274 pass (9 ignored). Preserve the
overlapping-run disconnect timeout and native -32602 attachment error as open
reliability work. No task-owned background process remains.

NODE-090 fixes demonstrated missing-session failures in discovered/routed native
initialization and child cleanup, including never-initialized lifecycle owners.
Public tests pass 7/7 plus three repeats, headers 35/35, CDP 78, composition 373
and host 269. Next: real OOPIF/replacement/Worker retirement, other Page-local
initialization errors, compensation/shutdown lifetime and broader API coverage.
The original foundation commit remains local; no push is authorized.
[Checkpoint](knowledge/pwright-chromium-topology-events.md#node-090-missing-session-retirement-before-page-publication).

NODE-089 final gates pass 35/35 public tests and three 4/4 held-receipt repeats;
269 host tests and isolated live cleanup pass. Preserve the earlier missing-
session connect failure as a remaining attachment-under-churn task. No new
commit/push or shared-process cleanup was performed.

NODE-089 corrects Context Page sequencing, deferred later-Page snapshots and
native protocol-error projection. Four controlled public success/failure cases
pass; release host tests pass 269. Next: OOPIF/session-close behavior, Page-order
and route handoff under churn, compensation failure and shutdown retirement.
The broad compatible Node API goal remains active, not complete.
[Checkpoint](knowledge/pwright-extra-http-headers.md#node-089-native-receipt-gates-and-context-page-sequencing).

NODE-088 implements bounded caller-polled native header fanout and early primary
result with retained compensation. Host tests pass 266 and combined public header
tests 31/31. Next: controlled real-CDP/Node early-failure timing, session-close
suppression, attachment handoff and shutdown retirement/foreign-waker audit; typed
contracts, Worker/OOPIF coverage and the broader compatibility goal remain open.
[Checkpoint](knowledge/pwright-extra-http-headers.md#node-088-caller-polled-native-jobs-and-early-primary-rejection).

NODE-087 implements synchronous Rust setter/navigation preparation, queue-before-
publication admission and frozen per-Page native plans; combined public tests pass
30/30. Next: concurrent driver progress, attachment handoff and early primary
settlement with bounded compensation ownership; these are not completed by the
logical ordering fix. Broader Node compatibility remains the unchanged goal.
[Checkpoint](knowledge/pwright-extra-http-headers.md#node-087-synchronous-rust-preparation-and-frozen-dispatch).

NODE-086 reproduced an unawaited-setter/Frame.goto race. The bounded overlapping
metadata model passes 53 tests and 128 pinned internal traces; the host remains
single-writer. Next: Rust call-order preparation, per-dispatch snapshots, bounded
concurrent driver progress and early primary settlement with tracked compensation.
The new public regression is intentionally retained, not marked resolved.
[Design and evidence](knowledge/pwright-extra-http-headers.md#node-086-pending-setter-race-and-bounded-overlapping-model).

NODE-085 wires the first-original-Page Referer rule into public Page/Frame.goto;
combined selected tests pass 24/24. Remaining header work includes pending setter
races, native fanout/early rejection, worker/OOPIF wire behavior, typed contracts
and creation fault/cancellation verification. Full navigation parity remains open.
[Checkpoint](knowledge/pwright-extra-http-headers.md#node-085-navigation-referrer-reconciliation).

NODE-084 implements selected Context creation headers in the Rust transaction;
header-only release passes 21/21 public cases. Remaining work includes creation
fault/cancellation paths, navigation referer reconciliation, typed header contracts,
concurrent native fanout and early rejection, worker/OOPIF/event coverage. Do not
mark the full Node compatibility goal complete on this subset.
[Checkpoint](knowledge/pwright-extra-http-headers.md#node-084-context-creation-headers).

NODE-083 Context-creation investigation reproduced three early-syntax failures
and a shared-attachment shutdown on invalid Page initialization. Rust desired
storage now defers syntax validation; expected initialization rejection closes
only the new Page and keeps Context usable. The expanded header-only lane passes
15/15, including 18 failed-Page retirements. Context creation's header option is
still next, not completed by this prerequisite correction.
[Checkpoint and remaining scope](knowledge/pwright-extra-http-headers.md#node-083-deferred-syntax-and-page-local-initialization-failure).

NODE-082 implements selected N-API/public Page and Context header setters,
eight composable artifact selections and a supervised public comparison lane.
Two observed discrepancies (premature boxed conversion and conversion-error
prefixing) were fixed; the final header-only run passes 10/10 tests. Next work
remains Context header creation options, navigation referer integration, native
fanout/compensation/cancellation timing and OOPIF/workers, not a full-parity claim.
[Detailed checkpoint](knowledge/pwright-extra-http-headers.md#node-082-public-selected-setters-and-language-phase-differential).

NODE-081 installs optional Rust-native header setters and attachment initialization.
Two isolated Chrome runs pass nine wire observations, 18 Page retirement cycles
and zero-owner shutdown. An initial stale-session header failure was reproduced
and addressed with acknowledged obsolete-session clearing. Host tests: 231 pass,
seven existing ignored; model: 46 pass; strict selected Clippy passes. Next:
concurrent fanout/compensation timing, Context options, navigation referer,
workers/OOPIF gates, typed N-API and selected public differential artifacts.
[Native installation and remaining work](knowledge/pwright-extra-http-headers.md#node-081-selected-rust-host-and-native-session-reconciliation).

NODE-080 implements the admitted Rust Context/Page settings registry and
proposal/restore/acknowledgement lifecycle. Tests pass 45/45; internal setter
metadata matches 54/54 and merge/referrer matches 484/484. Next: actual host
selection, native fanout/compensation, typed FFI setters and public browser gates.
[Transaction boundary](knowledge/pwright-extra-http-headers.md#node-080-bounded-desired-settings-transactions).

NODE-079 implements identity-aware native initialization before Page readiness.
Three focused tests and 335 composition tests pass (15 existing tests ignored).
Next remains the actual admitted settings registry, transactional fanout/rollback,
worker routing and public FFI setters; no header side effect is implemented yet.
[Scoped integration boundary](knowledge/pwright-extra-http-headers.md#node-079-identity-aware-native-initialization).

NODE-078 verifies the optional Rust extra-header policy (25 crate tests,
484 internal comparisons) and repeats 23 public upstream observations.
Next: identity-aware session initialization, admitted settings transactions and
native rollback, followed by typed host/FFI setters and paired wire tests.
The header Node API is not implemented; the full goal stays active.
[Required integration evidence](knowledge/pwright-extra-http-headers.md#native-integration-plan-and-required-evidence).

NODE-077 verifies post-execution close and timeout-then-browser-continuation in
both native profiles. Each has 27 paired matches; the arrival-close comparison
still fails, and Rust independently reproduces TargetStillOpen in one run.
Worker-owned pending/queued cleanup and borrowed-Page preservation pass 3/3
per profile. These are additional evidence, not a production close fix or full
Worker coverage. Next: header/referer reconciliation, exact errors/redirect
adoption and continued native close-boundary investigation; full FFI goal active.
[Evidence](knowledge/pwright-frame-navigation.md#node-077-causal-close-timeout-continuation-and-worker-teardown).

NODE-076 exposes selected public Node Frame.goto, bounded FFI conversion,
Context baseURL reuse and exact Response.frame identity. Independent and
combined native profile selection has unit/type/dependency and strict Clippy
coverage. Both profiles pass 23 paired Frame cases and independent Rust close,
but upstream held-response Page.close exceeds its diagnostic deadline. Combined
drag/Keyboard/Locator passes 10/10. Keep the close comparison failed and
investigate its causal boundary. Header reconciliation, exact navigation errors/redirects,
Worker teardown and remaining Playwright coverage are still pending.
[Current checkpoint](knowledge/pwright-frame-navigation.md#node-076-public-node-frame-navigation).

NODE-075 adds typed Rust host Frame.goto values and bounded response projection.
An independent admitted result owner survives host shutdown; the real Chrome
HTTP/hash/404/OOPIF/cancellation/escaped-result gate passes. This is not a public
Node method or generated-service installation. Next are Context option
reconciliation, foreign-copy admission/Response.frame identity and pinned
public differential tests; the full Node FFI objective remains active.
[Checkpoint](knowledge/pwright-frame-navigation.md#node-075-typed-rust-host-result-ownership).

NODE-074 completes the opt-in navigation 1.2 schema, generated models/server,
all four typed command/result codecs and Rust Frame client extension. Root
Page-only navigation stays 1.1 even with unified feature selection. 109 tests,
strict selected Clippy, generation freshness and the 15-package legacy boundary
pass. Next: connect the selected Rust host executor to this contract, then
admit foreign results/errors and expose public Node Frame.goto with pinned
differential verification. This is not completion of the FFI goal.
[Checkpoint](knowledge/pwright-frame-navigation.md#node-074-opt-in-typed-navigation-12-contract).

NODE-073 joins exact native Frame dispatch to the Rust host's ordered waiter,
absolute deadline, host shutdown and final publication fence. Explicit admission
caps eight executor owners independently of the shared resource plan. 670 Rust
units, strict selected/readiness checks, the 131-observation internal oracle and
live HTTP/hash/404/OOPIF/cancellation checks pass. Two preceding attachment
failures remain recorded and unexplained. Generated commands, precise foreign
error/response ownership and public Node FFI differential work remain.
[Checkpoint](knowledge/pwright-frame-navigation.md#node-073-rust-host-dispatch-scheduling-and-cancellation).

NODE-072 integrates admitted navigation registrations into the production Page
lifecycle lock and adds a distinct Rust host `frame-navigation` selection.
651 Rust units, strict selected/readiness Clippy, the 131-observation oracle and
actual host HTTP/hash/shutdown checks pass. Host-owned native dispatch and
pending-call cancellation, generated Frame commands and public FFI gates remain.
[Checkpoint](knowledge/pwright-frame-navigation.md#node-072-ordered-operation-registration-and-actual-rust-host-observation).

NODE-071 implements the bounded Rust navigation matcher/response owner: 67 leaf
unit tests and 29 pinned internal traces (131 observations) pass. Host operation
registration/scheduling, native cancellation, redirected-navigation adoption,
generated Frame commands and public FFI gates remain open.
[Checkpoint](knowledge/pwright-frame-navigation.md#node-071-bounded-navigation-matching-and-response-retention).

NODE-070 bounded multi-Page lifecycle composition and opt-in Rust host
installation pass 574 Rust tests, strict combined-feature Clippy and the real-host
Chrome gate, including escaped shutdown lease release. The host sink is explicitly
readiness-only, not the missing navigation response/waiter implementation.
[Checkpoint](knowledge/pwright-frame-navigation.md#node-070-bounded-multi-page-registry-and-opt-in-rust-host-selection).

NODE-069 implements an opt-in production ordered lifecycle observer and admitted
one-Page owner. Two real-Chrome document lifecycle gates pass after exposing and
fixing changed-loader replacement snapshots; 385 Rust units and 93/93 pinned
internal lifecycle snapshots pass. Strict Clippy and cleanup pass. This advances
the full Frame.goto
path; multi-Page/Worker ownership, waiter/response/cancellation semantics,
generated commands and Node host/public differential work remain open.
[Current checkpoint](knowledge/pwright-frame-navigation.md#node-069-opt-in-production-ordered-lifecycle-observation).

NODE-068 fixes the production driver/topology document-commit retirement gap.
Two live cases verify same-origin descendant disposal and cross-origin renderer
handoff through retained navigation access, the real driver and runtime.
Verification passes 378 units, strict Clippy, both Node host/addon source
configurations and the unchanged five-package topology dependency proof.
Next install the bounded lifecycle owner with its initialization fence and
response/cancellation ownership; portable Frame.goto and Node FFI remain open.
[Evidence and pending gates](knowledge/pwright-frame-navigation.md#node-068-production-document-commit-topology-retirement).

NODE-067 implements bounded native CDP-to-Rust lifecycle normalization and passes
41 navigation units, strict Clippy and a live local/nested Chromium lifecycle test.
Document responses, descendant pruning and hash preservation are exercised through
the real transport. This is not public Frame.goto: next integrate initialization,
topology/session handoff and operation response/cancellation ownership, then the
portable command and Node FFI. The full goal remains incomplete.
[Scope, evidence and remaining gates](knowledge/pwright-frame-navigation.md#node-067-bounded-native-lifecycle-normalization).

NODE-066 implements opt-in retained Frame navigation routes in the production
composition transaction, with budgeted snapshots and permanent observed
revocation. Native initiation delegates to the existing isolated adapter; worlds
are not required. Runtime rollback and in-flight reply races have dedicated tests.
All eleven new tests pass; 388 Rust units across four affected crates, strict
Clippy, both host/addon source configurations, 46 Node units and type/boundary
checks pass. Fifteen ignored native tests remain unexecuted; addons are unchanged.
Next connect native lifecycle/document/response normalization and cancellation,
then generate/expose Frame.goto over FFI and verify the public API.
[Contract and verification status](knowledge/pwright-frame-navigation.md#node-066-retained-routes-in-the-production-topology-transaction).

NODE-065 adds an opt-in, pre-admitted Rust Frame lifecycle reducer. It matches
93/93 pinned internal snapshots across 11 traces; 31 navigation unit tests pass.
Next wire exact retained routing and bounded native-event normalization, then
document/response matching, cancellation, the portable command and Node FFI.
The full goal and public Frame.goto remain unfinished.
[Contract and current evidence](knowledge/pwright-frame-navigation.md#node-065-optional-normalized-lifecycle-reducer).

NODE-064 implements borrowed world-independent Frame navigation routes and
explicit native initiation. Six pinned local/nested/OOPIF observations match;
21 navigation and 46 topology unit tests pass. No public Frame.goto is exposed.
Next: admitted owned invocation, persistent lifecycle/subtree observation,
distinct generated Frame command, cancellation and Node response identity;
then rebuild the selected addon and run the public differential corpus.
[Full design and remaining gates](knowledge/pwright-frame-navigation.md).

NODE-063 implements public page.frame name/glob/RegExp/predicate lookup through
Rust selection and scoped N-API values. The pinned public gate passes 41/41,
three resource groups, Page 7/7 and Frame 18/18. Next: remaining Frame APIs/events,
wider differential coverage and the open connection-reliability investigation.
The broad Node FFI goal is not complete.
[Contract, limits and evidence](knowledge/pwright-frame-lookup.md#node-063-public-lookup-over-the-rust-kernel).

NODE-062 Context baseURL is implemented and verified: 16/16 pinned public
observations, four resource/lifetime groups, Page 7/7 and glob 1647/1647 plus seven
checks on the final selected addon. Context metadata returns to its 560-byte
fixed index after last GC. Next integrate public page.frame matcher/cache behavior;
Frame.goto and broader API parity remain open. Connection reliability is not fixed.
[Contract and evidence](knowledge/pwright-context-creation.md#verification-status-and-remaining-gates).

NODE-061 refreshes original-layout glob/Page gates on the unchanged artifact and
corrects the existing bounded attachment diagnostic. Reliability remains open;
no production scheduler or timeout was changed. Next implement Rust-owned Context
baseURL with pre-creation admission and consistent enumeration/retained ownership,
then complete lookup matcher/cache integration and public differential tests.
[Current evidence](knowledge/pwright-chromium-topology-events.md#node-061-reuse-the-bounded-attachment-collector).

NODE-060 implements the internal native URL-glob conversion port. A labelled
late-observer run passes 1647 comparisons and seven boundary checks; original
attachment/Page regression failures remain open. Next: Context baseURL ownership,
full matcher/cache integration, Node parser memory attribution and attachment
reliability. This is not public page.frame support.
[Contract and evidence](knowledge/pwright-frame-lookup.md#node-060-actual-n-api-url-value-conversion-port).

NODE-059 implements bounded Rust glob preprocessing/restoration; 1647 exact
patterns match the pinned oracle using a diagnostic Rust URL parser. Next admit
the production parser and Context baseURL ownership, then complete N-API
matcher/exception/identity integration and full public lookup/order coverage.
The diagnostic parser does not satisfy the production resource gate.
[Contract and evidence](knowledge/pwright-frame-lookup.md#node-059-bounded-glob-preprocessing-and-borrowed-url-resolution).

NODE-058 implements the Rust FrameLookup kernel and a pre-admitted resolved-glob
compiler; 2810 patterns match the pinned oracle exactly. Next complete bounded
baseURL preprocessing and Context ownership, then N-API matcher/exception/cache
integration and full public lookup differential tests. The existing name-only
API and these kernel tests do not complete lookup.
[Implementation, limits and evidence](knowledge/pwright-frame-lookup.md#node-058-rust-traversal-and-bounded-resolved-glob-compiler).

NODE-057 pins the next [Frame lookup implementation gates](knowledge/pwright-frame-lookup.md)
with 14 passing oracle-only characterization tests. The moving upstream checkout
differs from Playwright 1.58.2 in RegExp state, glob validation and URLPattern;
implement against the pinned target. BaseURL ownership and multi-Frame first-match
ordering remain required dependencies, not silent omissions.
The unchanged addon now passes Mouse 44/44 in both experimental background and
original modes; [earlier intermittent failures remain unexplained](knowledge/pwright-raw-mouse.md#node-057-setup-experiment-not-a-causal-fix).

NODE-056 implements Rust-backed `Frame.name()`, optional bounded name storage
and atomic navigation publication. The shared local/OOPIF Frame gate passes
18/18. Next: full `page.frame` lookup semantics (including URL match forms),
Frame event delivery/order, Locator/Handle activation and attachment diagnosis.
The current Mouse regression is also open: extra moves appear without input
requests; bounded idle windows suggest, but do not prove, an initial-event race.
[Contract and evidence](knowledge/pwright-frame-publication.md#node-056-bounded-navigation-names-and-public-framename).

NODE-055 completes the Page-descriptor read/boxing gate and cache-aware public
enumeration. Its 7-case shared workload includes popup discovery and a stale
cache larger than the live Page limit. Node Page/Frame value evaluation now
selects explicit transient activation. Next: Frame name/lookup/event metadata,
Locator/Handle activation parity and the unresolved intermittent attach issue.
[Contract and evidence](knowledge/pwright-frame-publication.md#node-055-admitted-page-descriptors-and-cache-aware-enumeration).

NODE-054 closes the public new-Page root-binding exhaustion gate with a real
before/after target-leak reproduction and pre-native Rust reservation. Frame,
Mouse and drag regressions now pass on the rebuilt addon. The attachment matrix
does not reproduce earlier timeouts; preserve that intermittent investigation.
Next gate general Page-descriptor conversion/boxing and add Frame name/lookup
metadata before designing public Frame event delivery and ordering.
[Contract and evidence](knowledge/pwright-frame-publication.md#node-054-creation-time-root-binding-admission).

NODE-053 completes admitted N-API collection reads, retained Frame bindings and
public `frames`, parent/child and detach methods. Ten settled-state public
comparisons pass on local/OOPIF fixtures, with exact cleanup and finalization
checks. First diagnose connection-initialization timeouts preventing final
Mouse/drag regression results and gate wrapper exhaustion during new-Page
construction. Then design Frame name/lookup metadata and event delivery/ordering;
complex initial-attachment collection order remains an explicit parity gate.
[Contract and evidence](knowledge/pwright-frame-publication.md#node-053-public-node-frame-collections-and-retained-wrappers).

NODE-052 completes host topology projection, pre-native Page metadata admission
and the real local/OOPIF metadata gate. It also fixes two attachment defects
exposed by owned-Page navigation, with a rebuilt selected addon and input
regressions passing. Next: admitted N-API conversion and public Frame wrappers;
full compatibility remains open.
[Evidence and remaining gates](knowledge/pwright-frame-publication.md#node-052-live-host-projection-and-paused-oopif-attachment).

NODE-051 completes the bounded Frame metadata kernel and shared URL/tree
publication fence (27 new tests). Next connect the authoritative host topology,
budget N-API conversion and expose stable public Frame wrappers. The upstream
identity probe is not yet a paired candidate conformance result.
[Contract and integration gates](knowledge/pwright-frame-publication.md).

NODE-050 completes the 18-case native Frame lifecycle gate and exposes a separate
opt-in Node addon build. Actual selected Node parity is 44/44 Mouse, 24/24
Keyboard and 10/10 drag/Keyboard/Locator. Next verify pending native cancellation
during owner destruction and the broader Node corpus; full compatibility is open.
[Selection, evidence and remaining work](knowledge/pwright-chromium-drag.md#node-050-native-lifetime-gates-and-selected-node-addon).

NODE-049 fixes the post-focus Frame detach discrepancy with an optional atomic
Frame-to-Page lifetime handoff. The expanded native gate matches 12/12. Next:
native pre-focus detach and Page/Context destruction during the continuation;
public child-Frame wrappers and selected addon parity remain open.
[Contract, evidence and remaining gates](knowledge/pwright-chromium-drag.md#node-049-optional-frame-to-page-lifetime-handoff).

NODE-048: real local/OOPIF Frame coverage exposes a mandatory failing lifetime
gate (8/10 match). Post-focus Frame detach currently cancels the Rust command,
but Playwright keeps Page input pending until its deadline. Next implement an
explicit Frame-to-Page continuation handoff, not error normalization.
[Requirements and evidence](knowledge/pwright-chromium-drag.md#node-048-native-frame-gate-exposes-a-lifetime-discrepancy).

NODE-047 connects Locator press/sequential input to the exact owning Page drag
controller. Ten native host/oracle scenarios match, including delayed-press
timeout. Real child/OOPIF and native destruction/interruption gates remain;
public addon drag is still unselected. [Details](knowledge/pwright-chromium-drag.md#node-047-locator-keyboard-selects-the-owning-page).

NODE-046 implements selected Rust Keyboard Escape cancellation, with five native
oracle matches and retained cancellation owners. Next gates include selecting the
same provider for Locator-owned input and exact Page/Context native retirement;
public drag is not enabled. [Evidence](knowledge/pwright-chromium-drag.md#node-046-optional-rust-keyboard-escape-interception).

NODE-045 resolves the maximum-click slot/pending-command mismatch and rebuilds
the default Node addon. Host raw Mouse is 25/25; public Node is 39/44.
Escape, destruction and overlapping drag integration remain required.
[Evidence](knowledge/pwright-chromium-drag.md#node-045-maximum-click-batches-and-matching-host-admission).

NODE-044 implements opt-in Rust HostSession drag dispatch and close cancellation.
Before selecting it in the addon, resolve Escape, Page/Context destruction,
high-count/overlap admission and repeat public differential tests. This is an
integration checkpoint, not full Mouse parity or goal completion.
[Details](knowledge/pwright-chromium-drag.md#node-044-opt-in-hostsession-mouse-integration).

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

NODE-039 adds the caller-polled intercepted-movement composition (**57** unit
tests; **9** native cases). Next implement Page drag state and the real Mouse
dispatcher/live-state callback adapter, bounded Frame snapshot source and host
ownership/teardown. The composed helper is not public Node parity.
[Current scope](knowledge/pwright-chromium-drag.md#node-039-composed-intercepted-movement).

NODE-038 implements fixed-session capture ownership, fresh event subscriptions,
local checked epochs and retained interception enable/disable. **17** new tests
(**44** total) and the **6** native listener cases pass. Next compose the actual
move callback, Frame snapshot and Page drag state; then select the provider in
the host and rebuild/rerun public gates.
[Capture boundary](knowledge/pwright-chromium-drag.md#node-038-retained-capture-lifecycle).

NODE-037 completes bounded typed drag-data storage in the optional leaf:
**14** new tests (**27** total) and real captured-text delivery pass. Next add
capture subscription/epoch ownership, then compose the Page-scoped dispatcher
and host teardown. No addon rebuild or public gate change.
[Data boundary and remaining work](knowledge/pwright-chromium-drag.md#node-037-admitted-intercepted-data).

NODE-036 implements the Rust-owned browser drag-listener stage: **13** unit
tests, **6** native Chrome cases and an independent **3-package** leaf. Next
compose Frame snapshots, admitted intercepted data, Page drag state and host
teardown. This helper is not the complete dispatcher and does not change public
Mouse **36/41**. [Precise remaining requirements](knowledge/pwright-chromium-drag.md#next-implementation-requirements).

NODE-035 completes optional dispatcher selection in the native factory/Page
route, including standalone buttons. **13** new deterministic tests and fresh
default/passthrough **22/22** browser lanes pass. Still implement a budgeted real
Chromium drag provider, Page-scoped sharing across host calls and persistent
provider teardown; a provider created per command would not preserve drag state.
The addon is not rebuilt and public Mouse remains **36/41** on the prior artifact.
[Current selection boundary](knowledge/pwright-raw-mouse.md#page-scoped-dispatcher-selection-node-035).

NODE-034 implements an optional Rust Mouse dispatch ownership seam: separate
result/cleanup polling, live Page-state reader with retained admission, and
forClick tagging. Ten new deterministic tests pass (input **81** total).
Typed native factory/Page/host selection and a budgeted Chromium drag provider
are still pending. No addon rebuild or improved public **36/41** claim belongs
to this checkpoint. [Scope and evidence](knowledge/pwright-raw-mouse.md#optional-mouse-dispatch-ownership-node-034).

NODE-033: the expanded 41-case Mouse gate reproduces start-order races and the
missing drag path independently. A 32-slot Rust first-poll boundary shared by
raw Mouse/Keyboard improves **33/41 to 36/41**; seven new host ownership/order
tests pass (**115 / 6 ignored** total). Keep the five left-held-move failures as
required cases. Next implement an optional budgeted drag delegate (including
forClick bypass), then verify broader continuation/runtime-queue ordering and
first-error fidelity; do not substitute whole-action serialization or fake delay.
[Evidence and limits](knowledge/pwright-raw-mouse.md#same-turn-input-ordering-node-033).

Final NODE-033 artifact gate: Mouse **36/41**, Worker/profile **3/3**, Keyboard
**24/24**, typing **30/38**, actions **24/27**, screenshot **12/12**, lifecycle
**45/45**. Preserve all 11 typing/action attachment failures (5 candidate topology-
init, 6 oracle connect) as NODE-013 evidence. They precede the tested input actions;
no cause or repair has been established. Reload/shared still need final-artifact
reruns after their independent open issues are addressed.

NODE-032 contract layer is implemented: six generated Page commands,
capability/client/codec/server packages, **24** release tests, strict Clippy and
**3/6/7/10** dependency proofs. The native factory now passes **13** tests and
**22/22** typed-native browser cases with an **8-package** closure. Bounded Page
route storage and caller-polled retirement now pass **18** tests and **22/22**
full-wire browser cases, with a **16-package** closure. The optional controlled
invocation now adds **15** tests and **22/22** runtime browser cases (**17** packages).
Host-selected bounded retirement workers and all six typed HostSession methods
are now installed: **9** new tests, **22/22** host browser matches and real
close-during-delay validation pass. N-API/JS wiring now passes **26/26** public
comparisons and **3/3** profile/owned-and-borrowed Worker tests on a rebuilt addon.
Next finish the nested-runtime storage audit, exact error fidelity, wider
concurrency and full artifact-specific regressions.

NODE-032 now includes Rust click/dblclick scheduling and separate result/native
retirement boundaries. **22/22** pinned Chrome event/state cases match; input
**71**, model **6**, fake **23** tests and strict release Clippy pass. Raw Mouse
captures Page state before awaiting earlier replies, with no hidden task or JS
automation. Next finish concurrent error ordering, HTML drag composition and
artifact-specific verification. Public `page.mouse` is now exposed.
[Design, bounded storage and remaining gates](knowledge/pwright-raw-mouse.md).

NODE-030 minimal native reproducer is now implemented and exercised twice:
the raw CDP response-boundary close acknowledges but target presence persists
at 200/1000 ms, with no host/navigation/runtime/FFI or reload cancellation loaded.
CDP **73 + 11 + 3** tests and strict release Clippy pass. Keep the native issue
and original **16/18** public gate open, preserve absence/rollback semantics,
and continue remaining Node API integration independently.
[Raw isolation](knowledge/pwright-page-reload.md#minimal-raw-cdp-isolation).
The Mouse addon now completes the unchanged 57-case shared corpus: **53 match /
2 unsupported / 1 oracle-error / 1 setup-error**. Upload/download remain
unsupported; mouse-backed touch now matches without proving real Touchscreen.
The goBack case hits a candidate 5 s topology-init timeout before operations;
retain NODE-013 and do not turn setup failure into an oracle-error or skip.
Same-artifact Keyboard/typing/actions/screenshot/lifecycle pass **24/38/27/12/45**;
reload stays **16/18**. [Evidence](knowledge/pwright-raw-mouse.md#same-artifact-regression).

NODE-030 diagnostic checkpoint: bounded native controls reproduce an acknowledged
close with the target still present after reload timeout **without host Page
close**. Loader/request correlation separates the new document from old lifecycle
replay. Stop-loading is timing-sensitive and not adopted as a production fix.
Next isolate the native race and design close/cancellation settlement without
weakening absence proof or adding a mandatory navigation dependency. Host **99**
offline tests pass; the existing **16/18** public reload gate remains open.
[Causal evidence](knowledge/pwright-page-reload.md#causal-native-close-diagnostic).

Latest full regression on Keyboard addon `1319fa02...ded25dc1`: typing **38/38**,
actions **27/27**, screenshots **12/12**, lifecycle **45/45**, reload **16/18**.
Every held-modifier typing catalog entry now runs through the public API.
The two original short-response Page-close failures remain, and intermittent
attachment/window causes remain open. [Evidence](knowledge/pwright-raw-keyboard.md#same-artifact-post-keyboard-regression).

NODE-031 raw Keyboard primitives now pass **42** input tests, **103** Locator
regressions and **14/14** real-Chrome oracle cases, including empty-key timing.
Keyboard 1.0 capability/client/codec/server now pass **3/2/13/1** tests and exact
**3/6/7/10** package closure checks. The optional native backend passes **18**
tests and **14/14** typed real-Chrome comparisons with an **8**-package closure.
The optional Page wire composition now passes **11** tests and **14/14** wire-to-Chrome
comparisons, with a **16**-package closure. Independent aggregate Page-input
selection and retained-state admission are now implemented; input **46**, aggregate
**294**, host **89** tests pass plus a real-host attach/close resource gate.
An opt-in controlled runtime adapter now passes **18** composition tests, including
7 exact-route/cleanup/deadline tests; default/runtime closures are **16/17**.
The regenerated wire path passes a fresh **14/14** Chrome comparison.
Rust host/N-API and all five public `page.keyboard` methods now pass **24/24**
paired Chrome cases. Host **95**, root **295** tests pass; host closure is **229**.
Shared coverage advances to **52 match / 3 unsupported / 2 oracle-error**.
[Contract and integration tasks](knowledge/pwright-raw-keyboard.md).

NODE-030 typed navigation 1.1, codec/client/server, composition, host/addon and
public Node reload are implemented. Backend/client/codec tests pass **14/6/22**,
composition **287**, host **89**; navigation and host dependency proofs stay
**15/224** packages. Shared verification is **48/7/2**, no other errors. Public
reload is **16/18**: arrival-boundary Page close fails, but the added post-commit
close case passes. Full lifecycle is **44/45** on NODE-014 window geometry, and
screenshots are **0/12** at attachment. Next: native timing at the arrival boundary,
then Browser/Worker ownership, cache/redirect and broader option tests.
[Contract and implementation gates](knowledge/pwright-page-reload.md).

NODE-029 sequential typing now has frozen 1.22/2.5 baselines, generated 1.23/2.6
routes, strict focus/input composition, client/server/backend dispatch and public
host/FFI aliases. The initial Node gate passes 27/27 and shared verification is
46 match / 9 unsupported / 2 oracle-error. UTF-16 rejection is explicit, not full
preservation. The newer opt-in interleaved policy passes **30/30 typing + 27/27
action** tests, correcting the prior overlap and NODE-022 sample while keeping
default Rust exclusivity. It also fixes literal versus physical-code Shift
resolution. Mid-input target replacement, native enqueue/ack cancellation,
cross-Frame state and broader timeout-state parity remain open.
[Contract and evidence](knowledge/pwright-sequential-typing.md).

Lifecycle supervision checkpoint: the same native artifact now passes **45/45
in 192.715 seconds**. Bounded phase tracing showed the old 180-second aggregate
limit truncating the final connect before its API deadline. A 600-second suite
bound plus independent 60-second per-case watchdog replaces that aggregate-only
policy; no API deadline or assertion changes. **29/29** Node unit tests, types
and the 224-package boundary pass. Keep the earlier 18-pass/3-fail incomplete
run and screenshot attachment failure/recheck pair. NODE-013/014 remain open:
the next diagnostic needs causal attachment/window evidence, not a last-TAP
guess at shutdown deadlock. Manual orphan cleanup is not automated teardown.
[Evidence and scope](node-ffi-compatibility-design.md#node-013-lifecycle-supervision-evidence-2026-09-07).

Latest NODE-013/VS5 checkpoint: the Node recipe explicitly joins recursive
auto-attach with its three same-session prerequisites, keeping Context/world/
resume/publication fences. No cross-Page scheduler is added. Current artifact
passes screenshot 12/12 (both Worker cases), lifecycle 45/45 and the independent
serial/selected OOPIF cases 2/2. Shared verification completes 57 cases with
45 match / 10 unsupported / 2 oracle-error and no value/candidate/setup/cleanup
errors; it remains non-passing. Composition tests pass 287, host/probe tests 98,
and the 224-package boundary is unchanged. Earlier red artifacts and the negative
cross-session ABBA result remain evidence; NODE-013 is not declared resolved.
[Current contract and results](node-ffi-compatibility-design.md#node-013-recursive-attachment-prerequisite-join-2026-09-07).

VS5 screenshot increment: public Node `page.screenshot()` now invokes the Rust
content command with finite per-Page queueing and admitted byte handoff. The
10-case decoded-image/ownership lane passes on the handoff artifact after fixing
edge clipping and queued-result lease overlap; final-artifact verification is
8/10 on attachment timeouts and shared verification has 21 setup errors on that
preceding artifact; the later 3/12 trace-only and current 12/12 recursive-join
results are recorded separately above.
Rendering transactions, default font/caret behavior,
combined fullPage/clip and filesystem output remain design/implementation gates;
this does not complete VS5. [Current evidence](node-ffi-compatibility-design.md#node-page-screenshot-binding-and-ownership-checkpoint-2026-09-07).

Attachment investigation: a bounded, owned-page reconnect probe now distinguishes
named utility-world retention from connection success and memory measurements.
Stable-topology Pwright reconnects reuse one name; oracle reconnects add worlds.
Reload creates a new main world without reducing the observed utility counts.
Next design questions are Page-ID churn and overlapping-session latency, not an
assumed memory fix. NODE-013 remains open. See [evidence](node-ffi-compatibility-design.md#node-013-retained-world-diagnostic-2026-09-07).

Latest Node checkpoint: NODE-027's reproduced Worker cleanup failure is fixed
with host-selected, acknowledged owned-session retirement. Final full Locator
evaluation verification passes 23/23 with a clean supervisor, including borrowed
Page/Promise survival. Admission sealing, duplicate receipts, partial failure,
epoch mismatch and release-ack races are covered in Rust. NODE-013 attachment
timeouts and NODE-014 window dimensions remain open; the latest lifecycle
attempt hit its unchanged total deadline. See [current evidence](node-ffi-compatibility-design.md#node-027-host-integration-and-worker-verification-2026-09-07).

Historical preceding Node checkpoint: public Locator.evaluate composes the existing
host-owned Rust command with a bounded Node-thread conversion rendezvous.
The original getByLabel shared observation is wired through this public API.
Its expanded 22-case lane passes assertions but remains red on native running-
Worker cleanup (NODE-027). Next design must prove cancellation/release of a
never-resolving evaluation, including late-result ownership and borrowed Context
protection; do not weaken the supervisor or extend the timeout. This is progress,
not completion of VS3 or compatibility. See [the checkpoint](node-ffi-compatibility-design.md#public-locatorevaluate-ffi-checkpoint-2026-09-07).

NODE-027 boundary follow-up: reject early group release plus dropped result as
a general cleanup proof (a real Chrome test observes a late exception wrapper).
Design an exact owned-session retirement receipt for teardown, including epoch,
admission, acknowledgement and pending-command ordering; do not promote generic
detach events to destruction proof. Two live boundary tests pass while the
production Worker cleanup remains unfixed. See [evidence](node-ffi-compatibility-design.md#node-027-native-cancellation-boundary-evidence-2026-09-07).

Historical independent-leaf checkpoint: the Rust receipt-driven path was implemented and
verified with native session detachment and borrowed-Page survival. Remaining
NODE-027 integration is the bounded host-owned session inventory, registration/
dispatch seal, epoch-matched acknowledgements and partial-failure handling before
cleanup drain. This is not yet selected by the Node host or a green Worker gate.
See [the implementation checkpoint](node-ffi-compatibility-design.md#node-027-receipt-driven-rust-evaluation-implementation-2026-09-07).

> Status: planning the design work. This document identifies what must be
> designed; it does not approve an architecture or implementation.

## Purpose

Before choosing crates, traits, schemas, or transports, we need an ordered set
of design questions. A design task is complete only when its alternatives,
decision, consequences, rejected options, and validation evidence are written
down. Producing code is not part of these tasks unless a deliberately approved
prototype is needed to answer one narrow question.

## Agreed workflow

Before approving Pwright v2 design decisions, we will study Playwright one
topic at a time. Each lesson must cover the user-visible capability, the
internal design model, relevant upstream source locations, important costs or
limitations, and questions raised for Pwright. Learning notes are evidence for
later design tasks; they are not architecture decisions by themselves.

## Implementation checkpoint

On 2026-09-01 the user authorized implementation to begin. The first bounded,
exploratory slice is recorded in `architecture-v2.md` and implemented as the
new `pwright-core` crate. It contains only:

- dependency-free typed `Service`, `Layer`, and `Stack` composition;
- capability/plugin metadata and protocol-version identifiers;
- preflight validation for duplicate capabilities, duplicate command owners,
  missing dependencies, incompatible versions, and dependency cycles.

The slice passed eight release-profile unit tests, release-profile Clippy with
warnings denied, and a normal dependency-tree check containing only
`pwright-core`. It does not connect to a browser and does not alter legacy v1
behavior.

This checkpoint starts implementation but does not mark DT-001 through DT-072
complete. Until the gates below are resolved, further code remains exploratory;
the next browser-connected slice still requires explicit protocol, lifecycle,
backend, resource-limit, and acceptance decisions.

The next foundation checkpoint adds codec-independent typed commands, results,
events, lifecycle controls, classified operation failures, a bounded object
ownership graph, a bounded pending-call table, and a bounded event queue. The
implemented behavior now proves:

- numeric call, object, cancellation, method, event, and object-kind identities
  cannot be accidentally interchanged by the Rust type system;
- command parameters, results, error details, and event payloads remain tied to
  their static specifications;
- children require a live parent, duplicate IDs fail, adoption cannot create a
  cycle, and parent disposal produces a deterministic child-first cleanup plan;
- live objects and pending calls have hard count limits;
- cancellation must match the call's registered cancellation identity and the
  first cancellation reason wins repeated races;
- event overflow is explicit through `RejectNewest` or `DropOldest`, including
  zero-capacity behavior and attribution counters;
- no container starts tasks, reads a clock, or grows its live item count beyond
  the configured limit.

The expanded `pwright-core` suite passes 29 release-profile tests, Clippy with
warnings denied, rustdoc with warnings denied, and still has no normal
dependencies. The complete existing workspace also passes its release-profile
test suite after adding the new workspace member; environment-dependent Docker
browser tests remain explicitly ignored as before. This remains a foundation
only: no codec, transport, async runtime, backend, browser process, or public
Page API is implemented yet.

The first backend-connected checkpoint adds three provisional crates:

- `pwright-cap-navigation` defines the portable typed `BeginNavigation`
  primitive and depends only on `pwright-core`;
- `pwright-runtime` dispatches typed commands in process without serialization,
  rejects stale target objects, reserves bounded pending-call slots, correlates
  results, and releases pending entries when a dispatch future is dropped;
- `pwright-backend-cdp` maps that primitive to the existing `pwright-cdp`
  `Page.navigate` implementation and is the only new crate in the slice that
  imports CDP and Serde JSON.

The four v2 crates pass 40 non-browser release tests, including one complete
fake path from `Command<BeginNavigation>` through the runtime and CDP adapter.
An additional explicitly ignored live test passed against the dedicated Chrome
CDP profile and confirmed real navigation plus exact cleanup through an owned
incognito BrowserContext.

The live run found that Chromium returned `success: true` for
`Target.closeTarget` while two default-context test targets remained visible.
The lower-level implementation now at least validates the returned `success`
field, but the v2 live test deliberately uses owned-context disposal and polls
`Target.getTargets` until the exact target disappears. This is evidence for
DT-032 and DT-050: command acknowledgement is not resource-termination proof,
and an owned BrowserContext provides a stronger cleanup boundary. The two test
targets from the failed attempts were removed with Pwright's verified HTTP
closer; unrelated browser targets were preserved.

This checkpoint still does not implement public `Page::goto`. Commit/load
waiting, main-resource Response identity, redirects, deadlines, cancellation,
and page/frame close races remain required before that compatibility claim.

The browser-provider ownership checkpoints add four provisional crates:

- `pwright-browser-provider` is standard-library-only and defines distinct
  attach and launch contracts. External attachments contain only a connection;
  launched browsers additionally contain an explicit, consuming process lease.
- `pwright-provider-cdp` accepts only explicit, validated `ws://` or `wss://`
  endpoints and returns an external attachment through an injectable connector.
  It contains no discovery, launch, install, download, cache, or termination
  path.
- `pwright-provider-chromium` is separately opt-in. It discovers an already
  installed browser or accepts an explicit executable, requires either a fresh
  empty profile or an explicitly marked persistent profile, forces an
  ephemeral loopback CDP endpoint, waits for the browser-owned
  `DevToolsActivePort`, and returns a CDP connection plus a consuming process
  lease. It contains no browser bytes, download, installer, cache, or Node.js
  driver.
- `pwright-browser-install` is a separate standard-library-only contract. An
  exact engine/revision/target/integrity request consumes an explicit one-use
  download permission and composes injected resolve, fetch, verify, and install
  stages. It contains no stage implementation and no provider depends on it.

The v2 foundation now passes 46 non-browser release tests. A second explicit
live test attached to the dedicated Chrome profile, called
`Browser.getVersion`, dropped the external attachment, reattached to the same
endpoint, and called `Browser.getVersion` again. This demonstrates that
transport disconnect does not terminate the externally owned browser. The
test introduced no browser context or target and issued no close command.

The explicit-launch checkpoint now covers system executable discovery,
explicit-path launch, process exit observation, startup cancellation/failure
cleanup, and consuming termination authority. Fake process/waiter/connector
tests prove exact provider-owned flags, loopback-only endpoint derivation,
connection-failure cleanup, already-exited observation, and that dropping a
successfully delivered lease does not implicitly kill the browser. The next
DT-050 checkpoint adds a composable shutdown coordinator: bounded CDP
`Browser.close`, independent bounded process observation, force-kill escalation,
bounded post-kill observation, and cancellation-safe cleanup. Process exit,
not command acknowledgement, is authoritative. Persistent profiles require an
opaque token initialized in an empty directory plus an exact versioned marker;
ordinary nonempty Chrome profiles remain invalid. The provider never deletes
caller-owned profile data. The final DT-050 checkpoint adds the independent
`pwright-browser-install` contract. An installation root must be canonical and
empty both when selected and immediately before resolution. Verification is a
typed mandatory stage before installation; the installed receipt has neither
launch nor termination authority. Compile-fail tests require explicit download
permission and reject process control on a receipt. Dependency inversion proves
that existing attach and launch providers cannot reach this crate. This
completes DT-050's provider, ownership, profile, shutdown, and optional managed
installation boundaries; concrete download/catalog/archive implementations and
failure rollback remain later operational/plugin work.

The DT-051 backend checkpoint adds `pwright-browser-backend`, which depends only
on core. A backend descriptor names its engine and native protocol, then maps
each supported portable command to its owning capability, minimum version, and
native or emulated implementation classification. `BackendManifest` validates
all mappings against the installed `Registry`; `BackendCheckedService` binds a
typed `CapabilityCommandSpec` at setup and rejects unsupported or mismatched
features before its inner adapter can run. `ChromiumCdpBackend` declares direct
begin-navigation as native and the composed goto/click/fill behaviors as
emulated. CDP access requires the explicit
`NativeBackendAccess<ChromiumCdpBackend>` trait on adapter-owned services.
Portable capability/client/model crates neither depend on `pwright-cdp` nor
name its identities or messages. This completes DT-051's responsibility and
escape-hatch boundary; actual Firefox/WebKit/BiDi adapters remain independent
future implementations rather than changes to portable command contracts.

The DT-052 checkpoint adds the bounded `CdpTopology` recovery state machine to
the concrete Chromium adapter. The composition root supplies public object IDs;
the adapter maps CDP contexts, targets, sessions, frames, and selected execution
worlds to BrowserContext/Page/Frame lifecycle effects without exporting native
identities into portable crates. A page target may race ahead of its context,
and iframe children may race ahead of their parent; both paths defer within
explicit limits and are discarded cleanly if their target detaches first.
Popup notification waits for initialized opener and popup pages while ownership
remains BrowserContext→Page. Main-frame IDs survive Chromium process swaps, and
local iframe plus OOPIF events share one Frame identity. Ambiguous OOPIF detach
requires a synchronization confirmation before disposal. Crashes mark the
exact page, session detach and context close clean exact subtrees, and
connection epochs prevent stale pre-reconnect events from deleting rebound
objects. Only main and Pwright utility execution worlds are retained, keyed by
session plus execution-context ID. Generated CDP event normalizers, runtime
`Complete`-topology integration, and AAA race tests make these rules executable.
The state machine chooses no event pump, executor, transport, provider, browser
binary, or hidden object-ID allocator.

The DT-060 shared-budget checkpoint adds a deny-by-default, connection-local
`ResourceLedger` to core. Fourteen count/byte classes cover objects, calls,
commands, events, wire frames, response bodies, logs, traces, screenshots, DOM
content, selectors, handles, and backend state. Global and per-scope limits are checked
atomically; multi-class failure rolls back completely. Scope and lease metadata
have independent bounds, stream leases resize without release races, and
non-cloneable RAII leases release when cancellation drops a future. The first
real integration makes budgeted WireFrames consume aggregate `WireBytes`;
WireFrame clones share immutable storage and one lease. The allocation table,
eviction matrix, metrics schema, browser/runtime measurement separation, and
remaining enforcement work are canonicalized in
[resource-boundaries-v2.md](resource-boundaries-v2.md). This is a DT-060
foundation checkpoint, not completion of the memory investigation or every
future producer integration. The next wire checkpoint covers the inbound Tokio
path: a declared envelope is reserved before body allocation, decoded bytes are
reserved before copying, and the two leases overlap during the real two-buffer
peak. Route attribution survives in the returned frame, while cancellation and
malformed-input paths release staging capacity automatically. Outbound encoded
and length-prefixed buffers are also reserved before allocation; a cancelled
partial write keeps its lease in transport-owned state until it is completed.
Typed event handlers now optionally reserve `EventItem` and decoded
`EventBytes` together in the source-object scope. Reliable overflow consumes no
new budget; lossy overflow uses an atomic lease replacement so failure leaves
the previously queued event intact. Pop, clear, handler teardown, and protocol
extraction release all queue-owned capacity.
Runtime composition can now install that ledger only before state exists.
Create reserves object-scoped `LiveObject` capacity before graph mutation;
Adopt preserves the lease, while child-first Dispose and transport close
release exact subtrees. Runtime call admission reserves call-scoped
`PendingCall` capacity before queue/backend work and retains it through
cancellation or abandonment until terminal guard cleanup.
Connection admission now adds `ActiveCommand` only after a target head receives
an aggregate backend-execution slot. Connection waiters remain bounded
`PendingCall`s without double-counting active work. Immediate grants reserve
before scheduler mutation; FIFO promotion atomically moves the existing active
lease to the next call scope, and completion, cancellation, or future drop
releases it. AAA tests cover saturation, waiter exclusion, unpolled acquisition
drop, promotion, and injected promotion failure.
The encoded client can now install that ledger through direct, control-enabled,
or negotiated channel factories. It reserves a call-scoped `PendingCall` before
correlation or send mutation. Normal results release at correlation; a dropped
possibly sent future keeps the lease with its tombstone through cancellation
send until a terminal late result or connection close. Send failure, explicit
close, EOF, receive failure, and driver drop drain the same resource map. AAA
tests prove rejection atomicity and every implemented terminal boundary.

The first production-composition gate now separates declared requirements from
configured capacity. Each component can publish a zero-dependency
`ResourceRequirements`; merged plans take the strongest requirement in every
dimension, while all installed components still contend dynamically in one
connection-local ledger. `ProductionResourceLedger` rejects a plan that
exceeds configured class/scope/lease limits, rejects attachment to any ledger
with current or historical use, and rejects a component claim omitted from the
declared plan. Runtime limits now derive their exact `LiveObject`,
`PendingCall`, realizable `ActiveCommand`, lifecycle `EventItem`, scope, and
lease requirements. Both dispatchers expose a production constructor that
must claim those requirements before state exists.

The runtime lifecycle outbox now keeps an `EventItem` lease beside every
retained control message. Create and Adopt reserve before object-graph
mutation; a child-first Dispose reserves its whole ordered message batch before
disposing any node. A rolled-back claim keeps the lease, while pop, commit,
clear, or transport close releases the exact entry. This makes a saturated
control channel a typed, atomic rejection rather than a partially applied
lifecycle transition.

The explicit-close prerequisite adds a controlled-runtime two-phase disposal
transaction. Preparation validates the live subtree, reserves the complete
child-first control batch and all `EventItem` leases, and then leaves objects
and calls live while caller-owned asynchronous backend cleanup runs. Only one
lifecycle transaction may be active, so Create, Adopt, Dispose, and a competing
preparation cannot invalidate that plan. Dropping the preparation rolls every
reservation back. Commit revalidates the exact subtree, invalidates it
child-first, transfers the reserved messages to the outbox, and wakes admitted
calls with `ParentClosed`; transport close remains authoritative, drains state,
and invalidates the outstanding token. This closes the runtime half of the
backend-before-local-close ordering. A later composition transaction must also
serialize the Chromium topology and native-context route before a public
`BrowserContext.close` capability can claim end-to-end atomicity.

The next context-lifecycle checkpoint now supplies that first end-to-end
server-side transaction. The generated, independently selectable
`pwright.context_lifecycle` 1.0 surface owns only method `0x0003_0001`,
`browser-context.close`, targeted at BrowserContext. Separate contract, binary
codec, client-extension, and generated server-handler crates keep it absent
unless selected. The codec rejects attachments and rejects timeout or
cancellation metadata; the client exposes a no-argument close and treats an
already-disposed or racing `TargetClosed` context as success.

The Chromium backend publishes a bounded portable-context/native-context route
and closes only `Owned` contexts with `Target.disposeBrowserContext`. Default
and externally attached contexts fail before CDP I/O. Production topology now
has its own owned rollback-on-drop candidate and transaction gate, so event
mutations cannot replace its route while CDP is pending. The composed close
service prepares topology and runtime, awaits CDP, commits topology first,
commits deterministic Frame→Page→BrowserContext runtime disposal, and only
then returns success. Injected CDP failure proves both local owners and the
control outbox roll back. A dedicated Chrome 152 live proof removed the exact
temporary context/target while `Browser.getVersion` proved the externally
owned browser remained alive.

The remote completion checkpoint now closes the result/lifecycle race. The
ordinary runtime-control outbound and close service share one bounded priority
writer. Its capacity is the declared supervisor in-flight command limit plus
the singleton outbound lane. Close acquires the barrier before CDP, and after
successful local commit it drains every preceding message through the target
BrowserContext Dispose before the typed handler may write Result. A complete
memory-transport proof feeds those exact frames to the real encoded-client
driver: Frame, Page, and BrowserContext are invalidated child-first while the
close future remains pending, and only the following Result wakes it. Separate
AAA tests prove barrier priority, exclusion, drop release, and hard-capacity
rejection. A remote profile that advertises context lifecycle without ordered
control is rejected before connection work.

The attachment checkpoint also removes the hand-wiring gap.
`AttachedChromiumComposition` owns `ProductionCdpTopology` behind the exact
shared mutex consumed by the close service. Its factory binds that topology to
the attached CDP browser session and a controlled-runtime lifecycle handle,
and the attachment admits only one production runtime owner. The live Chrome
test now constructs the declared production plan, uses `attach`, mutates the
shared topology, obtains the factory service, closes the owned context, and
proves the external browser remains alive.

The cancellation-safe continuation checkpoint closes the remaining explicit-
close ownership gap. The attached factory requires a caller-supplied
`ReservedCleanupExecutor`; it reserves a bounded slot before any browser work.
Capacity failure is a typed pre-CDP rejection. An unpolled future remains lazy
and releases the slot when dropped, while a started pending future transfers
the exact remaining close transaction to that reservation. The caller can
poll the queued job and inspect completion/failure metrics; the composition
does not choose an executor or start a hidden task. AAA tests prove transfer,
laziness, and capacity rejection, and the dedicated-Chrome proof exercises the
continuable attached factory.

The Page-lifecycle checkpoint follows Playwright's two-contract design rather
than hiding it behind one boolean. Generated optional capability
`pwright.page_lifecycle` owns Page-only `page.close` and
`page.run_before_unload`, with separate codec, client-extension, and generated
server crates. Ordinary Chromium close uses browser-session
`Target.closeTarget`, bounded exact-target absence confirmation, paired
topology/runtime preparations, synchronized Page-route retirement, and the
same reserved continuation plus remote lifecycle-before-Result barrier.
Before-unload uses the exact Page session's `Page.close`, returns after
initiation, and leaves Page/Frame identity live. Tests cover method selection,
confirmation failure rollback, unpolled and started-future Drop, composition
advertisement, Frame→Page→Result remote application, and client idempotence. A
dedicated Chrome 152 proof closed only the selected Page while preserving the
parent context and external browser; the context live proof now also proves
descendant route-table retirement.

The following Browser-lifecycle audit records why the next command cannot be a
generic CDP adapter method. Playwright's launched Browser close consumes
process shutdown authority, while `connectOverCDP` close only closes its
synthetic transport and a remote prelaunched dispatcher ignores stop/kill.
Pwright will preserve that public outcome but keep external disconnect in
connection control and permit owned close only in a launched composition that
holds the unique `BrowserProcessLease` plus explicit remote authorization.
The separately opt-in launched composition now exists and owns the first half
of that boundary: a plan/ledger-proven CDP connection plus the unique process
lease and bounded local shutdown. The portable `pwright.browser_lifecycle`
command is now implemented as four independent
capability/codec/client/server crates. Only the launch leaf can add it to its
handshake manifest and convert an owner into a non-cloneable grant after the
accepted server session negotiates it and proves it shares the launch's exact
production connection ledger; a denied conversion returns the untouched owner.
Its caller-reserved continuation, terminal whole-subtree disposal, and
ordered Page/Context/Browser Dispose-before-Result path are covered by unit,
encoded-memory, compile-fail, and real-Chrome tests. The optional close reason
is now codec-bounded and propagated as one shared terminal diagnostic to every
already-pending descendant operation while backend cleanup remains
structurally `ParentClosed`. Public Browser disconnected-event ordering is now
implemented. Client-side reason persistence is also bounded and root-shared:
the first-polled close installs one `Arc<str>` before dispatch, descendants
reuse it, and operations first attempted after disposal report it without
retaining the connection. Source anchors and the remaining contract are
recorded in `knowledge/pwright-browser-close.md`.

The first disconnected-event foundation is now independently packaged as
capability, fixed-size codec, client helper, and queue-free one-shot server
publisher crates. A split runtime-control fence can send all descendant
Dispose messages while leaving Browser Dispose queued, establishing the only
valid insertion point for `browser.disconnected`. The publisher treats a
failed or dropped polled send as terminal rather than retrying a possibly
partial frame. The attach profile now advertises this capability only with a
matching handshake, ordered control lane, and sufficient fixed-size frame
limits; its direct publisher is built from the accepted protocol and shared
production ledger without an event queue. One composition factory shares that
same at-most-once publisher among the native-CDP liveness lane, connection
termination cleanup, and the optional owned-close wrapper. Encoded tests prove
descendant Dispose -> disconnected -> Browser Dispose -> Result, while native
loss proves the same order before ordinary connection cleanup.
The ignored-by-default live launch proof now negotiates lifecycle, Browser
events, and control against a separately spawned Chrome 152, verifies the full
five-frame terminal order, then polls native liveness and confirms that cleanup
coalesces the same close without a duplicate event or surviving process.

The BrowserContext-creation checkpoint implements the empty-options subset of
`Browser.newContext`, not the full Playwright options surface. Its design makes
creation depend on the already-implemented context lifecycle capability and
identifies two reusable prerequisites: rollback-on-drop prepared runtime object
creation, and bounded client identity that distinguishes a wire-announced
object from the one later claimed by a typed result. Native creation uses
`disposeOnDetach`; post-CDP publication failure must compensate by disposing
the exact new context. The four-owner transaction, non-reusing object-ID rule,
remote Create-before-Result fence, and acceptance gates are recorded in
`knowledge/pwright-context-creation.md`.

The Page-creation checkpoint extends that inverse lifecycle design to the first
related-object transaction. Optional capability `pwright.page_creation` owns
only BrowserContext `new_page` and depends on Page lifecycle. Its portable
result initializes a Page and its required main Frame together; CDP target,
session, context, and frame strings remain backend-private. Chromium creates
`about:blank` in the exact native context, uses a flattened target session, and
accepts only a bounded parentless `Page.getFrameTree` root as initialization
proof.

Runtime `PreparedObjectCreationBatch` validates and reserves Page→Frame graph,
`LiveObject`, and ordered lifecycle capacity without publishing either object.
Composition prepares the matching topology and Page route while their readers
are excluded, then commits runtime Page→Frame ownership before infallibly
publishing both prepared local views. Any post-target failure closes the exact
target and confirms bounded absence. A started operation abandoned by its
caller transfers to a pre-reserved, caller-owned continuation; exhausted
capacity rejects before ID allocation or CDP.

Remote completion physically orders Page Create, Frame Create, then Result.
The client claims both prior announcements as one state transition, so a
missing, mismatched, disposed, duplicate, or over-capacity second initializer
cannot expose a half-created Page. The dedicated Chrome 152 proof exercises the
production attach/runtime/factory path, verifies `about:blank`, main-Frame and
Page-route publication, closes Page and Context through their typed lifecycle
services, and confirms the externally owned browser survives. The detailed
contract and evidence are recorded in `knowledge/pwright-page-creation.md`.

The value-only evaluation checkpoint now implements the approved boundary.
Playwright's
Page convenience delegates to its main Frame, while by-value evaluation and
live JS/Element Handles remain different protocol operations. Pwright
therefore makes `pwright.evaluation` independently installable and leaves Handle
identity, disposal, and retention to a later dependent capability. Its tagged
value graph, Chromium main/local/OOPIF execution-world routing, bounded browser-side serializer,
typed JavaScript failures, production resource peak, explicit non-goals, and
acceptance tests and implementation evidence are recorded in
`knowledge/pwright-value-evaluation.md`. The separate capability/model,
generated schema/reference/server surface, bounded binary codec, Page/Frame
client extension, Chromium backend, production factory, exact session plus
`uniqueContextId` route, and composition selector are all present. Frame route
state is transactionally replaced with topology and is resource-declared only
when evaluation is selected. Fake, remote-wire, and dedicated Chrome
152 tests prove that no Handle object or lifecycle message appears; the live
production route also reaches a real cross-site OOPIF through its own session
and globally unique execution context. This
advances DT-020, DT-021, DT-031, DT-033, DT-040, DT-043,
DT-051, DT-052, DT-060, and DT-070 without treating Handle semantics as solved.

The explicit Handle-lifecycle checkpoint now establishes the first dependent
foundation without changing value-only evaluation. `pwright-browser-model`
adds stable `JsHandle` and `ElementHandle` kinds under `Frame`, while the
independently installable `pwright.handle_lifecycle` capability owns only
`handle.dispose`. Its generated schema, bounded binary codec, generated server
handler, and opt-in client extension are separate crates. Rust `Drop` still
sends no I/O; the explicit async request has no action timeout/cancellation
metadata and treats a locally stale handle or a racing `TargetClosed` response
as idempotent success. The Chromium adapter binds a bounded native object ID to
its exact CDP session and sends exactly one `Runtime.releaseObject` with no
hidden retry. Child-first client invalidation proves that disposing a Frame
family makes both Handle types locally stale. See
`knowledge/pwright-handle-lifecycle.md`.

The next Handle-evaluation checkpoint implements a bounded creation and
explicit-disposal foundation. The independently installable
`pwright.handle_evaluation` capability depends on Handle lifecycle and owns
`evaluation.evaluate_handle` without changing value-only evaluation. Its
schema, binary codec, generated server binding, and Page/Frame client extension
produce a portable initializer with concrete JSHandle or ElementHandle kind;
no CDP object ID crosses the wire. Remote client materialization requires the
exact prior Create announcement.

The Chromium leaf evaluates with `returnByValue: false` in the route's exact
session and `uniqueContextId`. Composition commits runtime Handle ownership and
a bounded native route as one synchronous publication step, compensates a
failed publication with native release, and disposes by releasing the same
native object before committing runtime removal. A dedicated Chrome 152 proof
creates and releases a DOM ElementHandle, proves primitive JSHandle release is
a no-op, and leaves the external browser alive. This advances DT-022, DT-031,
DT-040, DT-043, DT-051, DT-052, DT-060, and DT-070.

The additive 1.1 checkpoint now implements Handle-valued arguments without
adding Handle knowledge to `pwright.evaluation`. A separate recursive graph
supports nested portable Handle leaves, retains the original depth/node/entry/
string/binary bounds, and deduplicates native arguments in first-appearance
order. The 1.0 schema is frozen as its compatibility baseline; a connection
that negotiates 1.0 can still use value-only `evaluateHandle` but its codec
rejects Handle leaves. The Chromium service resolves all referenced IDs under
one route lock, atomically prepares output lifecycle ownership to exclude
concurrent disposal, and requires the same Frame, Page, CDP client, and current
world before native I/O or publication. Missing/disposed arguments are
`TargetClosed`; cross-Frame arguments are protocol/value failures. The backend
uses exact-context `Runtime.callFunctionOn` and never serializes native IDs into
source text. Chrome 152 proves an ElementHandle can enter a second call and
produce `SECTION`. Generic JSHandle context semantics are implemented. The
cross-world ElementHandle decision now fixes Playwright's Element-only rule,
Chromium describe/resolve mapping, temporary ownership, and the prerequisites
that must close before public composition. Its low-level CDP primitive is now
implemented with pre-I/O JSHandle/cross-session rejection, malformed-result
compensation, and a dedicated Chrome 152 main-to-isolated-world proof. Numeric
plus stable Frame-world routes and persistent Handle source-world identity are
now implemented with same-session replacement rejection before I/O. An opt-in
production adoption batch now prevalidates and pre-reserves the complete
temporary-object peak, owns exact `RemoteHandle` and backend-byte leases, and
exhaustively compensates partial failure. Its composition wrapper reserves
caller-owned cleanup capacity before adoption and transfers dropped adoption,
result-owner, and release futures to that continuation. Eight new AAA tests and
a second Chrome 152 main-to-isolated-world proof return every resource and
reservation to zero. The default command installation remains exact-world; a
separate `install_with_element_adoption` path now declares the additional peak
and composes pre-I/O argument classification, temporary substitution, Runtime
execution, topology revalidation, and exhaustive release. Three command-level
AAA tests cover child-Frame success, JSHandle rejection, and an OOPIF-shaped
different-session rejection. A third Chrome 152 adoption proof passes a real
child-Frame ElementHandle into its parent through the public service and
returns every owner to baseline. A fourth Chrome 152 proof creates a real
auto-attached OOPIF ElementHandle, rejects it before temporary-resource
admission, and preserves external Chrome. Chrome's OOPIF frame-tree root parent
shape is now accepted only for non-Page sessions, with positive and negative
AAA coverage. Aggregate migration now uses the tested live Frame/world resolver
for main, local-child, and OOPIF routes and the lifecycle leaf's two-phase
connection, parent, and topology retirement owners. The complete recipe has
moved its Handle route registry and all six Handle 1.2 commands to the
independent leaves; the older aggregate route/command types remain available
only for source compatibility. Cross-world adoption is now an explicit
aggregate backend option that contributes its own temporary-object resource
peak without adding another negotiated capability. The
Handle-local design is
now locked as the additive `pwright.handle_evaluation` 1.2 surface: five
commands cover `evaluate`, `evaluateHandle`, `jsonValue`, `getProperty`, and
`getProperties` on both Handle kinds, with atomic property-batch publication,
bounded browser-side enumeration, and inherited Frame/world ownership. See
`knowledge/pwright-handle-operations.md`. The 1.2 schema, generated server,
binary codec, typed client, bounded CDP backend, optional composition, ordered
Create-before-Result fences, cancellation cleanup, and real-Chrome duplex proof
are now implemented. Cross-world ElementHandle adoption remains an independent
compatibility decision.

The independent `pwright-composition-chromium-handle-evaluation` leaf now
implements the six-command 1.2 direct server composition over the smaller
lifecycle leaf. It adds bounded Frame/world routes, pre-CDP lifecycle
reservations, atomic single/batch runtime-plus-route publication, native
compensation, and caller-owned abandoned-result cleanup. Nineteen direct tests and a
dedicated Chrome 152 test pass, while a 24-package exact allowlist proves the
normal closure omits aggregates, topology, Context/Page commands, clients,
providers, launchers, installers, Tokio, Node.js, and browser bytes. The
backend-neutral `pwright-plugin-ordered-handle-delivery` now composes the
remote boundary from the leaf's exposed protocol/raw services: bounded
pre-admission, ordered single/batch Create-before-Result, physical Result-send
ownership, abandonment cleanup, and ordered Dispose. Six plugin AAA tests and
a 15-package exact allowlist cover that opt-in policy without adding it to the
direct leaf. The dedicated Chrome 152 proof now exercises the combined leaf and
plugin, including physical Create-before-Result and Dispose delivery.

The Handle route
table now has declared `RemoteHandle` and exact retained-byte ledger ownership,
current/peak and release-reason metrics, pre-CDP capacity rejection, and
prepared descendant retirement integrated with explicit Page and owned-Context
close. Route ownership by Frame/Page also folds main-world replacement, Frame
detach, Page crash, and reconnect into the topology/runtime lifecycle batch.
An opt-in Handle-aware runtime target retires all remaining routes and leases
when the Pwright connection closes. The complete application recipe now makes
that wrapper its concrete runtime type and delegates the common cancellation
and ordered-control traits through it. A caller-reserved continuation rejects
before CDP when saturated and completes plus disposes a Handle created after
its direct command future is abandoned.

The production root now makes lifecycle and evaluation explicit optional
profiles. Evaluation requires lifecycle and remote Handle selection requires
ordered control, but value evaluation remains absent unless separately
selected. Lifecycle owns the shared Handle-route budget, evaluation owns its
own CDP route budget, and preparation rejects mismatched limits, scope reuse,
or runtime capacity below topology plus maximum Handles. Handshake selection
then installs only the accepted codecs, backend routes, and generated handlers.
Its default remote-handler factory delegates creation, physical Result delivery,
and Dispose ordering to `pwright-plugin-ordered-handle-delivery`; the aggregate
keeps its older public decorators only for source compatibility, not as a
second active delivery policy. Dropping while Result send is pending performs
one ordered Dispose, and failed Create publication compensates the native
object. The all-capability profile and static handler chain now include both
Handle plugins; its runtime installer requires the lifecycle-leaf route table,
uses the evaluation leaf for both Frame and Handle-local commands, and makes
connection-close cleanup unskippable. Chrome 152 verifies the expanded
13-protocol handshake and that
external browser ownership is preserved. A second duplex Chrome 152 proof now
negotiates Handle evaluation 1.1, drives lifecycle Create messages into the
typed client, passes the returned ElementHandle through a second encoded
command, and completes graceful close without taking browser ownership. The
codec preserves the channel-generated cancellation ID while still rejecting
unsupported timeout metadata. See `knowledge/pwright-handle-evaluation.md`.

The Chromium topology-event checkpoint now replaces manual Page/popup and
child-Frame/OOPIF publication with optional production components. It is deliberately
caller-driven: selecting it declares one fixed session/decode memory profile,
while every `receive_once` initializes newly visible target-local sessions,
consumes at most one bounded broadcast event, and commits through the existing
topology/runtime/route transaction. A separate Page-discovery profile installs
browser-root flattened auto-attach and its Chromium ordering barrier without
adding a portable capability or task. It enumerates bounded non-default
contexts, represents the unique default Context without an invented native ID,
initializes the complete existing frame tree while a target is paused, resumes
it explicitly, then atomically publishes Page, frame tree, and Ready state. It
returns `PageReady`/`PopupLinked` in the same ordered update.
Generated Page/popup, local Frame, OOPIF target, execution-world, detach,
root-navigation, and Page-crash events are normalized.
Fake races prove existing/new Page admission, popup gating, foreign-context
rejection, malformed-tree rollback, local-to-OOPIF identity continuity, late
old-world destruction, two-source detach synchronization, sticky lag,
same-connection authoritative Context/Page/Frame reconstruction, and
stale-epoch rejection. A Chrome 152 production-composition run discovers its
OOPIF and unique main world without test-side child-event parsing, then
evaluates through the portable child Frame. Repeated Chrome 152 discovery runs
also prove the browser's opaque default-context ID, an existing target, popup
unpause/link ordering, a 1,100-event overflow of a 1,024-slot ring, ObjectId
preservation across new sessions, cleanup, and external browser survival. This advances
DT-032, DT-034,
DT-052, DT-060, DT-061, and DT-070. Browser/default-context reconstruction,
Page/popup discovery, initial full-tree setup, live-transport event-loss
recovery, and the optional portable `BrowserContext.page`/`Page.popup`
contracts, codecs, subscriptions, and bounded publication bridge are
implemented. The bridge can also hold ordered control through the final
required `Create` before admitting a discovery event. A caller-owned topology
pump now applies a bounded terminate/reconstruct policy, rejects ambiguous
same-connection replay when portable discovery callbacks are installed,
detects an abandoned polled turn, and terminates the supervisor as
`BackendEventFailed` through both cleanup barriers. The supervisor now polls
that pump as a retained fourth lane beside its unique reader, bounded command
set, and serial outbound lane with rotating priority. Its complete application
run performs reasoned cleanup and write-half shutdown while preserving dual
failures. Constructing a new remote Pwright transport together with a fresh CDP
attachment, runtime object graph, and unambiguous event view remains open, so
DT-052 and DT-061 are not complete. Native CDP sessions will not be replaced
inside an existing Browser graph.
The boundary is recorded in
`knowledge/pwright-chromium-topology-events.md`.

The next DT-052/DT-061 checkpoint resolves an earlier ambiguity by inspecting
the local Playwright sources. Chromium protocol closure is terminal there:
`CRConnection._onClose` disposes the root session, `CRBrowser._didDisconnect`
closes Pages and workers, Browser closes Contexts and emits `disconnected`, and
`BrowserDispatcher` closes and disposes the channel object. Pwright now exposes
one owned `CdpConnection::closed()` future and an independent,
caller-polled `ChromiumCdpLivenessBackend`. It makes native closure a terminal
supervisor backend failure even when no topology-event capability is selected.
A generic `ConcurrentServerBackendChain` rotates ready backend plugins fairly,
never cancels their retained turns, and forwards terminal cleanup to both.
`AttachedChromiumComposition::server_backend` is the canonical constructor and
always composes mandatory liveness with the caller's optional plugin.
Tests prove sticky connection closure, sticky topology-stream closure, plugin
fairness, and release of a pending topology turn when liveness terminates the
connection. Transparent CDP WebSocket replacement was removed: a new native
attachment must create a new Browser/object graph. Establishing a complete
fresh remote Pwright client attachment, runtime graph, and event view remains
open, so DT-052 and DT-061 are not complete.

The following checkpoint adds the reusable `ChromiumAttachFactory` above the
one-shot preparation. Factory construction validates the capability profile,
complete per-connection resource limits, and Browser-root ObjectId namespace
without I/O. Each `prepare` creates a distinct production ledger and connector;
each `attach` consumes that fresh preparation to create a distinct CDP
connection, topology, ObjectId source, and epoch-1 history. Tests compare ledger
identity across two preparations and prove invalid capability selection and an
exhausted root namespace fail before CDP. The factory intentionally stores no
runtime, topology, connection, event receiver, task, or browser process.

The next staged checkpoint now binds that attachment to a caller-built
production transport. `attach_remote` rejects a missing handshake before CDP,
passes the fresh ledger into the one-shot transport-builder plugin, verifies
the returned transport's exact plan and ledger identity, constructs the
handshake-first supervisor, and claims the attachment's only topology-event
driver. Consuming `install_runtime` accepts a caller-selected runtime installer
but enters `ChromiumRemoteConnection` only when the attachment confirms a real
one-shot runtime was installed. The final unit owns the fresh CDP attachment,
remote supervisor/transport, runtime graph, and optional unique topology-event
baseline without choosing a task, listener, executor, deadline, cleanup, or
capability service stack. Tests prove exact transport acceptance, foreign-ledger
and plan-drift rejection, and missing-handshake failure before endpoint I/O.
Dedicated Chrome 152 release tests also prove the staged runtime has only its
fresh Browser root, a second topology-driver claim is rejected, and a no-op
runtime installer cannot produce the final connection state.
Post-handshake typed event publication and complete handler/backend assembly
remain open, so DT-052 and DT-061 are not complete.

This is executable evidence toward DT-032, DT-033, DT-050, and DT-061, not
their completion. The optional Tokio executor now supports pre-reserved
continuations plus a caller-selected nonzero job deadline. Deadline expiration
drops the stuck job, releases its permit, records `timed_out`, and lets an owned
Chromium shutdown's kill-on-drop guard escalate; runtime shutdown records the
distinct `cancelled` outcome. Shutdown-wide graceful drain, retry policy,
application reporting callbacks, and end-user API stabilization remained open
at that checkpoint.

The application-policy checkpoint now closes the executor supervision gap.
Shutdown atomically closes new admission and drains both preaccepted
reservations and running jobs; an optional nonzero drain deadline reports
outstanding counts without cancelling them, and eventual completion publishes
one drained transition. Retry is opt-in per logical job and requires a factory
for a fresh future on each finite attempt, so the executor never retries a
unique consuming continuation such as owned Browser shutdown. One permit spans
the complete attempt/backoff sequence. A synchronous callback reports
admission, retry, terminal, and shutdown events; callback panics are contained
and counted. Deterministic Tokio tests cover shutdown/admission races, retained
reservations, retry accounting, deadline then eventual drain, and callback
failure. A launch-composition test proves a dropped, already-started
`Browser.close` continuation is included in application shutdown and empties
the runtime tree before drain resolves. End-user API stabilization remains
open, so this is still opt-in foundation rather than a frozen public v2 release
promise.

The wire production checkpoint now calculates that previously open overlap.
`FrameLimits` first derives the largest structurally attainable frame body.
The stream envelope profile then optimizes attachment count together with its
header cost and shrinking body room. One outbound operation includes its
budgeted frame, encoded envelope, and length-prefixed pending write; one inbound
operation includes its envelope and decoded frame; both are added for the
independent full-duplex reader/writer case. Production Tokio construction must
claim the resulting `WireBytes`, scope, and lease profile.

Budgeted frames now retain the identity of their exact connection ledger even
when they contain zero bytes. Production send rejects raw frames and frames
from another ledger before envelope allocation. The codec-neutral
`WireProtocol` also has a production claim and stamps its output with that
identity. This closes the protocol/stream production gate without making core
depend on Tokio.

The result-handoff audit found that the earlier protocol reserved only after a
codec had already allocated its payload. Production encode now reserves twice
the structurally attainable frame size before invoking the codec, explicitly
covering the bounded typed source and encoded buffer overlap. Successful encode
atomically shrinks the same lease to exact payload/attachment bytes and moves
it from the staging connection scope to the final call/object scope. Codec,
frame-validation, and resize failures drop the reservation. Tests inspect the
ledger from inside the codec to prove admission precedes allocation and prove
failure release. Direct in-process results returned to application code are
caller-owned; the remote runtime does not retain a separate result history.

The encoded-client production checkpoint adds a transport-plugin marker rather
than specializing the client for Tokio. A `ProductionFrameTransport` exposes
its complete profile and actual ledger. `ClientWireProductionProfile` combines
that profile with directional frame maxima and the pending-call bound. It
covers both full-duplex I/O with remaining queued frames and the distinct state
where encoding precedes a rejected registration against a full pending table.
Production direct, cancellation-control, negotiated, and negotiated-control
constructors claim the combined plan and reject a transport from another
connection ledger.

Typed event queues now have their own production profile. Capacity determines
`EventItem`, scope, and lease maxima; capacity multiplied by an explicit
maximum decoded size determines `EventBytes`. The existing event-specific
sizing callback is checked against that local declaration before ledger
mutation, preventing one plugin from exceeding its contract by borrowing
another plugin's unused global capacity. Zero-byte events still own an item
lease, and omitted event plans fail during construction.

Chromium topology now has the next production profile. Its derived byte ceiling
counts all duplicated native strings across bidirectional indexes, pending and
materialized page/frame records, session owners, crash state, and execution
worlds. Before a production mutation clones the pure topology, it reserves a
maximum-sized shadow; successful application shrinks that lease to the exact
retained string capacity and atomically replaces the old state/lease. Failed
topology or resource admission leaves the live map unchanged. Inputs are
compacted before retention, closing the short-string/large-capacity loophole.
The composition root must assign an extension scope; non-extension attribution
is rejected before construction.
AAA tests cover deferred materialization, crash/world inventory, detach,
reconnect, exact release, undeclared plans, and contended shadow rejection.

The locator backend audit found no variable-size long-lived service state; it
retains only a client handle, duration, `OnceCell<()>`, and atomic metrics. Its
active action future is the real owner of selector/value input, display label,
generated JavaScript, and possible runtime-install source. The new
`CdpLocatorProductionProfile` derives a conservative transient peak and scales
it by an explicit concurrency limit. Production calls validate spare string
capacity, acquire a clone-shared local slot, and reserve `SelectorBytes` before
returning a future. An unpolled future is charged, drop/completion releases, an
overprovisioned shared ledger cannot bypass the plugin's local concurrency, and
an omitted plan fails construction. Other backend services and future
capability factories remain DT-060 work.

The navigation DT-060 checkpoint closes the action-local growth paths found by
the audit. `CdpNavigationLimits` bounds input/result URLs, native identifiers,
simultaneously retained request IDs, response-header count and aggregate bytes,
and status/error text. `CdpNavigationProductionProfile` derives a conservative
per-action `BackendStateBytes` peak and scales it by an explicit concurrency
limit. Production calls acquire a clone-shared local slot and reserve the full
peak synchronously before returning a future. Request IDs are checked before
set insertion; response header count, types, and aggregate bytes are checked
before owned copying; retained strings are compacted. Tests prove construction
failure for an omitted plan, unpolled and polled drop release, completion
release, local concurrency enforcement despite an overprovisioned ledger, and
failure-before-retention for request and header floods. Result ownership after
backend completion remains a distinct DT-060 boundary, so the remote production
server profile remains intentionally incomplete.

The following raw-CDP checkpoint closes the transport-side event queue and
pending-command paths. `ConnectionConfig` now bounds pending commands, channel
items, command/response/event/error/WebSocket bytes, and rejects zero or
inconsistent capacities instead of allowing a channel panic. Command
registration and the production `WireBytes` reservation happen synchronously;
an RAII guard removes abandoned entries, and connection shutdown drains them
even when cancellation drops the reader loop. Bounded JSON serialization stops
before an oversized command is retained. Outgoing queue items share the command
lease, so caller cancellation cannot release bytes the writer still owns.

Events share one immutable JSON payload rather than deep-cloning per subscriber.
In production each unique event carries one exact shared
`EventItem`/`EventBytes` lease; retaining a delivered value consumes the finite
budget and makes the connection fail closed. `CdpConnectionProductionProfile`
declares the fixed reader overlap, concurrent command peaks, event capacity,
and one allocation-before-admission transient. Legacy v1 keeps the Tokio
broadcast receiver. The opt-in v2 path adds a task-free, preallocated fanout
that drops events with no subscriber and reclaims each slot after every current
cursor advances. Its subscriber capacity and fixed bookkeeping are explicit;
the Chromium composition can derive the former from selected plugins and
reject an undersized plan before CDP I/O. The attach-only
`ProductionCdpConnector` verifies plan inclusion and exact config/profile
identity before network access. Raw connection and live connector constructors
remain explicit compatibility paths.

The runtime execution-control checkpoint turns command metadata into enforced
behavior while keeping the generic runtime executor-independent:

- `OperationContext` and `CommandCancellation<Command>` provide a typed,
  backend-neutral cancellation hook without retaining command parameters;
- `ControlledDispatcher` races operation completion, a validated external
  cancellation signal, and an injected relative-deadline future;
- an accepted cancellation wakes the dispatch future, drops the operation
  future, awaits backend cleanup, and only then removes its pending-call entry;
- wrong cancellation identities do not wake or alter the running operation,
  and the first accepted reason wins cancellation/deadline races;
- observable operation completion wins if it is ready in the same poll;
- cleanup failures are classified separately as `Cleanup`;
- `DirectDispatcher` rejects execution-control metadata instead of silently
  ignoring it;
- `pwright-runtime-tokio` supplies an explicitly installed Tokio timer adapter,
  while `pwright-runtime` continues to depend only on `pwright-core`.

The CDP navigation adapter now implements its portable cancellation hook using
`Page.stopLoading`. The v2 foundation passes 58 non-browser release tests plus
the compile-fail ownership doctest. A third explicit live test used a hanging
HTTP socket bound to this Mac's Tailscale interface: real Chromium began a
pending `Page.navigate`, the runtime accepted the matching cancellation ID,
the CDP cleanup completed, the command returned `Cancelled`, the pending table
returned to zero, and the owned incognito context and exact target disappeared.

This is executable evidence toward DT-033 and DT-061, not completion of either.
Dropping a future still cannot await backend cleanup without a bounded
connection-level cleanup executor; transport-close fan-out, nested deadline
inheritance, and shutdown draining remain future slices.

The lifecycle and target-admission checkpoint joins the previously separate
object graph, pending-call table, and command dispatcher:

- controlled subtree disposal first computes a deterministic child-first plan,
  removes the objects, marks their active and queued calls `ParentClosed`, and
  wakes those calls so admitted operations run their backend cleanup hook;
- lifecycle cancellation already accepted by the runtime wins a simultaneous
  operation-result race, while caller cancellation and deadline expiry preserve
  the earlier operation-completion-first rule;
- the direct dispatcher refuses disposal of a subtree with pending calls rather
  than removing objects it cannot safely clean up;
- a standard-library-only target scheduler admits one command per target,
  preserves per-target FIFO order, and lets distinct targets run concurrently;
- target lanes and queued calls have hard limits, duplicate call IDs fail, and
  current/peak metrics remain bounded and observable;
- dropping or cancelling a queued acquisition removes it cleanly and does not
  call backend cancellation because the capability operation never started.

The v2 foundation now passes 69 non-browser release tests plus the compile-fail
ownership doctest. `pwright-runtime` still has only `pwright-core` as a normal
dependency; the optional Tokio timer remains isolated in
`pwright-runtime-tokio`. This is executable evidence toward DT-032, DT-033,
DT-060, and DT-061, not completion: transport-close fan-out, a bounded cleanup
executor, connection-wide admission/fairness, object-backend disposal ordering,
and multi-command transactions remain open.

The connection-termination and abandoned-call-cleanup checkpoint makes runtime
shutdown and dropped futures observable rather than implicit:

- transport closure is terminal and idempotent; it invalidates every object in
  deterministic child-first order, requests `TransportClosed` for all pending
  calls, wakes active and queued calls, and rejects later registration or
  dispatch;
- an accepted transport close preempts a simultaneously ready backend result,
  but preserves an earlier cancellation reason under the existing first-wins
  rule;
- transport loss never attempts `Page.stopLoading` or another abort over the
  already-dead transport; active operation futures are still dropped;
- dropping an admitted controlled-dispatch future removes its pending and
  target-admission state, drops the backend operation, and then submits exactly
  one owned cancellation job; dropping a queued or never-polled call submits no
  backend cleanup;
- the default `CleanupQueue` is standard-library-only, bounded, FIFO, and
  externally drained. Saturation rejects explicitly and reports accepted,
  rejected, queued, peak, completed, and failed counts; no runtime task is
  spawned.

The optional `pwright-runtime-tokio` adapter now also supplies a
`TokioCleanupExecutor`. It requires an active caller-owned Tokio runtime, bounds
active cleanup jobs with a fixed semaphore, rejects saturation or missing
runtime state, and reports current/peak, accepted, rejected, completed, and
failed counts. It does not construct a hidden runtime or thread.

The connection-wide admission checkpoint adds a second bounded scheduler above
backend execution. Commands first obtain their per-target FIFO position and
only target heads enter the connection FIFO. This prevents followers on one
busy page from reserving global capacity, caps aggregate work across independent
targets, and defines fairness as FIFO among target heads. Cancellation and
deadline races remain active in both admission stages and release all acquired
permits without invoking a backend hook before the backend starts.

`RuntimeLimits::new` uses the pending-call bound as its default active-command
bound; `with_active_commands` can lower it independently. Both direct and
controlled dispatchers expose current and peak connection admission metrics.

The codec-neutral wire checkpoint adds `pwright-transport`, which depends only
on `pwright-core`. It provides typed frame routes; hard payload, attachment, and
aggregate byte limits; exact-length fragmented-byte assembly; inclusive
same-major version policy; generic codec and executor-neutral transport traits;
and a bounded client-side call correlator. Both outgoing codec output and
incoming frames are checked against the local connection limits before a codec
decoder receives them. No JSON, Serde, WebSocket, Tokio, authentication, or
reconnection policy is selected by this crate.
The generic exact-length assembler now also accepts an optional shared ledger:
it reserves the full declared `WireBytes` before buffer allocation and transfers
the lease into its non-cloneable finished output. Rejection, incomplete input,
assembler drop, and output drop have deterministic accounting, so custom
non-Tokio stream adapters no longer need an unbudgeted fragment path.

The optional JSON-codec checkpoint adds `pwright-codec-json` as the only new
crate importing Serde and `serde_json`. It serializes any generated or
application message implementing `RoutedMessage`, verifies that decoded routing
matches the outer frame route, and explicitly rejects malformed JSON and binary
attachments. Its dependency tree is separate: `pwright-core` remains
dependency-free, while `pwright-runtime` and `pwright-transport` still depend
only on core.

The handshake checkpoint extends `pwright-transport` with bounded owned wire
capability IDs, installed manifests, required/optional requests, and
codec-neutral client/server hello values. Negotiation chooses the highest minor
in the common major range, preserves directional receive limits, fails on
missing or incompatible required capabilities, omits unavailable optional
capabilities, enables no unrequested capability, and returns deterministic
capability ordering. No browser operation can start as part of negotiation.

The typed-client checkpoint adds `pwright-client`, whose normal dependency
graph contains only `pwright-core` and the core-only
`pwright-browser-model` vocabulary.
`Client` owns the channel and injected call-ID source; typed `Page` handles keep
only weak references, compare connection identity as well as object identity,
and fail deterministically after disconnect. The atomic ID source cannot wrap,
and typed in-process commands reach `DirectDispatcher` without a codec.

The typed-object checkpoint extends the client with `Browser`,
`BrowserContext`, `Page`, and `Frame` handles. Codec-neutral initializers must
match the expected kind, root status, and exact typed parent; disconnected
parents cannot create children. The client deliberately does not mirror the
runtime object graph. A generic `Locator<Query>` is a local value scoped to a
page or frame, allocates no remote object ID, lets selector capabilities own the
query representation, and combines only when both values share exact remote
scope and connection identity.

The object-kind checkpoint removes the client-owned numeric kind table.
`pwright-core` provides generic semantic IDs, descriptors, catalogs, and marker
traits; the core-only `pwright-browser-model` owns the canonical Browser,
BrowserContext, Page, and Frame vocabulary. Capability schemas declare
canonical command targets, which are emitted on `CommandSpec` and included in
minor compatibility. Remote server inventories reject missing target metadata,
and catalog-configured runtimes reject unknown lifecycle tags and wrong-kind
dispatch before scheduler/backend work. Parent descriptors declare the
Browser→BrowserContext→Page→Frame topology. Explicit `Partial` mode supports
externally attached subgraphs while validating every present parent;
`Complete` mode requires non-root kinds to have a valid parent, and both Create
and Adopt use the same check. This adds no Node.js, browser binary, download,
provider, backend, transport, or executor dependency.

The server-configuration checkpoint threads that catalog into capability
inventory construction before handshake. Every target declared by an
advertised command must resolve to an installed descriptor; otherwise manifest
construction fails before capability negotiation or command traffic. The
catalog remains local configuration rather than a browser acquisition request.

The client-liveness checkpoint adds a bounded object-ID-to-token table with a
1,024-object default and an explicit configurable limit. It reports current,
peak, and limit metrics, rejects duplicate live initializers, and invalidates
existing handle clones synchronously and idempotently from runtime disposal
notifications. Ordered bulk application consumes the runtime's child-first
result without copying its parent graph. A stale operation now reports
`TargetClosed`, while a dead client owner reports `TransportClosed`; neither
handle nor client `Drop` sends protocol traffic.

The opt-in client-capability checkpoint moves navigation into
`pwright-client-navigation`. Its `PageNavigationExt` trait adds
`begin_navigation` only when the extension crate is installed and imported;
the base client knows no navigation types. Generic `Remote::dispatch` owns
call-ID allocation, handle/connection validation, portable failure mapping,
and response-ID correlation for every future capability. The navigation
extension still deliberately exposes `begin_navigation`, not a false
Playwright-compatible `goto`.

The later goto checkpoint keeps that primitive intact and adds a separate
`PageGotoExt`, so channels may install either command contract independently.
The portable high-level command defaults to `load`, supports Playwright's four
wait states, carries referrer, and leaves timeout/cancellation in shared call
metadata.

The encoded-client checkpoint adds opt-in `pwright-client-wire`, depending only
on `pwright-core` and `pwright-transport`. Its cloneable typed command channel
and unique caller-driven receive driver share a bounded pending/tombstone table;
they start no executor task. Commands register before send, non-result frames
are returned for composition, and typed decode happens after raw call-ID
correlation. Send failure, receive failure, EOF, explicit close, driver drop,
and codec protocol violations terminally wake waiters. Dropped send attempts
retain a bounded tombstone so one late result is ignored and releases its slot.

The public-future cancellation checkpoint upgrades that tombstone into a
bounded cancellation outbox when an application explicitly supplies a control
codec. Matching command/control version policy and limits are validated during
construction. Missing cancellation identities are synthesized per wire call;
dropping a possibly sent future queues one `Caller` cancellation without doing
I/O in `Drop`. Commands and control share an executor-neutral bounded send gate:
dropping a command still waiting for that gate rolls back without a tombstone,
while only the unique active sender is classified as possibly sent. Control
waiters take priority over already queued commands, and a completed frame send
releases the gate before its result wait. The cancellation-aware driver keeps
its receive future pinned, flushes control on the independent write path, and
later consumes the cancelled result to release the tombstone. Fixed metrics
expose queued, peak, and sent cancellations. Encoding failure rolls back, send
failure closes and clears the channel, and the no-control profile retains its
previous smaller behavior.

The Tokio transport now retains one bounded partially written envelope and its
offset, so dropping a send future cannot split the stream; the next send or
shutdown completes it first. A Tokio duplex test carries a dropped public
`Locator.fill` future through the canonical control codec into a
`RuntimeControlReceiver`, proves the runtime cancellation hook receives the
same call/object/method identity, observes `Cancelled`, and drains all client
and runtime state. A separate 16-byte-pipe test interrupts a frame mid-write
and proves the resumed first and following frame both decode in order.

The bounded server-supervision checkpoint replaces the cancellation proof's
manually separated command/control polling with the actual application-phase
connection loop. `ConcurrentTypedCommandHandler` produces owned typed work and
its static chain preserves capability-by-capability composition. One
caller-driven `EncodedServerSupervisor` retains the only receive path, polls
every active command fairly, and applies synchronous control transitions while
commands are pending; it starts no task or runtime. Its nonzero in-flight limit
is explicit. Accepted commands are polled before the next receive so their
cancellation identity exists in the runtime. A command received at capacity
closes the connection rather than entering an uncancellable hidden queue.
Current/peak/accepted/completed/control/unhandled/capacity/dropped metrics and
typed terminal reasons are exposed. Dropping the run future releases all work,
records a terminal state, and forbids restarting a potentially partial reader.

The handshake-and-half-close checkpoint makes the same supervisor an explicit
connection state machine. A handshake-required constructor prevents
application reads until a bounded ClientHello passes an injected async
authorization gate, capability negotiation, ServerHello encoding, and send.
The returned authorization context lets the application install only handlers
allowed for that peer; `AllowUnauthenticated` is a named opt-in rather than a
remote default. Negotiated version and receive limits guard every later frame,
and rejection sends no ServerHello. Dropping pending authorization is terminal.
EOF policy is either immediate drop or control-driven drain. The drain path
stops reading, marks the controlled runtime transport closed, wakes admitted
commands, writes their terminal results through the remaining half-connection,
and closes only after active work reaches zero. A real Tokio duplex test proves
a pending Fill returns `TransportClosed` and releases every runtime object and
call after the client write half closes.

The client-handshake checkpoint closes the other half of that state machine.
`WireProtocol` now has separate send and receive limits while its original
constructor remains a symmetric compatibility path. A consuming
`EncodedClientHandshake` owns the bootstrap transport and is the only route to
`NegotiatedClientTransport`; application channel factories are therefore
unavailable until ServerHello passes exact outer/selected-version, echoed
client-limit, requested-capability, required-capability, and minimum-version
checks. The resulting client and server protocol factories invert the same two
directional limits and pin application traffic to one version. Dropping a
pending client handshake drops its transport handle. A real Tokio duplex test
performs both peer handshakes and a typed runtime round trip on the same stream.

The authenticated-local-endpoint checkpoint adds optional
`pwright-endpoint-tokio` above the generic transport and server layers. A
private Unix listener uses mode `0600`, starts no task or accept loop, and reads
kernel peer UID/GID/PID. The client verifies an injected expected server UID;
the server converts the accepted identity into a UID authorization gate that
runs before capability negotiation. Existing paths are never removed, and
device/inode-checked cleanup cannot delete a replacement path. Tests cover
permission mode, both peer identities, UID rejection, bounded frame I/O,
pre-hello authorization, and safe pathname ownership. That same-UID Unix
profile does not authenticate a network peer.

The authenticated-Tailnet checkpoint adds the first non-Unix adapter without
weakening the generic transport or handshake. `TailnetEndpoint` rejects every
bind, server, and accepted-peer address outside Tailscale's
[`100.64.0.0/10` and `fd7a:115c:a1e0::/48` ranges](https://tailscale.com/docs/reference/reserved-ip-addresses).
Before constructing a frame
transport, both peers prove one fixed 32-byte PSK using a CSPRNG 32-byte server
nonce and direction-separated HMAC-SHA256 proofs. The key is never transmitted
or formatted and its owned allocation is zeroized. The fixed versioned exchange
is exactly 105 bytes; it has no length-controlled allocation, retry queue,
accept loop, task, or implicit deadline. A proof-carrying `TailnetPeerGate`
then supplies the authenticated source address to the unchanged Pwright
handshake gate.

Tests reject loopback/LAN/public binds before socket creation, cover the exact
IPv4/IPv6 boundaries, wrong client and server proofs, reflection, replay under
a fresh nonce, and preservation of the following frame boundary. A manually
selected integration test bound this Mac's actual Tailscale address and ran the
real PSK exchange followed by the canonical ClientHello/ServerHello handshake.
This PSK adapter remains expressly Tailnet-only.

The authenticated-mTLS checkpoint adds a separate general TCP adapter rather
than weakening that Tailnet policy. The caller supplies rustls client/server
configuration, trust roots, server name, and a fixed-capacity set of authorized
SHA-256 leaf-certificate fingerprints. Both sides require the exact
`pwright/1` ALPN, normal rustls certificate verification, a presented peer
certificate, and an exact pin before a frame transport exists. A two-entry pin
set permits a bounded old/new overlap during rotation; empty and duplicate sets
are rejected. Only a successfully accepted value can construct
`MutualTlsPeerGate`, so the unchanged Pwright handshake receives proof-carrying
peer identity rather than credentials in ClientHello.

The endpoint rejects wildcard binds, acquires one of a caller-declared number
of handshake slots before accepting another socket, and starts no accept loop,
task, runtime, or clock. rustls 0.23 defensively caps one buffered handshake
message at 65,535 bytes and one certificate at 65,536 bytes; each TLS
connection also receives an explicit outbound buffer ceiling equal to the
larger of 65,536 bytes and the configured length-prefixed Pwright envelope.
Those rustls-owned buffers are a documented third-party boundary, not falsely
charged as exact Pwright `WireBytes`. The caller still owns handshake/drain
deadlines, PKI issuance and revocation, durable key storage, and the operational
rotation schedule.

Tests cover mutual authentication, absent client certificates, a CA-valid but
unpinned server, exact ALPN, bounded pin rotation, bounded output buffers, and
preservation of the first Pwright frame after TLS. A selected live test bound
this Mac's actual Tailscale interface and completed mTLS, pin verification, and
the canonical ClientHello/ServerHello exchange. This completes DT-035's
transport/authentication boundary foundation; deploying a public listener
still requires an explicit application policy under DT-063.

The authenticated Chromium endpoint checkpoint adds the optional
`pwright-composition-chromium-tokio` leaf rather than making the base Chromium
root depend on endpoint policy. Its `AcceptedChromiumEndpoint` boundary covers
Tailnet and mTLS directly and wraps Unix with one explicit allowed UID. It
keeps the endpoint-derived gate paired with the exact accepted transport,
claims the predeclared Tokio stream retention plan from the fresh Chromium
ledger, carries the pair through runtime installation, and consumes the gate
inside same-supervisor handshake acceptance. No constructor exposes a way to
substitute another gate between those stages. Release tests prove the concrete
authenticated endpoint families produce statically typed production
transports and share the caller's exact ledger. A dedicated Chrome 152 test
proves a real kernel Unix peer identity reaches authorization and that dropping
the resulting connection leaves externally owned Chrome attachable. Listener
loops, admission concurrency, deadlines, and full application plugins remain
caller-owned. This advances DT-035, DT-041, DT-043, DT-051, DT-052, DT-060,
DT-061, and DT-063 without selecting a universal deployment transport.

The bounded-drain checkpoint adds `run_with_drain_deadline` without importing a
clock or executor into `pwright-server-wire`. Its injected future is dormant
until EOF leaves admitted work active. Commands that become ready during the
transport-close transition win before an simultaneously ready deadline;
otherwise expiry drops the remaining bounded work, records one deadline metric,
and terminates with `DrainDeadlineExceeded`. Tests cover both permanent pending
work and close-released work. The original `run` keeps its unbounded
compatibility behavior, so a production endpoint policy must deliberately
choose and inject a duration.

The capability-range and binary-contract checkpoint removes the assumption
that an installed capability is represented by only one minimum version.
Client requests and server manifests can now publish inclusive ranges, and
negotiation selects the highest common version. Disjoint required ranges fail;
disjoint optional ranges are omitted; the client rejects a selected version
above its maximum. The ranged ClientHello has a new canonical tag while the
decoder retains the legacy minimum-only representation.

The new `pwright-codec-binary` crate centralizes allocation-aware network-order
writing, borrowed bounded reading, exact malformed-input diagnostics, and
static positional schema metadata. Control, handshake, navigation, and locator
codecs now use it and publish validated top-level layouts, with golden-byte
coverage proving their encodings did not drift. Existing positional variants
cannot change within a major, while higher-tagged variants may be appended in a
minor after capability negotiation. Navigation header decoding now preflights
the minimum bytes and uses fallible reservation before allocating entries.
This is evidence toward DT-031 and DT-035, not completion of DT-031: nested
messages are still opaque, and an evaluated source IDL, generated Rust and
cross-language types, compatibility fixtures, and codegen remain open.

The handler-inventory checkpoint binds every remotely advertised command spec
to its owning capability and command ID. Sequential and concurrent static
handler chains can now derive the bounded handshake manifest directly. The
inventory rejects missing handlers for any command declared by a capability,
duplicate command or wire-method handlers, descriptor conflicts, undeclared
commands, and invalid dependency graphs before a ClientHello is accepted.
This closes the configuration drift where a server could maintain its handler
chain and advertised capability list independently; the raw manifest path
remains only as a compatibility and low-level construction API.

The nested-schema checkpoint replaces untyped `Message` placeholders with
stable referenced type IDs and adds distinct record/tagged-union descriptors
plus a canonically ordered catalog. Catalog validation accepts intentional
recursive types such as selector nodes without recursively walking values, and
rejects unknown references before generation or negotiation. Existing codec
top-level schemas now name their nested types. Complete per-capability catalogs,
an external source syntax, generators, and release-to-release nested
compatibility remain open, so DT-031 is still not complete.

The first source-IDL checkpoint records the local Playwright schema/generator
evidence and evaluates YAML, Protocol Buffers, FlatBuffers, and Rust-only
declarations. A strict, smaller Playwright-shaped YAML grammar is selected
provisionally, owned independently by each capability. The development-only
`pwright-schema-gen` crate rejects unknown keys, noncanonical IDs/tags,
duplicate fields, missing references, excessive shapes, and unsupported
recursive Rust models, then deterministically emits Rust wire models and a
complete static catalog. It has an explicit non-mutating `--check` mode; there
is no `build.rs`, and generated capability crates acquire no YAML, Serde, Node,
or generator dependency.

Navigation is the first representative source and generated catalog. Its
existing canonical bytes did not change. The grammar now includes strict
booleans and stable-tag enums; enum evolution is append-only within a compatible
minor release. Catalog-level compatibility now
allows new types and appended higher-tag union variants but rejects removed or
retyped types, record layout changes, existing union changes, version
regression, and changed nested references. The evaluated alternatives,
commands, and remaining gates are recorded in
`knowledge/pwright-protocol-idl.md`. At that checkpoint DT-031 remained open for
declarative string/collection/cross-field validators, dispatch metadata,
user-facing docs, transitive imports, recursive models, generated public-model
conversions, and a second-language fixture.

The generated-value-codec checkpoint adds shared `BinaryEncode` and
`BinaryDecode` contracts plus an explicit `BinaryValueContext`. String bytes,
binary bytes, repeated items, nesting depth, and total message nodes all have
caller-supplied hard limits. Decode validates lengths before copying, performs
fallible string/byte/vector reservation, and uses a schema-derived minimum item
size to reject impossible repeated counts before allocating. Generated
navigation models now implement these codecs; a simple generated goto is
byte-for-byte identical to the published hand-written payload, while tests
prove field-attributed string/depth failures and header-body preflight.

`pwright-schema-gen --previous` now compares the previous published source with
the candidate before generation or drift checking. Navigation keeps a checked-in
1.0 baseline. New types and appended union variants pass; record changes and
mutated old variants fail in generator tests. At that checkpoint DT-031
remained open for string/collection/cross-field constraint generation,
generated public-command conversions, dispatch/docs generation, transitive
imports, recursive models, and cross-language fixtures.

The navigation production codec now uses the generated models and bounded
generated codecs for BeginNavigation and Goto commands and results. This covers
success values, response headers, failure-kind enums, strict booleans, and typed
error details; the parallel hand-written navigation payload implementation has
been removed. At that checkpoint the conversion boundary still owned aggregate
header limits, canonical header ordering, and mechanical public-model mapping.

That memory checkpoint now emits lifetime-parameterized borrowed models for
every schema type that transitively contains UTF-8, bytes, or repeated values.
Repeated values cross an object-safe sequence reference; generated owner code
still applies the count bound and emits the field layout. Navigation commands,
successes, errors, and responses now encode through these views. The canonical
header sequence uses deterministic allocation-free selection and preserves
duplicates, eliminating cloned strings and the intermediate owned header
vector. Owned generated models remain the decode boundary. At that point DT-031
no longer listed borrowed encoding as open; generated public-model conversions,
declarative cross-field constraints, transitive imports, recursion,
dispatch/docs, and another language remain.

The numeric-constraint checkpoint adds inclusive `minimum`/`maximum` metadata
without narrowing the `i64 | u64` source domain. Source and catalog validation
reject empty, reversed, wrong-kind, and out-of-wire-kind ranges. Generated
owned and borrowed encoders plus decoders enforce the bounds, and ranges enter
layout fingerprints and exact minor compatibility. Navigation duration
nanoseconds now declare `maximum: 999999999`; generated decode performs the
check before `Duration` construction and the previous hand-written validation
branch is gone.

The shared-library checkpoint separates reusable wire values from capability
roots. A source declares exactly one `library`, `capability`, or non-installable
`protocol` family; capabilities list direct imports and the generator requires
the matching library sources.
`pwright-codec-common` now owns call metadata, duration, and failure kind.
Navigation reexports those exact generated Rust types and stable schema
constants, while its composed catalog and compatibility checks include the
imported graph. Consumers still install only the capability and its small
normal codec dependencies: no YAML parser, generator, Node runtime, transport,
executor, or browser is pulled in. Transitive imports and recursive import
graphs remain deliberately unsupported while their version policy is designed.
The CLI accepts separate repeatable `--import` and `--previous-import` paths,
so a capability compatibility check resolves the actual new and published
dependency graphs rather than comparing both sides against the newest library.

The size-constraint checkpoint adds inclusive UTF-8/byte `minimum_length` and
`maximum_length` plus repeated `minimum_items` and `maximum_items`. Source and
catalog validation reject empty, reversed, or shape-incompatible ranges.
Generated encode checks before writing; decode checks the wire length/count
before copying or reserving. These semantic bounds participate in layout
fingerprints and exact minor compatibility while remaining independent of
caller-selected hard resource ceilings. Per-element length constraints on an
opaque borrowed primitive sequence and cross-field aggregate rules remain open.

The command-metadata checkpoint makes each capability source declare stable
command IDs and method tags together with its command/result root variants.
The generator validates canonical ordering, unique methods, distinct existing
root variants, and forbids commands in a library. It emits public
`BinaryCommandSchema` constants; navigation tests compare both generated
mappings with the typed capability constants. Removing or changing a published
mapping is a minor-compatibility error. Typed marker/spec generation and server
handler-table generation originally remained open because they require separate
output ownership rather than placing capability APIs inside a codec module.

The multi-output ownership decision now fixes the next generator boundary.
The canonical schema moves under the portable capability crate. Generated API,
codec binding, optional static server glue, and protocol reference each land in
their owning crate or docs directory. The portable capability remains core-only;
generic core/runtime/client/server crates remain capability-agnostic; no output
uses `build.rs`. Commands will declare semantic params/results/errors and the
generator will synthesize common call/result envelopes while preserving the
current navigation golden bytes. The exact dependency graph, atomic emission
contract, rejected ownership models, and migration order are recorded in
`knowledge/pwright-generator-output-ownership.md`.

The first multi-output implementation moves navigation's canonical source and
baseline into `pwright-cap-navigation/schema`. Its symbolic command names now
drive a second committed generated artifact containing capability/command IDs,
methods, the command list, and `NavigationCapability::DESCRIPTOR`; the former
hand-written definitions are gone. `--capability-output` renders this core-only
surface alongside the codec output, and check mode detects drift in either.
The verified normal dependency tree for `pwright-cap-navigation` still contains
only `pwright-core`.

The reference-output checkpoint adds a third committed artifact generated from
that same navigation source. `--reference-output` emits deterministic Markdown
covering command IDs/methods/targets, semantic models, wire variants,
positional fields, cardinality, enum defaults, and declared constraints.
Write mode renders every requested artifact before writing; check mode reports
the reference together with stale Rust outputs without modifying any file.

The static-routing checkpoint adds core-only `CapabilityCommandDescriptor`
values and a generated ordered descriptor slice per capability. Each entry
binds command ID, numeric method, and semantic target kinds without containing
a service, codec, executor, backend, or allocation. The generic server
inventory records those descriptors while registering the statically composed
typed handler chain and can resolve them by command ID or method tag. Locator
publishes the same metadata shape while its schema migration remains pending.

The generated-server checkpoint adds a fourth committed output for the same
navigation source. `--server-output` emits a capability-specific crate with
one injected service per command, one shared `WireProtocol<NavigationCodec>`,
and a complete static `ConcurrentServerHandlerChain`. Its normal dependency
tree contains only the portable navigation contract, navigation codec,
generic server boundary, transport contract, and their portable prerequisites;
it imports no runtime, executor, backend, CDP, Node.js, or browser provider.
The chain itself supplies the checked handshake manifest, so adding a schema
command without regenerating the installed server composition is detected as
output drift rather than becoming a silently unadvertised route.

The semantic-command checkpoint completes the portable half of that migration.
Every command now names parameter, result, and typed-error models; published
model names are validated for uniqueness and compatibility. The capability
emitter owns the public records/enums/unions, typed marker structs, and
`CommandSpec`/`CapabilityCommandSpec` implementations, so their former
hand-written navigation definitions are gone. Navigation's root wire union now
embeds those command parameter/result records directly. Because positional
messages are inline, this removes field duplication without adding an envelope
or changing the established bytes. The codec emitter imports only the portable
public types and generates owned `From` conversions in both directions.
Navigation retains only policy-bearing borrowed adapters and validation for
aggregate header bytes and canonical header order.

The capability-dependency checkpoint adds a separate declarative
`dependencies` list to capability sources; it is deliberately independent of
wire-library `imports`. The generator rejects invalid, duplicate/out-of-order,
self, and library-owned dependencies, emits exact minimum versions into
`CapabilityDescriptor`, and treats any dependency change as a major-version
operation. A capability with no dependencies, including navigation, keeps the
same generated surface and dependency tree.

The locator-schema checkpoint exercises the same design with a recursive and
policy-rich capability. `pwright-cap-locator/schema/locator.yml` is now 1.22;
its preserved 1.21 baseline proves the ScrollIntoViewIfNeeded route is a compatible
append-only command after the 1.21 SelectOption checkpoint.
It generates AllInnerTexts/AllTextContents/Blur/Click/Count/Fill/Focus/GetAttribute/Hover/InnerHtml/InnerText/InputValue/IsChecked/IsEditable/IsEnabled/IsVisible/Press/ScrollIntoViewIfNeeded/SelectOption/SetChecked/TextContent/WaitFor IDs, methods, targets, the
recursive boxed selector wire tree, a complete catalog, protocol reference,
and the optional `pwright-server-locator` handler chain. The generator rejects
recursive models that have no finite wire value. `public_external` keeps
validated options, params, errors, and the unit result hand-written in the
core-only capability while preserving their stable schema names and mappings.
The immutable selector and matcher values now live in zero-dependency,
command-free `pwright-selector-model` and are reexported unchanged by
`pwright-cap-locator`. Both protocol majors and all downstream layers retain
the same type identity. The nine-type wire closure is also published as the
command-free `pwright.selector` 1.0 library and generated
`pwright-codec-selector` crate. Generator tests compare each definition exactly
with both published Locator tracks and compile a consumer that imports selector
types without importing Locator commands. Existing fill golden bytes remain unchanged, while round trips now
cover string and regex text/label/role/attribute matching, exact and regex test
IDs, plus `hasText`, `hasNotText`, `has`, `hasNot`, and visibility filters.

The corresponding locator checkpoint also fixes same-Frame identity and
backend behavior. An initialized Page locator uses its main Frame scope, inner
`has`/`hasNot` locators from another Frame fail locally, and all filters remain
lazy AST transformations. The Chromium runtime preserves base candidates for
relative descendant filters and implements visible/hidden selection with
Chromium visibility APIs. A dedicated Chrome 152 test combines all four new
nodes, clicks only the intended card, and proves hidden, excluded, and
text-rejected alternatives were untouched.

The pointer-action checkpoint adds Playwright-shaped `Locator.hover` without
coupling the portable capability to CDP. IDL `f64` support preserves fractional
positions in network byte order and removes `Eq` only from generated models
that transitively contain floats. Hover strictly re-resolves, waits for visible
and stable geometry, optionally scrolls, checks the hit target unless forced,
and never requires enabled state. Its bounded modifiers are canonicalized in
an allocation-free bitset; `ControlOrMeta` resolves from the browser platform.
Chromium emits one mouse-move event, while `trial` performs no mouse move.
Modifier presence is semantic: omission preserves the Page's current keys,
whereas an explicit empty set temporarily releases them. A fixed-size,
Page-scoped input coordinator serializes all main/local/OOPIF Locator routes,
dispatches canonical key-down transitions before the action, and restores the
prior state afterward. Trial still performs those modifier transitions, matching
Playwright's ability to expose modifier-dependent UI without firing the pointer
action. Controlled cancellation restores a dropped transaction; a directly
dropped backend future leaves bounded pending-restore state that the next input
transaction heals before proceeding.

The regex checkpoint keeps Playwright's `RegExp` branch distinct from string
matching. `RegexMatcher` validates canonical ECMAScript flags without pulling a
regex engine into the portable capability, while Chromium compiles the source
inside the bounded selector attempt and maps syntax failures to
`InvalidSelector`. Appended tags cover text, label, positive/negative text
filters, and mixed string/regex role name or description options. A dedicated
Chrome 152 proof exercises each route, runtime-v3 replacement, and invalid
source classification without retaining DOM handles.

The user-facing attribute checkpoint completes the current factory families
without introducing CSS interpolation or global configuration. Placeholder,
alt-text, and title strings compare raw attribute values using exact
case-sensitive equality or non-exact case-insensitive substring semantics;
their regex branches also use the raw value. Test-ID strings are exact and
default to `data-testid`. Explicit comma-separated names are validated and
capped at 16, then stored in each immutable selector so independently composed
clients cannot race on mutable process-global state. Schema 1.5 appends four
selector tags and one fixed attribute enum; runtime v6 retains those leaves.
A dedicated Chrome 152 proof clicks all five routes after navigation and shows
that exact placeholder matching does not collapse doubled whitespace.

The snapshot-query checkpoint separates reads from single-target actions.
`Locator.count` resolves the current selector once with strictness disabled,
returns zero without waiting, accepts multiple matches, and carries a bounded
`u32` result through the generated codec, client extension, server handler,
target router, and Chromium backend. It shares the locator operation budget
but never retains a result array or DOM identity. Unit, duplex wire, exact
utility-world routing, and real Chrome 152 tests cover this boundary.

The visibility-query checkpoint adds one strict immediate state command rather
than duplicating two backend paths. `Locator.is_visible` returns false for a
missing element, returns the backend-owned visibility state for one element,
and preserves strict failure for multiple matches. `Locator.is_hidden` is
implemented by the optional client extension as the negation of the same typed
command. Unit, duplex wire, utility-world, malformed-response, navigation, and
real Chrome 152 tests prove this composition.

The element-state checkpoint adds three separately installable command routes:
`Locator.is_checked`, `Locator.is_editable`, and `Locator.is_enabled`.
`Locator.is_disabled` composes over enabled without a fourth backend route.
These getters follow Playwright's distinct timing contract: missing or
detached targets are retried under the one caller deadline, strict violations
and invalid target kinds fail immediately, and an available target returns its
current state without polling for `true`. Selector runtime v6 centralizes
label retargeting and native/applicable ARIA state rules. Unit, generated
codec, duplex wire, exact utility-world, deadline, malformed-response, and
real Chrome 152 tests cover the new boundary.

The locator-wait checkpoint adds a separately installable
`Locator.wait_for` route at method `131080` and root tags 15/16. The public
state vocabulary is attached, detached, visible, and hidden, with visible as
the semantic default. Every attempt strictly re-resolves the Locator; zero
matches immediately satisfy hidden or detached, while attached and visible
retry under the runtime-owned caller deadline. Strictness, invalid selectors,
and malformed responses are terminal. Success returns unit and retains no
ElementHandle. Generated-codec, client, server, Chromium routing, bounded
deadline, malformed-response, and real Chrome 152 tests cover the route, and
the backend exposes a separate `wait_evaluations` metric without adding a
second operation lease.

The text-content checkpoint adds a separately installable
`Locator.text_content` route at method `131081` and root tags 17/18. It waits
for one strict connected target under the runtime-owned caller deadline, then
copies the exact nullable DOM `textContent` without normalization. Schema,
codec, and backend limits cap the result at 1 MiB of UTF-8; invalid selectors,
strictness, malformed values, and oversized results are terminal. The client
extension depends only on this command service, and unit, duplex wire,
utility-world, deadline, malformed-response, resource-boundary, and real
Chrome 152 tests prove the route without retaining a DOM handle.

The inner-text checkpoint adds a separately installable `Locator.inner_text`
route at method `131082` and root tags 19/20. It strictly waits for one
connected target, requires the HTML namespace, and copies the browser-rendered
non-null `HTMLElement.innerText`; this deliberately differs from raw nullable
`Node.textContent`. Schema, codec, and backend limits cap the UTF-8 result at
1 MiB. Typed strictness, invalid-target, malformed-result, oversized-result,
deadline, duplex wire, utility-world, resource-boundary, and real Chrome 152
proofs cover the route without retaining an ElementHandle.

The inner-HTML checkpoint adds a separately installable `Locator.inner_html`
route at method `131083` and root tags 21/22. It strictly waits for one
connected element and copies the browser's non-null serialized
`Element.innerHTML`, including comments, escaping, and SVG child markup;
unlike `inner_text`, it has no HTMLElement namespace restriction. The result
is bounded to 1 MiB, shares the existing content-read operation peak rather
than adding it, and is covered by typed, codec, duplex wire, deadline,
utility-world, strictness, malformed/oversized-result, and real Chrome 152
tests without retaining an ElementHandle.

The get-attribute checkpoint adds a separately installable
`Locator.get_attribute` route at method `131084` and root tags 23/24. It
strictly waits for one connected element and calls the browser's
`Element.getAttribute(name)` without an HTMLElement namespace restriction;
present values are exact strings, missing values remain `None`, and SVG is a
valid target. Attribute names are bounded to 64 KiB and results to 1 MiB. The
input, generated script, and result participate in mutually exclusive peak
branches rather than being summed with fill and content reads. Typed, codec,
duplex wire, deadline, utility-world, strictness, malformed/oversized-result,
pre-CDP oversized-name, and real Chrome 152 proofs cover the route without
retaining an ElementHandle.

The input-value checkpoint adds a separately installable `Locator.input_value`
route at method `131085` and root tags 25/26. It strictly waits and re-resolves,
uses selector runtime v6's shared follow-label retargeting, accepts only
`input`, `textarea`, and `select`, and returns the current non-null value
without normalizing it. Non-control targets remain typed failures. The result
is bounded to 1 MiB and shares the mutually exclusive locator-result peak.
Typed, codec, duplex wire, deadline, utility-world, strictness,
malformed/oversized-result, and real Chrome 152 proofs cover label, input,
textarea, and select behavior without retaining an ElementHandle.

The all-text-contents checkpoint adds a separately installable
`Locator.all_text_contents` route at method `131086` and root tags 27/28. It
follows Playwright's immediate `$$eval` snapshot rather than the strict getter
retry loop: zero matches return an empty vector, multiple matches are expected,
and each value is exact `textContent || ''` in selector order. Schema and codec
cap the collection at 4096 items; codec and backend additionally cap each UTF-8
item at 1 MiB and the aggregate at 4 MiB. Typed, codec, duplex wire,
utility-world, navigation replacement, malformed/oversized-result, resource,
and real Chrome 152 proofs cover the handle-free route.

The all-inner-texts checkpoint adds a separately installable
`Locator.all_inner_texts` route at method `131087` and root tags 29/30. Like
Playwright it performs one immediate, non-strict `$$eval`-shaped snapshot and
returns rendered `HTMLElement.innerText` values in selector order. Unlike the
unchecked upstream TypeScript cast, Pwright rejects SVG or any other
non-`HTMLElement` match as typed `InvalidTarget`, preserving the declared
`Vec<String>` result. It shares the reviewed 4096-item, 1 MiB-per-item, and
4 MiB aggregate limits with raw text collections, but production accounting
takes the maximum mutually exclusive collection branch rather than summing
both APIs. Typed, codec, duplex wire, cancellation, utility-world, navigation
replacement, malformed/oversized-result, resource, and real Chrome 152 proofs
cover the handle-free route.

The locator client-composition checkpoint adds Playwright-shaped `clear()` and
`all()` without advancing the locator protocol or adding backend commands.
Clear is exactly one existing Fill call with an empty value. All is exactly one
Count snapshot followed by local `nth(i)` construction; its returned Rust
`LocatorList` keeps one base selector and materializes locators on demand
instead of hiding an N-fold selector-AST allocation. It consumes the source
Locator to move that base; callers clone explicitly only when they need both.
Counts above the signed nth-index domain fail as `ResourceExhausted` before
list construction. Unit tests prove the minimal service bounds and exact
dispatches; Chrome 152 proves clear's Fill behavior and that an nth Locator
obtained before navigation resolves lazily in the replacement document.

The hover checkpoint advances locator protocol to 1.17 with method `131088`
and root tags 31/32. The public action keeps selector resolution lazy and adds
force, auto/no scrolling, a canonical allocation-free modifier set, an optional
finite fractional point relative to the padding box, and trial. The backend
waits for strict visible/stable geometry and an unobscured hit point without an
enabled check, then sends exactly one mouse move. `ControlOrMeta` is resolved
from the browser platform and trial dispatches no mouse input, while any
explicit modifiers are still pressed and restored. Generated-model,
binary-codec, typed-client, duplex-wire, backend, malformed-response, and real
Chrome 152 proofs cover the route. The optional modifier wrapper preserves
omitted versus explicit-empty semantics. Chromium uses a fixed-size Page owner
shared by main, local-frame, and OOPIF routes to serialize key transitions,
mouse dispatch, restoration, and cancellation cleanup. General keyboard,
mouse, and touch capabilities can reuse this owner, but remain separate future
capabilities.

The focus-management checkpoint advances locator protocol to 1.18 with Focus
method `131089`, Blur method `131090`, and root tags 33 through 36. Both are
strict lazy actions with selector-only parameters and generic runtime-owned
timeout/cancellation metadata. Chromium retries only missing or disconnected
targets, invokes native `focus()` twice or `blur()` once, and performs no
visibility, stability, enabled, scroll, keyboard, or pointer work. The codec
rejects both commands when 1.17 was negotiated. Unit and dedicated-profile
Chrome proofs cover typed dispatch, strict violations, focus/blur events,
active-element transitions, non-focusable success, already-blurred success,
temporary-target cleanup, and preservation of the externally owned browser.

The cross-major parity checkpoint freezes locator 2.0, advances the independent
2.x track to 2.1, and appends those same Focus and Blur IDs, tags, parameters,
and semantics without changing its extensible Click record. The exact selected
capability minor now reaches `LocatorV2Codec`: 2.0 rejects Focus/Blur before
encoding or backend work, while 2.1 enables both. The local protocol also keeps
its outer frame version independent from the Locator capability version.

The Press checkpoint is specified in
`docs/knowledge/pwright-locator-press.md` and advances both tracks together to
1.19 and 2.2 with one Press route, a bounded Playwright key-expression grammar,
full static US-layout resolution, rich CDP key events, and fixed-capacity
Page-owned pressed-key cleanup. It reuses the existing navigation barrier and
adds no Node.js or browser-distribution dependency. Generated compatibility,
typed client/server streams, the Chromium target router, selection-reset focus,
reverse-order failure/cancellation cleanup, and dual-major production dispatch
are implemented. Dedicated-profile Chrome 152 now also proves
selection-sensitive Arrow input, printable/shifted text, the platform-smart
select-all shortcut, Enter/form navigation waiting, and controlled cancellation
with reverse key release while preserving the external browser. This closes the
Press checkpoint's designed live gate.

The SetChecked checkpoint is specified in
`docs/knowledge/pwright-locator-set-checked.md` and advances the same tracks to
1.20 and 2.3. It adds one desired-state primitive with client-composed Check
and Uncheck, bounded extensible options in both majors, Playwright-compatible
idempotence and radio behavior, and post-click verification. Generated
compatibility, old-minor rejection, typed remote streams, selected-major
production dispatch, shared Page pointer cleanup, and dedicated-profile Chrome
coverage are implemented without Node.js or browser-distribution ownership.

The SelectOption checkpoint is specified in
`docs/knowledge/pwright-locator-select-option.md` and advances the tracks to
1.21 and 2.4. One bounded command accepts strings or conjunctive
value/label/index descriptors, preserves empty-list deselection and DOM-order
results, retries every pre-commit actionability and option-availability state,
and fires Playwright-compatible input/change events after one atomic mutation.
Locator stays independently installable: ElementHandle selection remains a
separate future capability-composition decision until Handle adoption across
execution worlds has a bounded lifecycle contract.

The ScrollIntoViewIfNeeded checkpoint is specified in
`docs/knowledge/pwright-locator-scroll-into-view.md` and advances the tracks to
1.22 and 2.5. The command remains Locator-only and strict, waits for stable
geometry but not visibility, enabled/editable state, or hit-target reception,
and avoids repositioning at full intersection. Missing, detached, layout-less,
and unstable targets retry before the one non-replayable scroll commit. Frozen
1.21/2.4 rejection, cross-major bytes, typed remote streams, selected-major
dispatch, bounded cancellation, and dedicated-profile Chrome geometry are its
acceptance gates. Browser.version was deliberately not substituted here: it is
cached browser initializer data in Playwright and awaits a bounded initializer
extension/publication design rather than a misleading per-call RPC.

The control-schema checkpoint adds a third schema identity: a non-installable
`protocol` family has a tagged-union root but cannot declare capability
commands or dependencies. Control 1.0 now owns its five exact root tags,
positional fields, strict booleans, cancellation-reason enum, baseline, catalog,
and generated reference in YAML. Production encoding and decoding use the
generated model; hand-written code is limited to conversion into semantic core
IDs and the cross-field canonical absent-parent rule. Existing control golden
bytes and every typed malformed-payload error remain stable.

The external-count checkpoint resolves the handshake grammar decision.
ClientHello writes the required and optional capability counts as one header
before either variable-length capability segment, whereas an ordinary repeated
field writes its count immediately before its own items. The new `counted_by`
relationship is legal only on a repeated field referencing an earlier required
`u32`; it is validated, fingerprinted, compatibility-checked, documented, and
used by both owned and borrowed generated codecs without emitting an adjacent
count. `pwright-codec-handshake` now has a canonical 1.0 YAML source, baseline,
catalog, generated reference, and production generated codec. Borrowed adapters
encode canonical capability maps and slices without an intermediate owned wire
tree. Generated decode is followed only by semantic ID/order, version-policy,
combined-count, and set-uniqueness checks. Legacy minimum-only decoding and
both empty and populated golden layouts remain exact.

The semantic-default checkpoint makes enum defaults part of the reviewed
source rather than a Rust-only extension. The generator rejects a missing
default variant, derives `Default` on both public and wire enums, marks the
selected variant, and rejects default changes within a compatible minor
release. Navigation now declares `wait_until: load` in its schema and has no
hand-written `WaitUntil::default` implementation.

The typed real-Chrome checkpoint upgrades the live navigation test to cover the
attach-only CDP provider, real temporary incognito context and target, matching
runtime Browser/Context/Page graph, typed client handle initialization,
`PageNavigationExt`, direct dispatch, and the CDP navigation backend in one
path. It passed against the dedicated Chrome 152 CDP profile. The test disposes
only its temporary context and confirms `Browser.getVersion` still succeeds
after the typed client is dropped, preserving external browser ownership.

The composable-driver checkpoint adds synchronous `FrameHandler` routing to
`pwright-client-wire`. Deterministic `or_else` chains identify which optional
subsystem handled a frame and preserve the original frame if none did. The new
opt-in `pwright-client-control` handler decodes only control frames, forwards
Create/Adopt as typed lifecycle events, applies idempotent Dispose invalidation,
rejects Cancel/SetSubscription in the wrong direction, and leaves other routes
untouched. Neither layer starts executor work.

The runtime-lifecycle-emission checkpoint adds an opt-in bounded control outbox
to both dispatchers. Create, changed Adopt, and child-first Dispose mutations
preflight their required capacity while holding the object-state lock. A full
outbox therefore rejects before changing the graph or accepting controlled
parent-close cancellation. Exclusive rollback-on-drop claims keep encoding
failures from losing the oldest message. The new `pwright-runtime-control`
sidecar sends one validated Control frame per caller-driven `send_once` and
acknowledges it only after transport success; it brings no executor or socket.

The typed-event checkpoint adds opt-in per-event handlers and queues. A handler
decodes only its registered `EventSpec`, validates the route source, and leaves
unrelated frames available for composition. Queue overflow returns either the
rejected reliable event or the dropped oldest telemetry event. A separate
bounded subscription tracker emits SetSubscription only on first-listener and
last-listener edges and forgets disposed-object listener state without invalid
unsubscribe traffic.

The bounded-stream checkpoint adds a canonical binary WireFrame envelope in a
standard-library-only crate. It rejects malformed routes, truncation, trailing
bytes, and every oversized declared length before body copying. A separate
optional Tokio adapter length-prefixes envelopes, checks the outer length before
allocation, serializes writers independently from its single reader, recognizes
clean frame-boundary EOF, and explicitly shuts down only caller-owned I/O.

The bidirectional-control checkpoint adds a canonical Serde-free codec for all
lifecycle, cancellation, and subscription controls. The runtime receiver
accepts only caller/deadline cancellation and bounded live-object subscription
changes; it rejects runtime-owned lifecycle messages in the client direction.
Duplex tests now cover runtime-to-client Create and client-to-runtime
SetSubscription across the binary stream and optional Tokio transport.

The typed-server checkpoint adds a unique-reader server driver and statically
composable async frame handlers. Each typed command handler validates outer and
decoded identities, dispatches through the runtime Service boundary, verifies
result correlation and route, and sends exactly one response. Runtime control
implements the same handler contract, allowing one inbound chain. A Tokio
duplex test proves typed Remote dispatch through client wire, server wire,
DirectDispatcher, and back to typed result decoding.

The handshake-first checkpoint wraps that entire inbound chain with a strict
connection state machine. Application frames before negotiation, duplicate
hello frames, and post-negotiation frames using any version other than the
selected one are terminal handler failures. The initial outer frame version is
deliberately non-authoritative; bounded `ClientHello` contents drive
negotiation, then the server reply and all later traffic use the selected
version. Both the initial and established receive paths apply the server's
advertised frame limits before delegation.

The concrete-hello checkpoint adds a separate Serde-free binary codec for both
handshake directions. Counts and capability ID lengths are checked before
allocation, capability ordering is canonical, and malformed versions, IDs,
UTF-8, ordering, routes, attachments, truncation, and trailing data fail
explicitly. A server integration test decodes the canonical ClientHello,
negotiates, and decodes the emitted canonical ServerHello.

The Playwright-goto checkpoint adds CDP-session-filtered commit,
DOMContentLoaded, load, and 500 ms whole-page network-idle waiting. It returns
the HTTP(S) main-resource response snapshot, preserves HTTP error statuses as
successful navigations, returns null for same-document and data-style
navigations, forwards referrer, detects replacement loaders, and maps event
stream lag/closure explicitly. Its dedicated real-Chrome test passed through
typed Page dispatch and verified context cleanup plus external-browser
survival. Cross-browser adapters and lifecycle-managed Response objects remain
open parity work.

The navigation-wire checkpoint adds `pwright-codec-navigation` as a separate
optional crate. It canonically encodes both the low-level navigation primitive
and Playwright-shaped goto commands/results without Serde, validates bounded
strings and response headers before allocation, rejects malformed tags,
truncation, trailing bytes, attachments, and noncanonical header order, and
preserves all operation failure kinds and navigation error details. A Tokio
duplex contract test proves a typed `PageGotoExt::goto` request and response
through client wire, stream framing, server wire, and bounded runtime dispatch.
Its normal dependency tree contains only the navigation capability, core, and
transport crates.

The first locator-action checkpoint adds core-only `pwright-cap-locator` and
the independent `pwright-client-locator` extension. A Locator remains a lazy
value and sends its selector expression with each typed click; the Chromium adapter
re-resolves every retry and checks strictness, attachment, visibility,
two-animation-frame stability, enabled state, viewport intersection, and hit
target before dispatching a mouse move followed by a balanced press/release.
The whole sequence now uses the same fixed-size Page input transaction as
Hover, including local-frame and OOPIF routes. A dropped transaction retaining
a pressed button is released by controlled cancellation or repaired before the
next Page input transaction; a failed release is retried during cleanup before
the original error returns. `force`, `trial`, mouse button, and nonzero click
count are explicit portable options. A controlled-runtime
test proves that one `CallMetadata` deadline terminates the retry loop without
input, and a dedicated real-Chrome test proved typed client dispatch, an actual
DOM click, context cleanup, and survival of the externally owned browser.
The selector expression now supports validated CSS plus first-class text,
label, and typed ARIA-role leaves; positive/negative `nth`; normalized
exact/substring text filters; element-identity intersection; deduplicated
ordered union; and `Within` descendant chaining. Page, Frame, and Locator gain
opt-in Playwright-shaped `get_by_*` factories without adding these dependencies
to the base client. Role options cover Playwright's name, description, checked,
disabled, expanded, include-hidden, level, pressed, and selected filters, and
invalid role/state combinations fail before backend dispatch. Both the CDP
compiler and wire decoder bound recursive complexity; the CDP compiler also
bounds aggregate string bytes. A real-Chrome test executes chained dialog and
button roles plus label and text clicks while excluding an out-of-scope
same-name button. Regex matchers, complete W3C accessible-name/implicit-role
conformance, cross-frame chaining, other actions, navigation coupling, and
call-log-rich timeouts remain open.

The following Click-parity audit found a protocol-evolution constraint before
adding more fields. At Playwright commit
`de214f440b7e34937fe4886f046b78b757136087`, public Locator click also accepts
position, modifiers, delay, scroll, and interpolated mouse-move steps, while
navigation waiting remains part of the action. Pwright's existing
`pwright.locator.click_options` is a positional record published since locator
1.0. Such records have no enclosing length or unknown-field representation;
the compatibility checker therefore correctly rejects even appended optional
fields within major 1. This checkpoint does not mutate that layout, falsely
claim 1.18 compatibility, or expose options the backend cannot honor. The next
protocol design must choose between a locator major-version transition to an
extensible tagged option bag and an appended versioned click command that keeps
the legacy route. The decision must also cover unknown-field bounds,
canonical ordering, negotiation, navigation-wait composition, and avoiding a
new command for every later Playwright option.

The bounded extensible-record prototype now removes the wire-format unknown
from that decision. Its envelope is `u32 record_bytes`, followed by fields in
strictly increasing nonzero `u16 tag` order; every field carries its own `u32
value_bytes`. The writer backpatches both lengths in one destination buffer,
and the reader checks the declared record bound before borrowing field slices.
Known fields must consume their exact slice; unknown fields are skipped without
allocation. Schema metadata requires a fixed maximum encoded size and stable
tags. Within one major, an existing tag, field contract, and record-size range
cannot change or disappear; only a new optional tag is compatible. The YAML
generator prototype accepts `encoding: extensible`, a bounded `encoded_size`,
and tagged fields, then generates the codec and catalog metadata. A separately
compiled generated option bag round-trips while skipping an injected unknown
tag. This establishes a viable primitive but deliberately does not yet migrate
the published locator 1.x Click record or choose its new major-version route.

The major-transition decision is now narrowed: locator keeps the single
logical ID `pwright.locator`, while an installation may offer separate 1.x and
2.x tracks. Each client request and accepted connection selects exactly one
track. This avoids both replacing old clients and permanently accumulating
version-suffixed Click RPCs. The transport manifest now keys support by
capability ID plus major, rejects duplicate tracks, negotiates only the
requested major, and still rejects multiple selected majors in one server
hello. Release tests cover 1.x selection when 2.x is also installed.

The Chromium production composition now closes this publication gate. An
opt-in locator profile advertises separate 1.22 and 2.5 support ranges assembled
from separate generated inventories. After handshake acceptance it resolves
the selected locator major and constructs only that major's codec and handler
tree before any application frame is decoded. Unknown majors fail closed, and
the legacy v1-only builder returns no handler for a v2 connection. Removing the
v2 opt-in restores the original one-track manifest and leaves resource
requirements unchanged. A production round trip proves that accepted v2
Click, Focus, Blur, Press, SetChecked, SelectOption, and ScrollIntoViewIfNeeded commands pass through
the selected outer handler, runtime object validation, Page routing, and the
CDP backend.
The canonical all-capabilities installer now consumes that selected-major
handler instead of the legacy v1-only builder, and its external-Chrome
conformance recipe is wired for an opt-in locator 2.x handshake.

The frozen locator 2.0/2.1/2.2/2.3/2.4 wire artifacts and current 2.5 artifact remain separate
from locator 1.x. Their bounded extensible Click record uses optional
tags 1 through 10 for the Playwright options button, click count, delay, force,
modifiers, `no_wait_after`, position, scroll, steps, and trial; timeout and
cancellation continue through call metadata. A zero-field record encodes the
default option set in exactly four bytes. Release fixtures round-trip the full
record, skip an injected future tag, validate the 128-byte body bound, and
assert the v1/v2 versions and positional/extensible kinds separately. Steps
are additionally capped at 10,000 so one small request cannot create
unbounded sequential CDP work.

The public v2 mapping preserves absent values separately from explicit false,
zero delay, and empty modifiers. `LocatorV2Codec`, generated v2 server
bindings, and the same-named client `click` method round-trip all ten fields
through the real stream. The CDP adapter now covers actionability, position,
scroll, force, temporary modifiers, trial, delay, correct multi-click pairs,
bounded interpolated steps, cancellation cleanup, and requested main-frame
navigation commit waiting. Generated multi-command server tables are balanced
binary trees rather than left-deep chains and split capabilities above sixteen
commands into two typed groups; this preserves routing order and keeps the
current dual-major production build tractable.

The SetChecked checkpoint adds command 20 on both tracks without duplicating
Check and Uncheck routes. Their client extensions fix the desired boolean and
dispatch the one typed primitive. Both majors encode the same bounded
four-field extensible option record, while frozen locator 1.19 and 2.2 peers
reject it before payload creation. Chromium performs strict follow-label state
inspection, idempotent no-op, radio protection, shared Page pointer input,
trial actionability, and a terminal post-click assertion without replaying an
acknowledged click. The detailed source evidence, state machine, resource
boundary, and remaining acceptance gates are recorded in
`docs/knowledge/pwright-locator-set-checked.md`.

The SelectOption checkpoint adds command 21 on both tracks. Its request is
bounded to 1,024 descriptors, 64 KiB per string, and 1 MiB aggregate string
bytes; its result has the same count, item, and aggregate limits. The codec
rejects old 1.20/2.3 peers before payload creation, and the Chromium adapter
commits only after all requested options are found and enabled. It uses no
pointer input, navigation barrier, DOM-handle retention, or mandatory Handle
capability. The cross-world ElementHandle overload remains explicitly open.

The selector-runtime lifecycle checkpoint removes the full semantic runtime
from the actionability retry script. One shared `OnceCell` lazily installs it in
the current JavaScript execution context, concurrent cloned services share the
installation, and retries use a compact script. A document replacement returns
an explicit missing-runtime state and triggers an idempotent reinstall without
registering `Page.addScriptToEvaluateOnNewDocument` on an externally owned
browser. Three fixed counters expose successful installs, compact evaluations,
and post-navigation misses without retaining call history. Unit tests cover
concurrent first use, malformed installation acknowledgement, compact retries,
and simulated context replacement; the real-Chrome test now navigates between
the role click and the label/text clicks using the same service.

The full-recipe concurrency checkpoint exposed a browser-scheduling edge that
single foreground-Page tests missed: Chrome suspends `requestAnimationFrame` in
background Pages, so an unconditional two-frame stability wait can retain a
Locator command indefinitely. Pointer and scroll actionability scripts now use
the two-frame check only for visible documents and a zero-delay timer yield for
hidden documents. The same proof also aligns the full profile's supervisor
in-flight limit with its four runtime active-command slots, then overlaps two
Locator clicks and two slow navigations without lag or external-browser
ownership leakage.

The first non-click locator-action checkpoint adds the typed `Fill` command,
`FillOptions`, and opt-in `LocatorFillExt` without changing generic Locator
storage. Its CDP state machine re-resolves on every retry, retargets a label to
its associated control, waits for visible/enabled/editable state, uses CDP text
insertion for text-like controls and contenteditable elements, and uses native
constrained-value semantics plus `input`/`change` for date-like inputs. Empty
fill is selection plus Delete. `force` bypasses actionability checks but not
target/type validation. Invalid targets and values are distinct portable
errors; direct fill values are bounded at 1 MiB. Unit tests cover retry, input
dispatch, clearing, constrained fill, typed terminal errors, metrics, and
resource bounds. The dedicated real-Chrome test verifies readonly auto-waiting,
label retargeting, contenteditable fill, date fill/events, cleanup, and survival
of the externally owned browser.

The locator-wire checkpoint adds `pwright-codec-locator` without making it a
dependency of the capability, typed client extension, or in-process backend.
Its canonical bounded payload preserves click/fill options and values, count
and visibility results, execution metadata, success, every failure category,
and typed locator error details. The selector and command vocabulary advances
this independently negotiated capability through 1.7 without changing the common
frame protocol. Malformed routes, attachments, strings, selector values, ARIA
roles/state combinations, recursive depth/node counts, durations, enum/option
tags, click counts, truncation, and trailing bytes fail explicitly. Tokio
duplex tests prove the complete typed remote click, count, fill, and
client-composed hidden paths,
while a tampered-identity test proves the server rejects inner/outer call-ID
disagreement before backend dispatch. The codec adds only the capability,
shared common/binary codecs, core, and transport as normal dependencies. The
capability itself still depends only on core, and the optional generated
locator server selects no runtime, backend, CDP, browser provider, Node.js, or
browser download.

The first explicit Chromium production root now lives in
`pwright-composition-chromium`. Its preparation phase selects navigation,
locator, or both; derives one additive concurrent resource plan; filters the backend's
potential feature set to the capabilities actually installed; constructs the
wire capability inventory; and provides plan-checked factories for capability
protocols, the complete-topology runtime, and the handshake-first supervisor.
Only the separate `attach` method receives an endpoint or performs I/O, and it
uses `ProductionCdpConnector`; the composition has no Node.js, browser install,
discovery, launch, or process-termination dependency.
The reusable `ChromiumAttachFactory` holds only that immutable recipe and
per-connection limits. It creates a new ledger for every preparation and never
clones a prepared or attached object graph.

This checkpoint also fixes a prerequisite routing flaw: runtime dispatch used
to validate the target `ObjectId` and then discard it before backend service
invocation. `TargetedService` now carries that identity through both direct and
controlled dispatch while an automatic compatibility implementation preserves
ordinary `Service` users. A backend descriptor can now be strictly validated
as a complete declaration or filtered against an installed capability
registry. Tests prove navigation-only advertisement, omitted locator resource
rejection, exact ledger identity on encoded frames, initial runtime/supervisor
state, target forwarding, and preparation without an endpoint.

The page-routing checkpoint makes that root execute against multiple concrete
CDP sessions. A bounded `CdpPageServices` table maps each materialized portable
Page `ObjectId` to the exact target/session pair and removes detached or
reconnecting routes. Navigation and locator service factories share each
profile's concurrency counter across every bound Page while locator runtime
installation remains target-local. The route profile accounts the two native
ID pairs retained by the routing entry and `CdpSession`; the composition plan
uses saturating additive combination because connection, topology, routes,
runtime, backend calls, and wire staging may all be live together.

Both direct and caller-policy controlled page runtimes are available. The
controlled path composes deadline and bounded-cleanup plugins and forwards
command cancellation to the Page's session. `synchronize_topology_update`
applies emitted Create/Dispose ownership to the runtime and refreshes routes
from the same committed topology snapshot; normal callers no longer need to
maintain those two views independently. Tests cover exact-session routing,
unchanged-route reuse, navigation cleanup routing, detached-route rejection,
runtime subtree disposal, and cross-Page local-concurrency enforcement.

The topology transaction checkpoint turns that ordered bridge into a
rollback-capable composition boundary. `ProductionCdpTopology` exposes a
resource-accounted prepared shadow that publishes only on `commit` and releases
its candidate lease on drop. Runtime Create/Dispose batches now preflight the
complete object graph, semantic parent kinds, pending-call policy, lifecycle
outbox, object leases, and cancellation signals before replacing live state.
The route table similarly prepares its bounded replacement while excluding
command lookups. `transact_topology_update` composes those pieces so every
fallible step precedes the runtime commit and only infallible moves follow it.
AAA tests prove prepared-topology rollback, runtime batch rollback, successful
three-state publication, and composition-level rejection without a partial
Page, main Frame, or route.

The selected-handler composition checkpoint adds plan-checked navigation and
locator handler factories to the Chromium root. Each returns `None` for an
omitted capability and otherwise binds both generated commands to the caller's
shared runtime using a protocol from the same ledger. The handler crates add no
runtime, provider, browser, Node.js, or download choice. `Arc<runtime>` now also
implements the lifecycle control-source boundary, so the command handlers,
client-control receiver, and outbound control driver can share exactly one
runtime instance.

The remote-server resource checkpoint now charges every command future retained
by `EncodedServerSupervisor` as a call-scoped `PendingCall`; the composition
adds that lease to the runtime's lease for the same call instead of taking the
old maximum. Duplicate active call IDs therefore fail at the resource boundary,
and every terminal path drops the supervisor lease. An optional remote
handshake profile adds the transport declaration and server-hello encoding
peak only when selected. It caps the peer's advertised receive frame size
before authorization or response allocation and emits a frame owned by the
same ledger. An optional control profile similarly accounts its outbound
protocol plus every retained `(ObjectId, EventTag)` subscription route. Remote
control requires the handshake component and both share one peer frame bound.

The caller-driven remote-loop checkpoint adds a generic outbound lane to the
unique-reader supervisor. Receive, active commands, and outbound lifecycle
sends rotate priority after every completion; commands retain their existing
round-robin order. An idle outbound lane is parked on an optional caller-driven
readiness future, so it cannot create a ready-loop, while an outbound error is a
distinct terminal reason. Sources without readiness notification retain the
connection-progress fallback. `RuntimeControlOutbound` adapts the runtime lifecycle source without a
task, listener, executor, or socket choice. The Chromium root can now build
inbound control and outbound lifecycle halves from the accepted handshake, one
shared runtime, transport, and production ledger. Navigation and locator also
have accepted-handshake handler factories; preparation rejects any installed
capability whose exact version, inbound limits, or peer frame ceiling diverges
from the remote handshake contract.

The typed server-event checkpoint adds the missing producer boundary as the
independent `pwright-server-events` crate. One channel installs one static
`EventSpec`; its cloneable producer retains typed payloads in an explicitly
bounded queue and its single outbound consumer performs codec validation and
transport I/O. `RejectNewest` and `DropOldest` remain observable outcomes. A
temporarily claimed oldest event continues to occupy logical queue capacity,
so lossy overflow cannot displace an event already being encoded or sent.
Encoding or transport failure, and cancellation before completion, restores
the claim at the front for ownership and accounting; only a successful send
releases its `EventItem` and `EventBytes` leases. Because a generic transport
may already own a partial write, cancellation or transport failure is terminal
for that connection and does not authorize retry there. The codec's emitted
route must exactly match the typed source and event tag.

`ServerEventProductionProfile` adds the complete retained queue to the maximum
typed-source/encoded-frame overlap. Production construction rejects a protocol
whose bound differs from the declared profile and rejects a transport backed
by another ledger. Static outbound chains alternate successful plugin lanes;
they never start the next lane while the selected lane has an active send.
Chromium composition accepts zero or more explicit
`ChromiumRemoteEventProfile` installations. Their tags must be unique, they
require the remote handshake, their declared wire contract must match it, and
the accepted-handshake factory returns `None` for an omitted event type.

The BrowserContext-creation checkpoint implements the approved no-options
slice without making it a mandatory dependency. Separate capability, codec,
client-extension, generated-server, CDP-backend, and Chromium-composition
layers implement `Browser::new_context`; the capability declares that context
lifecycle must also be installed. The runtime now has rollback-on-drop prepared
creation and the connection owns a nonzero, non-wrapping monotonic ObjectId
source. Any failure after native creation performs exact-context compensation.
The remote ordered writer physically sends Create before Result, and the
client's bounded announced/claimed state rejects a missing, mismatched,
duplicate, or already-disposed initializer. Saturated continuation capacity
rejects before CDP. A dedicated Chrome 152 proof created a real context and
target, closed them through the portable lifecycle service, and verified the
external browser survived. This is evidence toward DT-020, DT-021, DT-030,
DT-032, DT-033, DT-040, DT-041, DT-043, DT-052, DT-060, DT-061, and DT-070;
options, other backends, and broader API parity remain open.

The post-handshake composition checkpoint now makes the fresh remote unit a
staged state machine rather than a bag of helpers. Handshake acceptance consumes
the runtime-installed connection and advances only its own supervisor. A
one-shot application installer receives an owned, non-constructible context
containing that accepted handshake, exact transport, attachment, runtime, and
optional unique topology-event driver. Selected navigation, evaluation,
locator, Context/Page creation, and Context/Page lifecycle factories are
available on the attached root, while typed event sender/outbound pairs remain
independent plugins. A selected topology driver must be transferred out of the
context before installation can succeed. The root always places native CDP
liveness before the caller backend, and the final application `run` always uses
the complete supervisor path that shuts down its exact transport. Dedicated
Chrome 152 checks prove same-supervisor handshake acceptance, negotiated
navigation-handler construction, topology-driver consumption/rejection,
mandatory liveness typing, and one exact transport shutdown on EOF. This is
evidence toward DT-021, DT-030, DT-032, DT-040, DT-041, DT-043, DT-051,
DT-052, DT-060, DT-061, and DT-070.

The first concrete capability-subset recipe now installs an
`Arc<DirectDispatcher<CdpPageServices>>` and only the two negotiated navigation
handlers. `NoServerControl`, `NoServerOutbound`, and `NoServerBackend` make all
three omitted lanes explicit; the composition root still adds mandatory native
CDP liveness. Installation rejects a profile with no navigation capability, a
peer that did not negotiate it, or a selected topology-event driver that this
minimal recipe cannot own correctly. A dedicated Chrome 152 test pre-provisions
one Page, then proves handshake, `BeginNavigation`, typed result decoding,
drain-on-EOF, exact transport shutdown, and external-browser survival in one
connection. This is deliberately a pre-provisioned-Page recipe: automatic Page
discovery requires lifecycle control plus an owned topology backend.

The following automatic-discovery assembly checkpoint removes the
self-reference that previously blocked that larger recipe.
`ChromiumTopologyServerBackend` now owns an `Arc` runtime and a cloned ordered
control writer while retaining exactly one in-flight pump future. The
discovering-navigation runtime installer selects a controlled Page dispatcher
with caller-bounded cleanup capacity. Its application installer requires the
navigation, remote-control, topology-event, and page-discovery components,
consumes the unique topology driver, installs an inert portable-event publisher,
and places the owned topology pump in the final backend lane. Existing tests
still prove retained-turn and terminal-drop semantics; a dedicated Chrome 152
test proves two-capability negotiation, final type assembly, unique-driver
consumption, and external-browser survival. The next live proof found and fixed
an important compositional wake-up gap: an inert portable-event publisher must
still fence every topology update through its final lifecycle `Create`, because
an ordinary outbound lane may already have observed an empty runtime queue.
A ledger-verified full-duplex memory transport now runs a real client handshake
and lifecycle driver beside the complete server supervisor. The client learns
Browser/Context/Page identities only from control frames, constructs a typed
Page, navigates real Chrome, completes `Close`/`CloseAck`, and leaves the
external browser reachable.

The caller-driven endpoint-server checkpoint packages the remaining listener
edge without introducing a service loop. `ChromiumEndpointAcceptor` gives the
pre-bound Unix, Tailnet-PSK, and mTLS endpoints one static one-connection
accept boundary; `AllowedUnixEndpoint` keeps the Unix listener and its allowed
UID together. `ChromiumEndpointServer` combines one such caller-selected
endpoint with the stateless attach factory and an explicit external CDP URL.
Each explicit `accept_connection` first accepts and authenticates one peer,
then creates a fresh ledger and CDP attachment for only that peer. It creates
no task, runtime, retry policy, accept loop, or default deadline. A rejection
ordering test proves failed endpoint authentication cannot reach CDP, and a
dedicated Chrome 152 test proves that the packaged Unix listener derives the
real kernel UID, completes the Pwright handshake on the same connection, and
leaves externally owned Chrome reachable afterward. Admission concurrency and
deadlines remain deployment policy.

The complete current-capability application checkpoint adds
`ChromiumAllCapabilitiesRuntimeInstaller` and
`ChromiumAllCapabilitiesApplicationInstaller` as an opt-in recipe above the
same staged connection. It refuses partial installation: Context creation and
lifecycle, Page creation and lifecycle, value evaluation, navigation, locator,
Handle evaluation and lifecycle, screenshot, DOM content, Page events, remote
control, Page discovery, topology events, the explicit Playwright-compatible
Chromium utility world, and all three remote Page-event queues must be selected,
while all twelve application capability IDs plus control must be negotiated.
The runtime installer requires the Handle route table and wraps its dispatcher
in the Handle-aware transport-close owner. The final static type chains every
command handler, activated subscription control, ordered lifecycle output,
three bounded event outbounds, topology discovery, native liveness, and a new
independent `PageLoad` backend. The PageLoad backend owns only another broadcast
cursor, never another CDP reader or task, so it and topology ingestion observe
the same bounded native event stream without stealing events from each other.
Cleanup remains a caller-driven bounded queue, and Page-close polling remains
an injected policy. Unit proof covers the independent cursor and exact profile;
a dedicated Chrome 152 proof assembles all thirteen negotiated protocols and
then reattaches after drop to prove external-browser ownership is unchanged.

The Chromium utility-world checkpoint is an explicit infrastructure profile,
not a hidden consequence of attaching to a browser. For each Page and OOPIF
session it follows Playwright's ordering: enable Page, acquire the complete
frame tree, enable Runtime, register a named new-document world, create that
world for every existing frame, install recursive auto-attach, then resume a
paused target. Names include the portable Page ID instead of using one global
spoofable constant. A fresh frame tree may forgive one creation error only when
it proves that exact frame disappeared; arbitrary CDP failures remain visible.
The router waits for each matching Runtime event before publishing that
Frame's locator route. Main, local-child, and OOPIF Frames bind their exact
session and utility `uniqueContextId`; Page calls delegate to the stable main
Frame. World removal fails closed, and replacement constructs a fresh
selector-runtime cell. Portable Frame evaluation remains in the main world.
Fake-CDP tests cover ordering, the disappearing-frame race, strict failure,
all three Frame host shapes, world replacement, and routing isolation. A
dedicated Chrome 152 test writes a global in the utility world, proves it is
absent from the main world, then creates a button only inside a child iframe
and resolves a trial click through that Frame's portable object ID. It cleans
up the exact target and confirms the externally owned browser remains
reachable.

The exact suite count is reported by the verification command rather than
frozen here. The v2 foundation passes its non-browser release tests plus the
compile-fail contract doctests. The changed v2 crates pass release Clippy with all warnings
denied, and the full workspace passes rustdoc with warnings denied. A full
workspace Clippy run on the current toolchain is separately blocked by the
legacy tonic server's newly reported `result_large_err` lint, including
generated tonic code; this checkpoint does not change that v1 ABI.
`pwright-runtime`, `pwright-transport`, and `pwright-browser-backend` retain only
`pwright-core` as a normal dependency. This is further evidence toward DT-032,
DT-033, DT-050, DT-051, DT-052, DT-060, and DT-061, not completion: complete
cleanup retry/drain supervision, complete generated codecs and cross-language types,
safe concurrency bypass rules,
production drain-duration policy, multi-command transactions, remaining
selector conformance, optional cross-world ElementHandle adoption,
event-class assignments and backend subscription activation, cross-language
payload compatibility fixtures, cross-browser goto backends,
response/request object lifecycles, and
full Playwright navigation conformance remain open. The explicit
real-Chrome tests remain ignored by default unless a caller supplies the
required browser endpoint; the typed locator action test was rerun against
Chrome 152 at this checkpoint and covered click, selector-runtime recovery,
all implemented fill paths, immediate state getters, the complete
missing/attached/visible/hidden/detached wait cycle, and strict retrying
text-content reads with exact whitespace preservation, rendered inner-text
that excludes hidden descendants, typed inner-text rejection of an SVG target,
and exact HTML/SVG inner-HTML serialization.

## Phase 0: establish the evidence

### DT-001: Current-system baseline

- Question: What does today's Pwright actually support, depend on, allocate,
  and expose?
- Deliverable: verified capability, dependency, process, memory, and lifecycle
  inventory of the existing workspace.
- Done when: every current public surface and important operational behavior is
  mapped to its owning crate, and unknowns are explicitly listed.

### DT-002: Playwright reference map

- Question: Which Playwright behaviors are contractual targets, useful design
  references, or intentional non-goals?
- Deliverable: source-linked map covering API objects, channel protocol,
  dispatchers, auto-waiting, selectors, events, errors, browser backends,
  installation, and resource lifecycle.
- Done when: “Playwright-compatible” has separate, testable meanings for API
  shape, observable behavior, protocol compatibility, and browser coverage.

### DT-003: Memory investigation plan

- Question: How will we distinguish Pwright runtime memory from browser memory
  and identify retained state?
- Deliverable: measurement protocol and workloads for idle, navigation,
  repeated page creation, handles, network traffic, tracing, and teardown.
- Done when: measurements can separately report the client, runtime, browser
  parent, renderer, GPU, and utility processes, plus internal object counts.
- Checkpoint: the optional, standard-library-only `pwright-process-metrics`
  crate now reads a bounded process-table snapshot, retains no command lines,
  validates the PID graph, and separately reports one Pwright runtime plus the
  browser parent, renderer, GPU, utility, other-child, and total RSS-sum groups.
  Its opt-in `ledger` feature adds a bounded caller-driven recorder without an
  executor, timer, browser provider, CDP client, or background task. The plan
  fixes workload kind, cold baseline, cleaned warmup observations, steady-state
  observations, sample interval, and teardown deadline. Every observation
  correlates the same runtime/browser PID roots with OS attribution,
  `ResourceMetrics`, and browser object counts; phase order, monotonic time,
  root stability, capacity, completeness, and teardown duration fail closed.
  A release test against dedicated Chrome 152 performed four real portable
  BrowserContext+Page create/close cycles and returned all logical state to its
  cleaned warm baseline. A second driver performs a complete lifecycle warmup,
  three Playwright-shaped `goto(wait_until=load)` operations, and final
  teardown. It exposed and fixed an invalid immediate-post-close measurement:
  the drivers now bound and poll Chrome process settlement, and the report
  independently verifies that temporary browser helpers returned to the warm
  process count. Warmup itself waits for a stable count rather than requiring
  equality with cold startup, because first use may legitimately start a
  persistent helper. The navigation driver now repeats the complete fresh
  attachment/runtime workload three times; a Chrome 152 run held six browser
  processes throughout and observed runtime RSS deltas of
  `+131,072..+1,310,720` bytes and browser RSS-sum deltas of
  `+229,376..+4,947,968` bytes. A third driver installs Handle evaluation/lifecycle
  independently, proves composition rejects topology-plus-Handle capacity that
  exceeds runtime capacity, then correlates three retained ElementHandles
  across route, runtime-object, and `RemoteHandle` ledger boundaries. Explicit
  disposal releases one while Page teardown parent-retires the other two. A
  fourth driver measures the minimal idle external attachment from pre-attach
  cold state through three steady samples and explicit runtime/CDP teardown. It
  proves the sole Browser object and connection leases release, the sticky CDP
  closed signal resolves, and a fresh attachment still reaches the externally
  owned Chrome. A fifth driver installs the single-cursor network component
  before Page creation, performs one complete Context/Page/network/close
  warmup, reuses the exact one-lane capacity, and then runs three data-URL
  network lifecycles on a persistent Page. It samples only after ordered
  lifecycle publication, exact batch acknowledgment, zero native request
  correlation, and an empty control outbox. Explicit Page and Context close
  retire their driver adapters only after local publication; failed Page close
  preserves the live adapter. The first run exposed portable retention directly:
  steady `LiveObject.current` rose `6 -> 8 -> 10` as three completed
  Request/Response pairs remained beneath the Page. The corrected base contract
  now sends complete terminal value snapshots and retires each Request root
  after every selected event queue accepts the batch. A subsequent Chrome 152
  run held the same persistent Page at `[4, 4, 4]`, returned logical state to
  baseline, reduced the browser tree from 10 to 8 processes, and observed
  post-minus-cold RSS deltas of `+1,671,168` runtime bytes and `-231,620,608`
  browser RSS-sum bytes. These are one-run diagnostics, not
  budgets. The bounded series model now requires an identical plan and exact
  run count, rejects any run before logical cleanup and browser-process
  settlement, and reports RSS delta ranges without applying an unreviewed
  threshold. The current driver selects the task-free reclaiming fanout with
  two plugin-derived cursors, places a CDP response barrier after navigation
  and teardown, drains unrelated tail events, and asserts zero queued raw
  events at each boundary. Two consecutive three-run Chrome 152 network series
  kept every persistent Page at `[4, 4, 4]` and returned every resource class,
  including `EventBytes`, exactly to the warm baseline. The process-stable
  second series observed `+98,304..+1,425,408` runtime bytes and
  `+884,736..+5,357,568` browser RSS-sum bytes. Owners, scopes, leases, objects,
  and native correlation remain strict. A later warmup-settlement run held six
  browser processes across all three reports with runtime deltas of
  `+81,920..+1,490,944` bytes and browser deltas of
  `+212,992..+4,849,664` bytes. A separate complete-recipe Chrome 152
  proof now overlaps two slow navigations and two Locator clicks while topology,
  PageLoad, and shared network cursors remain active. Its fanout peak metric
  observed 6 or 7 cursors beneath the safe derived capacity of 10, then returned
  to three current cursors, zero queued events, and zero lag. This validates the
  allocation boundary but is not a tracing or RSS workload. The
  network codec now derives the largest terminal payload and
  retained snapshot size from the same metadata limits, and preparation rejects
  a smaller remote frame or event-byte budget before CDP I/O. Owned-browser
  teardown now has a truthful terminal observation: a non-constructible proof
  comes from one bounded process snapshot containing the runtime PID but not
  the exact owned Browser PID. The same workload recorder accepts that proof
  after ordinary two-root baseline/warmup/steady/pre-teardown samples, checks
  roots, deadline, resources, and objects, and never invents zero browser RSS.
  A bounded three-run series launched distinct Chrome 152 processes; all exited
  gracefully, released every ledger lease, proved their exact PIDs absent, and
  removed their temporary test profiles. Runtime teardown deltas were
  `+32,768..+589,824` bytes and measured pre-teardown browser RSS sums were
  `1,204,158,464..1,208,762,368` bytes. Tracing,
  the remaining non-network benchmark series, Linux
  PSS, and reviewed budgets remain open; see
  `knowledge/pwright-process-memory-metrics.md`. The first network prerequisite
  now exists: a core-only bounded lifecycle model and optional event contract,
  plus a Chromium normalizer that reserves its complete active-correlation
  `BackendStateBytes` ceiling. A Chrome 152 release proof observed one real
  request/response/finished lifecycle with zero retained native correlation
  and no body read. This is implementation evidence, not the DT-003 network
  memory workload by itself. Local production composition now adds a caller-polled Page
  cursor, atomic runtime lifecycle admission, final-Create wire fencing, four
  bounded fail-closed typed queues, Page-parent retirement, and explicit lane
  plus runtime-child capacity. Remote delivery now uses one dedicated canonical
  codec/outbound and one sequence across all four public event tags; generic
  per-event outbounds remain rejected. A one-slot production client handler ran
  4,096 deliveries with one peak lease and zero retained leases, while the
  native tracker ran 4,096 completed exchanges with no retained history. A
  second Chrome 152 proof covers redirect ordering, HTTP 500, and truncated-body
  transport failure. Page-scoped dynamic server activation now requires an
  exact pre-registered cursor, keeps four tags in one bounded mask, filters the
  ordered remote stream, disables only the final network-only owner, and
  preserves a selected navigation owner. One attachment-level caller-polled
  driver now routes one CDP cursor through a bounded exact-session adapter
  table. Empty Context masks are retained independently, Page native ownership
  uses the direct/inherited union, and dual Page/Context publication is Page
  first with shared portable identities. Context first/last edges now run one
  bounded exact-session batch, commit only after all sessions succeed,
  compensate definite failure in reverse, and retain an idempotent inverse
  cleanup plan when the future is cancelled. Explicit
  `BrowserContext.newPage` now registers and, when inherited interest exists,
  enables a provisional exact-session adapter before Page/Frame publication;
  the cursor holds a racing provisional-session event until commit, while
  publication failure disables the domain, retires the lane, and closes the
  target. Cancellation recovery is explicit and idempotent. Automatic new-Page
  discovery now uses the same transaction and detaches its session after
  publication rollback. Session replacement uses one resource-declared shadow
  adapter, preserves direct/Context masks, joins old active-Request cleanup to
  runtime rebind, and has success, rollback, and cancellation-recovery proofs.
  Parent detach retires the lane only after topology/runtime commit. The client
  exports Page and BrowserContext first/last-listener helpers, while the server
  driver owns a receive/publish/exact-acknowledgment turn. Application installer
  assembly and the first process-RSS network workload are implemented. Repeated
  network series across other named hosts, tracing/owned-browser teardown
  series, and reviewed budgets remain open.

  The upstream comparison now identifies one concrete retention candidate in
  the local Playwright `1.63.0-next` checkout: terminal events reuse but do not
  dispose Request/Response dispatchers, while the generic per-type bucket waits
  until 10,000 entries before trimming its oldest 10%. Pwright's base
  `pwright.network-events` capability now removes native correlation, publishes
  a complete terminal Request/optional-Response value snapshot, and then
  disposes the temporary Request root plus Response child. Post-terminal live
  handles are intentionally deferred to a separate capability with hard
  object/byte limits, explicit client acknowledgment, disconnect cleanup, and
  admission backpressure.

## Phase 1: define product boundaries

### DT-010: Users and installation profiles

- Question: Who uses Pwright, and what is the smallest install for each user?
- Deliverable: a short set of profiles such as embedded Rust library, agent
  CLI, remote client, remote runtime, system-Chrome attachment, and optional
  managed browser.
- Done when: every profile lists required and forbidden dependencies, binaries,
  browser ownership, and delivery surfaces.

### DT-011: Compatibility contract

- Question: How closely must Rust Pwright match Playwright?
- Deliverable: compatibility levels for naming, API shape, behavior, errors,
  events, selectors, auto-waiting, protocol, and test reuse.
- Done when: each level is classified as exact, adapted for Rust, deferred, or
  intentionally incompatible.

### DT-012: Scope and non-goals

- Question: What belongs in v2, and what must remain outside it?
- Deliverable: explicit v2 scope, later scope, and non-goals.
- Done when: Chromium-only versus multi-browser, attach versus launch, local
  versus remote, and library versus CLI/server responsibilities are settled.

### DT-013: Success budgets

- Question: What quantitative limits define a successful rewrite?
- Deliverable: budgets for binary size, dependency count, idle RSS, per-page
  overhead, object counts, event queues, startup time, and operation latency.
- Done when: each budget has a workload, measurement method, target, and failure
  response rather than an unmeasurable word such as “lightweight.”

## Phase 2: design the public contract

### DT-020: Rust API object model

- Question: Which Playwright objects exist in Rust, and which are resources,
  values, builders, or views?
- Deliverable: object hierarchy and ownership table for Playwright,
  BrowserType, Browser, BrowserContext, Page, Frame, Locator, handles, network
  objects, workers, downloads, and artifacts.
- Done when: clone, Send/Sync, lifetime, close/dispose, and equality semantics
  are defined for every public object category.
- Checkpoint: `Page.content()` is designed as local delegation to the Page's
  initialized main `Frame`; only `Frame.content` is a wire command, so child and
  OOPIF Frames remain independently addressable without a duplicate Page
  protocol surface.

### DT-021: Rust method conventions

- Question: How should async calls, options, builders, return values, streams,
  callbacks, and errors feel idiomatic without drifting from Playwright?
- Deliverable: representative signatures for navigation, locator actions,
  evaluation, events, contexts, downloads, and shutdown.
- Done when: the conventions handle cancellation and optional capabilities
  consistently and do not require CDP types.

### DT-022: Selector and locator semantics

- Question: What exactly makes a Locator lazy, strict, retryable, composable,
  and compatible with Playwright?
- Deliverable: selector AST, locator composition rules, resolution boundary,
  strictness behavior, and supported engines.
- Done when: CSS, text, role, label, filter, nth, and/or composition have
  unambiguous observable semantics.

### DT-023: Actionability and auto-waiting

- Question: What conditions and retry rules govern click, fill, navigation,
  assertions, and page transitions?
- Deliverable: action state machines and deadline rules.
- Done when: visible, stable, enabled, editable, event-receiving, navigation,
  detachment, retry, and timeout behavior can be tested independently.

## Phase 3: design internal boundaries

### DT-030: Client/runtime boundary

- Question: Is every call logically a channel command, and how does the
  in-process path avoid serialization and duplicated state?
- Deliverable: comparison of direct typed dispatch, an actor boundary, and an
  encoded channel, including costs and remote extensibility.
- Done when: object ownership and call flow are clear in local and remote modes.

### DT-031: Protocol schema and code generation

- Question: What is the source of truth for objects, commands, results, events,
  errors, metadata, and controls?
- Deliverable: evaluated IDL options and a minimal representative schema.
- Done when: Rust types, validators, dispatch tables, documentation, and
  compatibility metadata can be generated without binding the model to JSON.

### DT-032: Object identity and lifecycle protocol

- Question: Which entities require identity across calls, and how are create,
  adopt, dispose, collection, and disconnection represented?
- Deliverable: object-tree invariants and lifecycle state machines.
- Done when: leaked clients, duplicate dispose, parent close, stale handles,
  reconnect, and bounded collection have specified outcomes.

### DT-033: Calls, cancellation, deadlines, and errors

- Question: How does one API operation travel end to end and terminate?
- Deliverable: call state machine and typed error taxonomy.
- Done when: timeout, caller cancellation, target close, browser crash, protocol
  violation, unsupported capability, transport loss, and cleanup failure are
  distinguishable.
- Checkpoint: controlled runtimes expose a generic typed subtree interruption
  for non-disposal lifecycle failures. Chromium Page crash uses it to terminate
  queued and active calls as `BrowserCrashed` while keeping object identities
  live; Frame disposal remains `TargetClosed`, first terminal reason wins, and
  direct runtimes require the affected subtree to be idle.

### DT-034: Events and backpressure

- Question: How are subscriptions activated, queued, delivered, and stopped?
- Deliverable: subscription lifecycle and overflow policies by event class.
- Done when: slow or abandoned consumers cannot produce unbounded memory and
  critical lifecycle events cannot be silently treated like telemetry.

Implementation checkpoint (2026-09-03): the first concrete event slice is
`pwright.page-events/page.load`. Event capability metadata now has the same
stable ownership discipline as commands: semantic `EventId`, numeric wire
`EventTag`, target object kinds, duplicate-owner rejection, and server
inventory preflight. The event-only capability can be negotiated without
navigation or locator. Its fixed-size codec duplicates Page identity inside
the payload and rejects route/payload disagreement. Client installation uses
`RejectNewest`, because a Page lifecycle transition must not silently become a
telemetry window.

The Chromium adapter activates `Page.enable` followed by
`Page.setLifecycleEventsEnabled(true)`, reverses only the lifecycle toggle so
other Page-domain capabilities remain usable, and normalizes only current
main-frame `load` events to the stable portable Page ID. It owns no reader,
queue, or task. The server control path is now an ordered async barrier:
`SetSubscription` reserves capacity, awaits the selected backend activator, and
commits logical subscription state only after success. Failed enable and
disable operations preserve the exact prior route and resource state;
idempotent listener edges do not call the backend. Chromium composition wires
the PageLoad activator to the same synchronized Page/CDP-session route used by
commands, so a later command cannot overtake activation on the wire reader.

Object cleanup is now retryable and caller-driven: successful deactivation
edges release their logical route one at a time, while a failed edge and every
later route remain leased for another attempt. Chromium composition consumes a
committed topology update's Dispose effects; a Page route already retired by
that update makes disable a safe success rather than redirecting work to a new
session. Clean EOF uses a second async supervisor barrier, signals runtime
transport closure first, attempts every backend deactivation, releases every
local route even when cleanup fails, and reports the first plus an additional-
failure count. Both cleanup loops use constant additional memory.

An activation or deactivation awaiting CDP is now represented by one explicit
in-flight transaction. If its future is cancelled, pending enable retains its
reserved lease and pending disable retains the active route. Clean EOF can then
compensate either uncertain edge with disable before releasing ownership. The
supervisor polls each control once before considering later transport input,
preserving stream order. If the handler declares that this pending poll released
the input frame, it may retain exactly one next frame while watching for EOF;
that frame is not dispatched until the control succeeds. EOF instead drops the
control future and runs the async close compensation barrier. Conservative
third-party handlers opt out of prefetch by default.

The same async cleanup barrier now reaches one completed attempt for every explicit supervisor
terminal path, including receive, protocol-validation, command, control,
observer, outbound, command-capacity, and shared-resource failures. The hook
receives the typed close reason. `close_with_controls` is the explicit async
close API for owners that have a control/backend boundary; the older synchronous
`close` remains a logical close only. If the primary operation and cleanup both
fail, `TerminationCleanup` retains both errors instead of replacing the primary
cause. Terminal cleanup events are not delivered through an observer that is
already failing. A dropped run future still cannot await Rust async cleanup;
the next owner call with controls retries an unfinished barrier. The activated
receiver retains the route and its resource lease until each backend disable
future resolves, so cancelling cleanup cannot erase its retry state.

Event ordering is now an explicit portable transport contract. Each installed
typed event channel owns one monotonic `EventSequence`, starts at one, and
assigns the number before queue admission. A server-side reject, drop, or
resource failure therefore consumes a position instead of hiding loss. The
sequence never wraps: exhausting `u64` terminates emission with a typed error.
Backend adapters still return plain semantic `Event<Spec>` values; only the
server channel and codec handle `SequencedEvent<Spec>`, so ordering is neither
CDP-specific nor part of a browser-native payload.

The client accepts any positive first sequence as the connection-epoch
baseline, then distinguishes consecutive delivery from an explicit gap.
Duplicate, regressing, and zero sequences are protocol errors with separate
metrics. Sequence observation precedes local bounded-queue admission, so a
local `RejectNewest` remains visible as local backpressure instead of being
misreported as wire loss. Starting a new connection epoch releases stale queued
events and resets only the baseline; cumulative gap/error counters remain
observable. The fixed-size Page-event codec carries the eight-byte sequence and
source identity; Context-Page and popup variants add one related Page identity.
`pwright.page-events` is version 1.2 for the additive discovery-event surface.
The caller-owned topology publisher forwards only installed lanes and treats
queue reject/eviction as explicit event loss requiring reconstruction.

This completes the DT-034 foundation: subscription edges, backend activation,
bounded delivery, reasoned cleanup, ordering, and loss detection all have
portable contracts and executable tests. Individual future event capabilities
must still classify their own overflow policy and decoded-size limit.

Network-event design checkpoint (2026-09-04): Page network listeners require a
pre-registered caller-polled lane for the exact Page/session before the first
`Network.enable`. Four public tags share one bounded per-Page interest mask and
one ordered remote queue; the publisher filters only that remote queue from the
committed mask, while optional local queues remain independently installed.
The first edge enables the domain before committing, later edges only change
the mask, and failed or cancelled native transitions retain the prior state.
The last edge sends `Network.disable` only when network events are the sole
installed domain owner. A selected navigation plugin is initially a
connection-lifetime shared owner because its current `goto` operation enables
Network without an operation-scoped release. Page retirement forgets the exact
lane and logical mask after control cleanup. BrowserContext fan-out over
current Pages, explicit `newPage` inheritance, and automatic new-Page discovery
is implemented. Existing-Page discovery prepares and enables one separately
budgeted shadow adapter, carries the old direct/Context interests forward, and
cleans active Request roots in the runtime rebind. Failed publication restores
the old lane and disables/detaches the incoming session; cancelled activation
is explicitly recoverable. Page-session detach holds
the network state guard across topology/runtime publication, committing lane
retirement only after the Page subtree is disposed and preserving it on
transaction failure.

BrowserContext network design checkpoint (2026-09-04): Context fan-out uses
one optional caller-polled connection driver with one CDP cursor and a bounded
Page/session normalizer table. The driver, exact attachment identity checks,
explicit retirement, empty Context retention, separate direct/inherited masks,
effective native union, and Page-first dual-scope wire publication with shared
Request/Response identities are implemented. Context activation now snapshots
one bounded exact-session plan before native I/O, commits after every action,
compensates definite failure in reverse, and preserves a cancelled enable for
an exact idempotent disable cleanup edge. Explicit `BrowserContext.newPage`
now prepares and, when needed, enables a provisional adapter before publishing
Page/Frame runtime, topology, or routes. Its non-owning registration handle
does not block the driver cursor; a racing provisional-session event is held
until commit. Publication failure reverses Network ownership and retires the
lane before exact-target cleanup. Automatic new-Page discovery now joins the
same transaction and detaches the auto-attached session after rollback. Session
replacement preserves old logical interest, enables one replacement-only shadow
adapter before publication, disposes old active Request roots in the same
runtime batch, and atomically commits the new exact session. The adapter table
is capped by the network Page-lane limit plus one shadow;
Context masks have an independently configurable limit; an in-flight fan-out
may retain at most one bounded Page/session/action plan. Public Context
listener helpers now target both Page and BrowserContext portable identities.
The owned driver turn performs network-batch publication and exact Page
acknowledgment; one unacknowledged normalized batch blocks the next driver turn,
replacement, and retirement. Parent retirement has atomic success and
failed-publication preservation proofs. A selected network driver must now be
consumed by the application installer; its registration handle joins topology
independent of installer take order. The all-capabilities recipe owns the
negotiated network queue, publisher, outbound, and one retained supervisor
backend turn, and terminal connection notification drops the driver and clears
its exact Page lanes. Unit teardown and dedicated-Chrome
install/drop/external-browser-survival proofs pass. The first network workload
also proves warmup lane reuse, three settled Request/Response lifecycles, zero
native correlation at every steady sample, and Page/Context close adapter
retirement. Its corrected terminal-snapshot contract additionally proves a
stable `[4, 4, 4]` live-object series on the persistent Page. The bounded
three-run series driver now repeats the complete attachment/runtime workload,
requires cleanup before admitting each report, and exposes diagnostic RSS-delta
ranges; cross-host repetition and reviewed budgets remain.
Route preparation continues to reject any active
Context Page whose exact lane was not prepared by a positive participant;
silent event loss is not an interim behavior.

### DT-035: Transport and codec boundaries

- Question: Which semantics belong to the protocol versus in-process, pipe,
  WebSocket, or future transports?
- Deliverable: transport and codec contracts, framing, binary handling, limits,
  authentication boundary, and connection negotiation.
- Done when: adding a transport cannot change Page or Locator behavior.

Implementation checkpoint (2026-09-03): write-half closure is now a separate
opt-in `ShutdownFrameTransport` capability rather than an assumption hidden in
`FrameTransport`. Its contract requires serialization with frame sends,
completion and flushing of any transport-owned partial frame, clean peer EOF
after preceding frames, and cancellation-safe retry. The Tokio adapter and its
production wrapper implement it; `Arc<T>` preserves the capability without a
runtime-specific wrapper.

The unique encoded-client receive driver exposes `close_transport`. It first
closes bounded logical state and wakes/releases pending calls, then enters the
same priority send gate used by commands and cancellation controls before
shutting down the write half. Commands already queued behind that barrier
recheck logical closure after acquiring the gate and never write after EOF.
Cancellation of the close future leaves the channel logically closed and a
later owner call can retry shutdown. The server supervisor exposes both a
plain transport close and a combined control/backend-cleanup plus transport
close. The combined error retains both failures when cleanup and shutdown fail.

Tests prove that shutdown waits behind an active client writer, prevents a
queued command from writing, completes a transport-owned partial Tokio frame
before EOF, preserves the server's prior terminal reason, drains a result before
server EOF, and reports dual close failures.

The second-transport checkpoint adds `pwright-transport-memory`, a
standard-library-only full-duplex `FrameTransport` with an explicit queue bound
per direction. Send and shutdown mutate state only when their caller-owned
future is polled; unpolled sends have no side effect. Overflow, local write
closure, peer receiver loss, and poisoned state are distinct errors. Each
direction retains at most the declared number of already-bounded `WireFrame`
values plus one replacement receive waker. Shutdown drains preceding frames
before EOF, endpoint drop releases queued frames, and no task or executor is
created. The same typed client/channel and typed server handler conformance test
that crosses the transport boundary now passes without a socket or Tokio
transport dependency.

The graceful-close checkpoint upgrades `pwright.control` to schema 1.1 while
retaining the checked-in 1.0 baseline. It reuses the unchanged handshake's
bounded named-version negotiation with a reserved `pwright.control` entry:
absence selects the legacy 1.0 baseline, while only an explicitly returned 1.1
entry unlocks Close/Ack. This avoids a bootstrap-format flag day. A
role-bound `NegotiatedControlCodec` receives that exact selected minor and
independently keeps the outer application version on every WireFrame. It gates
both encoding and decoding by minor and direction, so a 1.0 peer is never sent
tag 5/6 and invalid Client-Ack or Server-Close traffic fails locally.

The new core-only `pwright-connection-control` crate contains allocation-free,
caller-driven client and server machines. Side effects are explicit actions:
Client Close -> Ack wait -> write shutdown; Server Close -> reasoned cleanup ->
Ack -> write shutdown. Client wire keeps its machine on the unique driver,
closes pending logical calls before submitting Close, validates and discards
pre-barrier frames one at a time, and exposes ignored/woken counts. The server
decorator composes over an arbitrary existing control handler and guarantees
cleanup before Ack. Tests cover schema compatibility, independent negotiation,
version and role rejection, exact outer/application version preservation,
legacy failure before local close, a complete memory-transport exchange, and
cancelling/resuming Ack wait without a duplicate Close.

The endpoint checkpoint completes the non-Unix authentication foundation with
two distinct adapters: Tailnet-only mutual PSK for an already encrypted
Tailnet, and general mTLS with caller-owned PKI plus bounded certificate pins.
Neither changes Page/Locator semantics or the generic transport contract.

The production-composition checkpoint installs the reserved support range only
when Chromium remote control is selected. The handshake limit must therefore
hold every installed browser capability plus this one protocol-family entry;
an undersized profile fails during preparation. Accepted connections construct
server-role negotiated codecs for the inbound runtime receiver, independent
outbound lifecycle lane, and composable graceful-close decorator. The control
profile declares a second simultaneous encode peak for CloseAck while an
outbound send is retained. After the ordered Close barrier is observed, the
supervisor now terminates immediately with `PeerGracefulClose`, drops bounded
active command work, and does not wait to reinterpret the exchange as EOF.

## Phase 4: design composition

### DT-040: Capability definition

- Question: What is the smallest independently installable capability, and what
  may it contribute?
- Deliverable: capability anatomy covering public API, protocol messages,
  runtime handlers, backend support, dependencies, tests, and diagnostics.
- Done when: navigation, DOM, input, network, tracing, and browser launching can
  be classified without circular ownership.
- Checkpoint: DOM serialization is assigned to a new read-only
  `pwright.dom_content` capability rather than screenshot or arbitrary
  evaluation. The granularity decision follows independently useful authority,
  backend machinery, and resource cost—not one capability per method and not a
  broad Page grab bag. Its 1.0 schema, generated manifest/server binding,
  attachment codec, typed Page/Frame client facades, Chromium utility-world
  adapter, shared locator/DOM Frame route, direct dispatch, negotiated remote
  handler, all-capabilities recipe, and dedicated-Chrome proof are implemented.

### DT-041: Composition and dependency rules

- Question: Are capabilities composed with Cargo features, generics, a runtime
  registry, generated manifests, process plugins, or a deliberate combination?
- Deliverable: alternatives analysis including compile time, binary size,
  diagnostics, versioning, and downstream ergonomics.
- Done when: missing dependencies, duplicate ownership, incompatible versions,
  and cycles all fail predictably.
- Checkpoint: runtime subset selection now has a matching compile-time proof.
  `pwright-composition-chromium-navigation` depends on the CDP backend with
  default features disabled and only its navigation surface selected, while
  preserving the complete aggregate composition. It binds an injected CDP
  client and pre-provisioned Page to production-budgeted generated handlers;
  no task, executor, transport, browser provider, launcher, installer, Node.js,
  or unrelated capability is selected. An isolated consumer plus a CI Cargo-
  tree allowlist fails closed on any new Pwright dependency. Runtime
  negotiation and Cargo selection are therefore complementary rather than
  substitutes. Other capability leaves and duplicate/incompatible selection
  diagnostics remain open.
- Checkpoint: value evaluation is the second independently selectable Chromium
  server slice. Its adapter was extracted into
  `pwright-backend-cdp-evaluation`; the aggregate backend reexports it without
  becoming a dependency of `pwright-composition-chromium-evaluation`. The leaf
  binds one Frame, distinguishes default and explicit unique execution
  contexts, retains the bounded context-route lease, and installs only the
  generated evaluation handler. Its exact consumer graph excludes navigation,
  Handles, the aggregate composition/backend, clients, providers, installers,
  runtimes, Node.js, and browser acquisition.
- Checkpoint: read-only DOM content is the third independently selectable
  Chromium server slice. `pwright-backend-cdp-dom-content` owns only the fixed
  Frame serializer and exact utility-world route; the aggregate backend
  compatibility-reexports it. The one-Frame composition installs only the
  generated `Frame.content` handler and jointly budgets its persistent route,
  backend result, decoded protocol result, and wire peak. Its exact graph
  excludes public evaluation, screenshot, navigation, aggregate
  backend/composition, clients, providers, installers, and runtime choice.
  Evaluation and DOM content share only the command-free
  `pwright-backend-cdp-common` failure classifier, so shared mechanism grants
  no command authority.
- Checkpoint: Page screenshot is the fourth independently selectable Chromium
  server slice. `pwright-backend-cdp-content` owns the native capture and
  bounded base64 decode, while the aggregate backend compatibility-reexports
  it and Locator screenshot explicitly reuses its opaque admission
  transaction. The one-Page composition validates the exact Page before CDP,
  installs only `Page.screenshot`, and jointly budgets backend result, decoded
  protocol result, and binary wire attachment. Its graph excludes DOM content,
  public evaluation, Locator screenshot, navigation, aggregate
  backend/composition, clients, providers, installers, and runtime choice.
- Checkpoint: Locator screenshot is the fifth independently selectable
  Chromium server slice. Its capture/translation backend accepts a
  command-free `LocatorScreenshotGeometryResolver` plugin. The first-party
  Chromium resolver is now an independent bounded leaf and shares one extracted
  selector compiler/runtime with the complete Locator backend, so the exact
  23-package consumer graph excludes public Locator commands and the aggregate
  CDP backend. The generic capture composition still accepts alternate
  resolvers without linking the first-party implementation.
- Checkpoint: the public `pwright` umbrella now exists with an empty default
  feature set. Its explicit axes separate portable evaluation/navigation
  contracts, Playwright-shaped clients, raw CDP, their minimal Chromium server
  leaves, executor-neutral runtime/transport contracts, and Tokio adapters. It
  only reexports selected crates and contributes no hidden policy or
  implementation. Eleven isolated downstream packages prove that the empty
  profile has no Pwright dependency, content/Locator-screenshot/DOM/evaluation/navigation clients
  have no CDP/server, Chromium content has no client/DOM/evaluation/Locator/
  navigation/aggregate backend, Chromium DOM content has no client/evaluation/
  screenshot/navigation/aggregate backend, Chromium evaluation has no client/
  navigation/aggregate backend, and Chromium navigation has no client/provider/
  installer/aggregate-backend dependency.
  The Chromium Locator-screenshot profile additionally rejects both
  `pwright-cap-locator` and the aggregate CDP backend.
  A default-build compile-fail doctest also proves an omitted capability has no
  public module. A `full` feature is deliberately postponed until the complete
  supported topology is known.

### DT-042: Core versus policy layers

- Question: Which behaviors are invariant automation semantics, and which are
  optional retry, tracing, recording, leasing, rate-limit, or timeout policy?
- Deliverable: responsibility table and composition order rules.
- Done when: optional policy cannot silently alter protocol or object-lifecycle
  invariants.
- Checkpoint: Playwright tracing source was decomposed into a base
  BrowserContext metadata/group/chunk capability and independently selected
  DOM snapshot, ARIA snapshot, screencast, network body, HAR, source, console,
  video, filesystem/ZIP, and Artifact contributors. The base recorder owns no
  I/O or executor, and an injected sink must declare bounded admission. The
  state and resource contract is recorded in
  `knowledge/pwright-tracing.md`. The core-only bounded recorder and its exact
  production profile are implemented in `pwright-trace-model`; a detached
  chunk gates any next chunk/session until release. `pwright-cap-tracing`
  freezes the six BrowserContext-only 1.0 commands and exposes only discard or
  an injected sink, not Artifact/ZIP or contributor flags. The bounded binary
  codec, Playwright-shaped opt-in client facade, and generated six-service
  server chain are implemented and transfer no trace-body attachment. The
  task-free `pwright-tracing-service` explicitly composes one pre-registered
  bounded recorder per Context with an injected monotonic clock and sink, uses
  the same instance for all six generated services, and requires Context/all
  sink cleanup after releasing base recorder state. All six commands now pass
  a full bounded typed-client/server in-memory duplex proof, and server
  manifests omit `pwright.tracing` when its handlers are absent. The optional
  `pwright-plugin-tracing-navigation` leaf wraps `navigation.goto` only and
  contributes correlated before/after/error entries through an injected
  Page-to-Context route. A dedicated-Chrome test proves one real CDP `goto`
  and external-browser survival. A DT-003 run repeats one warmup plus three
  measured chunks, returns `TraceBytes.current` to zero after every sink
  handoff, and passes logical/browser-process teardown checks. The optional
  `pwright-trace-sink-file` leaf now turns one detached chunk into a versioned
  `.pwtrace` file without adding a task, runtime, browser, downloader, archive,
  or network dependency. It requires a fresh caller-granted directory,
  preallocates bounded per-Context accounting, checks exact encoded file and
  aggregate disk quotas before path creation, streams to a create-new temporary
  file, syncs, and atomically renames. Six AAA tests cover the exact bytes
  through `TracingService`, rejection before file creation, independent
  Context/chunk/aggregate limits, and refusal to adopt an existing directory.
  The separate `pwright-trace-sink-pull` leaf is the task-free boundary before
  remote streaming. Its fallibly preallocated FIFO rejects on global chunk,
  per-Context chunk, per-chunk byte, or aggregate-byte pressure. A queued chunk
  keeps its original `TraceBytes` leases and recorder gate; `pop` releases queue
  admission but transfers that ownership intact to the caller. Context/all
  abort drops only chunks the sink still owns. Six AAA tests cover service
  handoff, transfer-without-release, all admission axes, and selective cleanup.
  Rich trace contributors, a concrete remote transport adapter, repeated
  series, and reviewed budgets remain.
  Source inspection then corrected the remote-transfer direction: Playwright
  returns an Artifact and lets the client pull a child Stream with `read(size)`;
  it does not push trace chunks unsolicited. Pwright records that contract in
  `knowledge/pwright-artifacts-and-streams.md`. The first prerequisite is now
  implemented without expanding tracing 1.0: `ArtifactBytes` is an independent
  ledger class, Artifact/Stream have stable object vocabulary, and the
  command-free `pwright-artifact-model` owns one bounded source shared by
  independent Stream cursors, with explicit EOF and a two-phase prepared read.
  Commit advances one cursor, drop rolls back, a second prepared read on that
  Stream fails, and close releases that Stream's source owner. Seven AAA tests
  cover cancellation, overlap accounting, bounds, close, empty EOF, shared
  source accounting, and independent cursors.
  The next checkpoint freezes `pwright.artifact` 1.0 with generated
  `Artifact.open_stream/failure/delete` and `Stream.read/close` contracts on
  exact object kinds. Its independent codec enforces metadata, request-size,
  attachment-count/index/length, result-byte, and failure-string bounds, copies
  successful read bytes into a separate `ArtifactBytes` ledger, and preserves
  explicit empty EOF. Six codec tests and two generated-server tests prove all
  typed shapes plus capability presence/absence. Typed Artifact/Stream handles
  now reuse the shared bounded client object table, while the independent
  client facade adds all five Playwright-shaped methods and proves remote
  Create-announcement and direct capacity checks. The task-free service
  preallocates bounded Artifact/Stream tables, injects Stream IDs, separately
  limits source/read/failure/retained-result bytes, cleans subtrees child-first,
  invalidates prepared reads on teardown, and completes one client-to-registry
  binary duplex pull. The separate `pwright-artifact-runtime` layer now couples
  registry preparations to controlled-runtime Create/Dispose transactions,
  verifies child-first topology, fences exact lifecycle control before command
  success, and compensates a Stream after failed Create publication. Five AAA
  tests cover normal ordered wire control, runtime-capacity pre-admission,
  transport-close rollback, and failed-fence cleanup. Its memory prerequisite
  is now implemented separately: `pwright-trace-format` reserves exact
  `ArtifactBytes` before allocation and returns that same owned PWRTRACE buffer;
  Artifact model/registry/runtime adopt it without copying or acquiring another
  lease, while the file sink reuses the same format writer. The fifth runtime
  test proves `TraceBytes` reaches zero after handoff while the exact
  `ArtifactBytes` lease survives until delete.
  The next checkpoint freezes the independent `pwright.tracing_artifact` 1.0
  capability with one BrowserContext command and explicit dependencies on base
  tracing and Artifact transfer. Generated capability, schema, binary codec,
  server handler, and opt-in client facade do not modify either dependency's
  1.0 surface. The leaf producer detaches one chunk, encodes directly into the
  exact final `ArtifactBytes` allocation, registers it transactionally, and
  fences the exact Artifact Create before success. Artifact and Stream IDs
  share one source. Failed encoding releases `TraceBytes` and reopens the
  recorder; failed control publication compensates runtime and registry state.
  Four producer AAA tests plus client, codec, and server-manifest tests cover
  successful PWRTRACE readback, exact handoff accounting, a complete typed
  client/binary/generated-server path, announcement claiming, bounded
  rejection, failed-fence cleanup, dependency completeness, and handler
  presence/absence. Rich contributors, ZIP compatibility, repeated series, and
  reviewed budgets remain.

### DT-043: Crate and feature topology

- Question: After boundaries are understood, how should they map to crates and
  Cargo features?
- Deliverable: dependency graph and representative consumer manifests.
- Done when: minimal profiles contain only their approved dependencies and the
  graph has no cycles. Crate names are deliberately postponed until this task.
- Checkpoint: the first representative minimal manifest is executable at
  `examples/v2-navigation-only`. Its only direct dependency is
  `pwright-composition-chromium-navigation`; the leaf selects the dedicated
  `pwright-backend-cdp-navigation`, the command-free shared failure classifier,
  the navigation capability/codec/generated server, and common CDP/core/wire
  foundations. `scripts/check-v2-navigation-dependencies.sh` allowlists its
  exact 15-package normal Pwright closure and runs in CI, so Locator,
  screenshots, network events, tracing, Artifacts, providers, managed browser
  installation, the aggregate backend/composition, and v1 crates are forbidden.
  Shared-infrastructure granularity and remaining capability leaves remain
  open; see
  `knowledge/pwright-dependency-topology.md`.
- Checkpoint: the shared Chromium topology is independently installable as
  `pwright-backend-cdp-topology`. The leaf owns only bounded
  Browser/Context/Page/Frame/World state, generated-CDP-event normalization,
  lifecycle effects, reconnect reconciliation, and resource-accounted
  rollback transactions; it issues no command and starts no task. The
  aggregate backend compatibility-reexports its unchanged public surface.
  `examples/v2-topology-only` and
  `scripts/check-v2-topology-dependencies.sh` enforce an exact five-package
  normal Pwright closure: consumer, topology leaf, browser model, CDP model,
  and core. Capability, codec, client, server, runtime, provider, launcher,
  installer, full composition, aggregate backend, and v1 crates are forbidden.
  The leaf's 36 AAA tests remain attached to their implementation owner.
- Checkpoint: Context/Page creation and lifecycle are four independent CDP
  backend leaves. Context creation has an exact 6-package closure; Context
  lifecycle has 8 because it consumes topology ownership; Page lifecycle has
  6; and Page creation has 8 because exact-target compensation explicitly
  depends on Page lifecycle. Each has a standalone consumer and CI allowlist.
  The leaves preserve all 17 existing AAA tests, issue only their documented
  Target/Page calls, bound every retained native ID and close-confirmation
  loop, and import neither the aggregate nor unrelated capabilities. The
  aggregate compatibility-reexports the same API. Composition remains
  responsible for atomic topology/runtime/route publication around these
  native operations. Dedicated Chrome 152 tests pass for explicit Page close
  and the complete Context-create/Page-create/Page-close/Context-close path;
  both prove that the externally owned browser remains alive.
- Checkpoint: Handle lifecycle is an independent six-package CDP backend leaf.
  It retains one bounded native object ID and issues exactly one
  `Runtime.releaseObject`, with no hidden retry or evaluation dependency.
  Retained Handle creation, Handle-local value/Handle evaluation, `jsonValue`,
  `getProperty`, and bounded `getProperties` form one ten-package
  `pwright-backend-cdp-handle-evaluation` leaf because they share private
  invocation, parsing, and cleanup transactions. That leaf depends on the
  existing by-value evaluation backend; no reverse edge exists. Two standalone
  consumers and exact CI allowlists exclude navigation, Locator,
  content/screenshot, network events, topology, Context/Page creation and
  lifecycle, the aggregate backend, codecs, client/server, runtime, provider,
  installer, Node.js, and browser acquisition. The aggregate compatibility-
  reexports both APIs, and all 11 AAA tests remain with their implementation
  owners. A dedicated Chrome 152 test passes for real element/primitive Handle
  creation, Handle-valued invocation, explicit native release, and preservation
  of the externally owned browser.
- Checkpoint: Handle lifecycle is also an independently selectable 15-package
  Chromium server composition. The lifecycle-owned route registry counts both
  live and in-flight producer reservations, pre-admits maximum retained native
  ID/preview bytes, shrinks to exact publication bytes, and carries an opaque
  typed payload so evaluation plugins do not become lifecycle dependencies.
  Generated `Handle.dispose` performs one injected native release, then retires
  runtime and route ownership. Backend failure preserves both owners; primitive
  Handles perform no native I/O; a dropped started release transfers to a
  caller-owned bounded continuation queue; and missing continuation capacity
  rejects before native work. Multi-Handle creation now reserves the complete
  batch and publishes its routes under one registry lock around the runtime
  batch commit; injected commit failure leaves both sides unpublished and
  releases every reservation. Nine release AAA tests and a dedicated Chrome
  152 proof pass. Exact standalone and umbrella allowlists exclude evaluation,
  topology, other capabilities, aggregates, clients/providers, Tokio, Node.js,
  and browser acquisition. The next slice is the Handle-evaluation composition
  that consumes this registry and lifecycle installation.
- Checkpoint: Page and network event normalization are independent CDP backend
  leaves with exact seven-package closures. Page events explicitly depends on
  the command-free topology state needed to identify current Page/main-Frame
  routes and maps `PageReady`/`PopupLinked`; network events owns a bounded
  Page/session request correlation lane and has no topology dependency. Neither
  leaf owns a receiver, queue, task, runtime, transport, response body, browser
  session, or process. Standalone consumers and CI allowlists forbid unrelated
  commands, codecs, clients, servers, providers, installers, aggregate backend,
  Node.js, and browser acquisition. The aggregate compatibility-reexports both
  APIs, and all 10 AAA normalization/resource tests stay with their owners.
  Three dedicated Chrome 152 tests pass for caller-owned Page-load delivery,
  bounded request/response/finish without body reads, redirect plus HTTP-error
  semantics, transport failure, cleanup, and external-browser survival.
- Checkpoint: the complete Chromium Page-event server surface is also an
  independently selectable 15-package composition leaf. It installs three
  separately bounded `RejectNewest` queues for `PageLoad`,
  `BrowserContextPage`, and `PagePopup`, one independent caller-polled CDP
  cursor for load normalization, and the ordered topology creation fence. Its
  profile retains each lane's exact capacity and decoded-byte bound and rejects
  zero capacity, under-budgeted discovery payloads, incomplete aggregate
  selections, and mixed wire profiles. The leaf owns no task, executor,
  topology, transport reader, browser, process, Node.js, or browser
  acquisition. The complete recipe now consumes this leaf, the aggregate only
  compatibility-reexports it, and the umbrella exposes separate client and
  Chromium Page-event features. Three consecutive Chrome 152 runs pass for
  overlapping navigation/Locator cursors, PageLoad delivery, complete drain,
  graceful close, and preservation of the external browser. The next event
  composition boundary is the larger ordered network lifecycle surface.
- Checkpoint: the cross-event network delivery and causal lifecycle turn are
  now an independently selectable 16-package composition leaf. One queue and one sequence source
  preserve `request -> response -> requestfinished/requestfailed` ordering
  across all four public event tags. Its exact profile rejects zero capacity,
  outbound frames that cannot hold the largest terminal snapshot, and decoded
  retention below the codec's derived maximum. Aggregate conversion also
  rejects mismatched normalization/codec limits. Its generic publisher and
  retained backend own the Create fence and one receive -> publish -> exact-Page
  acknowledgement turn through injected batch-driver and object-safe dynamic
  subscription-selection ports. The leaf owns no CDP receiver, listener
  mutation, topology registry, executor task, browser, process, Node.js, or
  browser acquisition. AAA proofs cover routed decoding, Page-before-Context
  fan-out, post-publication acknowledgement, and terminal retained-future
  release. The complete recipe consumes the new backend; old aggregate types
  remain compatibility APIs. Separate `client-network-events` and
  `chromium-network-events` umbrella profiles keep the isolated matrix at 30.
- Checkpoint: Page input is an independent five-package CDP backend foundation.
  `pwright-input-model` owns only the shared 16-key chord bound, while
  `pwright-backend-cdp-input` keeps the necessarily coupled Playwright US-key
  resolver and serialized keyboard/mouse transaction state together. It owns
  no selector, Locator command, capability, runtime, transport, task, browser
  session, or process. The aggregate compatibility-reexports the previous
  coordinator, transaction, and modifier types; the full Locator adapter has a
  one-way dependency on the input leaf and the input leaf has no reverse
  Locator edge. `examples/v2-input-only` plus an exact CI allowlist reject any
  sixth Pwright package. Fifteen AAA tests cover bounded parsing, key mapping,
  serialization, dispatch, rollback, and reverse-order cleanup, while two
  dedicated Chrome 152 tests exercise typed Locator input and explicit
  cancellation without terminating the external browser.
- Checkpoint: the complete CDP Locator implementation is now the independent
  `pwright-backend-cdp-locator` leaf. It composes only the portable Locator
  contract, command-free selector runtime/failure classifier, Page-input foundation, CDP, core,
  and their value models; it owns no screenshot capability, aggregate backend,
  runtime, transport, server, provider, browser launcher, or process. The
  aggregate preserves its existing Locator type reexports, native-CDP access,
  and a narrow screenshot-geometry compatibility adapter. The standalone
  `examples/v2-locator-only` consumer and exact ten-package CI allowlist make
  that boundary executable, while all 98 Locator AAA tests remain beside the
  implementation owner.
- Checkpoint: `pwright-composition-chromium-locator` now binds one explicit
  Page or Frame identity to the independent backend and one generated Locator
  protocol table. Its typed profile selects exactly Locator 1.22 or 2.5, shares
  a caller-supplied Page input coordinator, and reserves unique execution-world
  routes only for Frame installations. The 18-package standalone dependency
  proof excludes the aggregate backend/composition, screenshots, navigation,
  public evaluation, runtime, provider, launcher, installer, and browser
  acquisition. Three AAA tests prove v1 routing, v2 unique-context routing,
  shared input, resource release, complete handler construction, and rejection
  before CDP for an unbound object. A dedicated Chrome 152 integration test
  covers the new v2 composition directly.
- Checkpoint: `pwright-composition-chromium-context-lifecycle` is the first
  independently selectable ownership-changing server leaf. It stages an exact
  topology close, child-first runtime disposal, and bounded caller-owned
  continuation before native `Target.disposeBrowserContext`. One injected
  retirement plugin prepares any Page-route, Handle, network, or composite
  cleanup without linking those capability crates, and returns an infallible
  post-close commit token. Its exact 17-package consumer graph rejects the
  aggregate backend/composition, clients, providers, launchers, installers,
  Tokio, Node.js, and browser acquisition. Five AAA tests prove full commit,
  rollback, unowned rejection, plugin ordering, handler construction, and
  cancellation-safe continuation; a dedicated Chrome 152 test closes one
  temporary owned Context and preserves the external browser.
- Checkpoint: `pwright-composition-chromium-context-creation` independently
  installs the Playwright-shaped creation/lifecycle pair instead of exposing a
  creation-only half. One pre-CDP publication plugin reserves optional state;
  runtime and topology publication follow native creation, and any later
  failure disposes the exact native Context through the lifecycle backend. Two
  caller-owned continuation queues separately bound creation and close. Six
  AAA tests prove paired create/close, backend rollback, exact compensation,
  plugin-capacity rejection before CDP, started-future transfer, and missing
  continuation rejection. Its standalone exact 22-package graph excludes the
  aggregate backend/composition, clients, providers, launchers, installers,
  Tokio, Node.js, and browser acquisition.
- Checkpoint: `pwright-composition-chromium-page-lifecycle` independently
  installs ordinary Page close plus before-unload initiation. Ordinary close
  pre-admits topology, child-first runtime disposal, an injected retirement
  plugin, and caller-owned continuation capacity before native I/O, then waits
  for bounded exact-target absence before committing. Before-unload resolves
  only the exact Page session and preserves local liveness. Seven AAA tests
  cover commit, rollback, plugin rejection, session routing, cancellation,
  capacity, and invalid setup; a dedicated Chrome 152 test confirms Page-only
  closure while preserving its Context and the external browser. Its exact
  17-package closure rejects aggregates, clients, providers, launchers,
  installers, Tokio, Node.js, and browser acquisition.
- Checkpoint: `pwright-composition-chromium-page-creation` pairs
  `BrowserContext.newPage` with the required Page-lifecycle leaf. It reserves
  optional publication state before CDP, binds the exact native route
  provisionally, publishes Page and main Frame as one topology/runtime unit,
  and compensates the exact target on every post-create failure. Creation and
  close have independent bounded continuations. Six AAA tests and one Chrome
  152 test cover the paired path, two-stage plugins, capacity rollback,
  cancellation transfer, exact compensation, and external-browser survival.
  Its exact 22-package graph excludes aggregates, clients, providers,
  launchers, installers, Tokio, Node.js, and browser acquisition.
- Checkpoint: `examples/v2-umbrella-profiles` adds twenty-one representative
  downstream profiles over the new `pwright` package: no features,
  `client-context-creation`, `client-context-lifecycle`, `client-page-lifecycle`,
  `client-page-creation`,
  `client-locator`, `client-content`,
  `client-locator-screenshot`,
  `client-dom-content`, `client-evaluation`, `client-navigation`,
  `chromium-context-creation`, `chromium-context-lifecycle`,
  `chromium-page-lifecycle`, `chromium-page-creation`, `chromium-locator`, `chromium-content`,
  `chromium-locator-screenshot`,
  `chromium-dom-content`,
  `chromium-evaluation`, and `chromium-navigation`. A second CI verifier compares
  each normal Cargo graph against an exact allowlist and separately checks
  feature edges, including complete rejection of the aggregate CDP backend
  from every minimal Chromium profile. The umbrella feature graph is therefore
  concrete and executable,
  not only a proposed crate list. Remaining capability families must earn
  equivalent leaf and consumer proofs before being added.
- Checkpoint: `examples/v2-evaluation-only` is the second standalone minimal
  manifest. Its sole direct dependency is
  `pwright-composition-chromium-evaluation`, whose normal 15-package Pwright
  closure uses the extracted evaluation backend rather than the aggregate CDP
  backend. `scripts/check-v2-evaluation-dependencies.sh` allowlists that whole
  graph in CI, making absence of navigation, Handle, provider, installer,
  runtime, full composition, and v1 crates executable evidence.
- Checkpoint: `examples/v2-dom-content-only` is the third standalone minimal
  manifest. Its sole direct dependency is
  `pwright-composition-chromium-dom-content`; its exact normal 15-package
  Pwright closure includes only the command-free shared CDP classifier plus
  DOM-content capability, codec, generated server, adapter, composition, and
  common CDP/core/wire foundations. Its CI allowlist rejects public evaluation,
  screenshot, navigation, aggregate backend/composition, clients, provider,
  installer, runtime, and every v1 crate.
- Checkpoint: `examples/v2-content-only` is the fourth standalone minimal
  manifest. Its sole direct dependency is
  `pwright-composition-chromium-content`; its exact normal 17-package Pwright
  closure includes separately reusable screenshot value and attachment-codec
  leaves but excludes DOM content, public evaluation, Locator screenshot,
  navigation, aggregate backend/composition, clients, providers, installers,
  runtimes, and every v1 crate. CI rejects any future graph expansion.
- Checkpoint: `examples/v2-locator-screenshot-only` is the fifth standalone
  minimal manifest. Its exact 23-package closure combines the independent
  capture backend with the command-free selector runtime and first-party
  selector-geometry plugin, selector/screenshot models, reusable codecs, and
  the generated one-command server. CI still forbids `pwright-cap-locator`,
  the aggregate CDP backend/composition, clients, providers, installers,
  runtimes, Node.js, and browser acquisition.

## Phase 5: design browser integration

### DT-050: Browser provider and ownership

- Question: How are system browsers, explicit executables, remote endpoints,
  and optional managed downloads discovered and owned?
- Deliverable: provider contract and process-lease lifecycle.
- Done when: Pwright can never terminate a browser it does not own, and browser
  download is impossible without explicit installation and invocation.
- Checkpoint: `pwright-composition-chromium-launch` is an optional leaf above
  the attach-only root. It accepts only an explicit launch request, uses a
  production connector whose connection retains plan and ledger identity, and
  returns a composition paired with one consuming process lease. Binding
  rejects foreign plans and foreign ledgers. External attachment has no owned
  shutdown method by compile-time proof. A live installed-Chrome test proves an
  unpolled shutdown remains lazy, a polled shutdown reaps the exact launched
  process/profile, and the unrelated external Chrome remains live. The leaf has
  no installer/downloader dependency and leaves managed installation explicit.
- Checkpoint: an additionally selected `pwright.browser_lifecycle` leaf reserves
  its own codec/wire resources and augments only the owned launch handshake;
  the attach-only manifest remains unchanged. Missing handshake configuration
  fails before process spawn. Only a real accepted session that negotiated the
  capability can consume the process owner into a non-cloneable close grant;
  acceptance must also come from a production supervisor sharing the launch's
  exact connection resource ledger. An unbound handshake and an equally
  configured foreign connection are rejected; rejection returns the exact
  owner. The remote service validates the Browser target, prepares complete
  runtime disposal, consumes shutdown authority once, transfers started
  cancellation into caller-reserved continuation capacity, and publishes
  descendant-first Dispose through Browser Dispose before Result. A real
  Chrome proof exercises unprivileged, unbound, foreign-session, and exact-owner
  authorization paths before the one valid remote close reaps the exact
  process. Portable success remains `()` across capability, codec, client,
  server, and wire. An optional composition-only report sink receives the typed
  provider report after runtime disposal; the default sink retains nothing.
  Encoded and live-Chrome proofs show the sink receives provider diagnostics
  while no PID, exit status, shutdown method, or profile path enters the remote
  result.

### DT-051: Backend abstraction

- Question: Where is the boundary between Playwright semantics and Chromium
  CDP, future WebDriver BiDi, Firefox, or WebKit implementations?
- Deliverable: backend responsibilities and escape-hatch policy.
- Done when: CDP identifiers and domain messages cannot leak into portable API
  contracts unintentionally.

### DT-052: Browser context, target, and session mapping

- Question: How do public BrowserContext/Page/Frame objects map to backend
  contexts, targets, sessions, execution worlds, and process events?
- Deliverable: Chromium mapping and recovery state machines.
- Done when: creation races, target crashes, detach, popup, iframe, reconnect,
  and exact-target cleanup have defined behavior.

## Phase 6: operational design

### DT-060: Memory ownership and limits

- Question: What state exists, who owns it, how large may it grow, and when is
  it released?
- Deliverable: allocation inventory, limit table, eviction policy, and memory
  metrics schema.
- Done when: object tables, event queues, bodies, logs, traces, screenshots,
  selectors, handles, and pending calls all have explicit bounds.
- Checkpoint: `pwright.content` 1.0 now proves screenshot admission without an
  unbounded queue. The Chromium adapter reserves its configured maximum before
  base64 decode, rejects an oversized encoded response before allocation,
  shrinks the lease to exact retained bytes, and releases Page/global admission
  on completion or dropped future. The remote frame owns a separate
  `WireBytes` copy and the client result owns a separate `ScreenshotBytes`
  lease, so source, transport, and consumer lifetimes are not conflated.
- Checkpoint: Frame DOM serialization uses a dedicated `DomContentBytes` lease
  retained by a non-cloneable UTF-8 value, one active read per Frame, bounded
  connection concurrency, and one UTF-8 wire attachment. Chromium locator and
  DOM content share one utility execution-route record, and an unnegotiated
  remote DOM-content capability installs no handler and reaches no backend.
- Checkpoint: accepted-capability gating now covers every remote application
  handler and typed event lane, not only DOM content and locator. The gate runs
  before codec construction, service wrapping, queue creation, or production
  resource claims. Remote event profiles also retain their stable capability
  owner and are rejected during preparation when that owner is not installed.
  A control-only handshake proves all ten current application surfaces remain
  absent and the complete resource metrics snapshot is unchanged.

### DT-061: Runtime and concurrency model

- Question: Which runtime services and tasks exist, and what ordering is
  guaranteed per browser, context, page, session, and call?
- Deliverable: task/actor model, scheduler rules, fairness, and shutdown flow.
- Done when: the design has no task per API object, no unbounded spawn path, and
  no hidden global runtime dependency.

### DT-062: Observability and diagnostics

- Question: How can users explain memory, latency, retries, hangs, object leaks,
  browser failures, and missing capabilities?
- Deliverable: stable metrics, structured events, diagnostic snapshots, and
  opt-in tracing rules.
- Done when: diagnostics are useful without retaining full payloads or stacks
  by default.

### DT-063: Security and trust boundaries

- Question: What changes when attaching to local, tailnet, or untrusted remote
  browsers and runtimes?
- Deliverable: threat model for endpoints, credentials, browser data, file
  access, downloads, evaluation, plugins, and remote transport.
- Done when: authentication, authorization, origin, secret redaction, and unsafe
  escape hatches have explicit owners.

## Phase 7: prove and evolve the design

### DT-070: Testing and conformance strategy

- Question: How will behavior be verified without coupling tests to one backend
  or relying primarily on mocks?
- Deliverable: test pyramid covering generated protocol tests, deterministic
  fakes, backend contract tests, real-browser tests, Playwright comparison
  tests, failure injection, and memory soak tests.
- Done when: every compatibility and resource claim maps to an automated test.
- Checkpoint: screenshot tests cover schema drift, command metadata/options,
  invalid combinations, direct Page routing, typed CDP parameters, exact base64
  padding bounds, oversize rejection, dropped in-flight cleanup, generated
  remote handler transfer as one binary attachment, and client-side retained
  result accounting. A dedicated-profile live test verifies real Chromium
  full-page PNG dimensions, lease release, isolated Context cleanup, and
  preservation of the externally owned browser. JPEG/WebP and pixel-fidelity
  conformance remain open.
- Checkpoint: the command-free `pwright-screenshot-model` now owns format,
  scale, clip, options, and retained screenshot bytes while
  `pwright-cap-content` reexports those exact types. The first Locator
  screenshot design is a separate capability and one cancellation-bounded
  server command, not an addition to Locator/Page capture or a droppable
  two-command temporary-handle sequence. The semantic selector model has now
  moved into zero-dependency `pwright-selector-model`, and its recursive wire
  closure is published as `pwright.selector` 1.0. The independent
  `pwright.locator_screenshot` 1.0 schema, frozen baseline, portable contract,
  bounded codec, typed client extension, and generated server binding are now
  implemented. The codec rejects Page-only clip/full-page options before
  encoding and retains decoded bytes under `ScreenshotBytes`. The command-free
  `pwright-codec-screenshot` helper now gives Page and Locator capture the same
  attachment index/count/length/limit checks and exact result-copy accounting.
  The complete Chromium profile owns its full-Locator resolver's selector and
  Frame-route budgets plus capture, codec, and wire budgets without advertising
  ordinary Locator or Page screenshot. The independently installable capture
  leaf instead accepts a command-free geometry resolver plugin and excludes
  `pwright-cap-locator`; applications combine that plugin's declared resources
  with the capture profile. Backend and composition tests cover strictness,
  post-scroll geometry, future-drop cleanup, local routing, and negotiated
  binary transfer. A dedicated-Chrome proof verifies exact visible and
  below-fold PNG dimensions, Context/target cleanup, zero retained screenshot
  bytes, and continued external-browser reachability. Child-Frame capture now
  follows Playwright's native owner translation: each ancestor is resolved by
  `DOM.getFrameOwner` on its parent session and contributes its native border
  origin. Shared linked translation nodes remain bounded by the Frame topology.
  Unit proofs cover local and nested OOPIF routing, including exact child,
  parent, and Page session ownership; a dedicated-Chrome same-process iframe
  proof verifies an exact 70x50 PNG and cleanup. Cross-site OOPIF live and
  decoded pixel fidelity remain open; see
  `docs/knowledge/pwright-locator-screenshot.md`.

### DT-071: Versioning and compatibility

- Question: How do API, capability, protocol, codec, and backend versions
  evolve independently?
- Deliverable: compatibility and negotiation rules plus deprecation policy.
- Done when: old/new local and remote combinations have a defined result and
  capabilities can evolve without forcing an unrelated full-stack upgrade.
- Checkpoint: locator 1.21 and 2.4 now prove independent generated artifacts,
  a frozen 2.0 baseline, exact-major negotiation, exact selected-minor codec
  gating, one selected per-connection handler tree, v2-on production routing,
  and v2-off rollback isolation. Cross-capability release and deprecation
  policy remains broader DT-071 work.
- Checkpoint: locator 1.22 and 2.5 append ScrollIntoViewIfNeeded behind exact
  old-minor rejection while preserving byte-identical v1/v2 payloads. The
  selected-major server route reaches a two-phase Chromium utility-world wait
  and synchronous commit, waits only for stable geometry, skips repositioning
  at intersection ratio one, and retains no DOM handle, input transaction, or
  new resource class.

### DT-072: Migration and release strategy

- Question: How can v2 be validated without destabilizing current Pwright?
- Deliverable: isolation, comparison, adoption, rollback, and release plan.
- Done when: implementation can begin with a bounded vertical slice, current
  users retain a supported path, and no legacy code must be rewritten in place.
- Checkpoint: the new `pwright` umbrella is additive beside every v1 package and
  changes no legacy default. All optional dependencies carry matching path and
  `0.1.0` version requirements, and Cargo successfully normalizes/lists the
  package. Registry upload verification correctly remains unavailable until
  those currently unpublished v2 leaf crates are released; the required order
  is foundational leaves, composition leaves, then umbrella. Repository users
  can select the verified Git features before that release without Node.js or
  browser acquisition.
- Checkpoint: adding the evaluation contract/client/server feature trio remains
  additive and no-default-features. The complete backend continues to compile
  and pass its existing test suite through a compatibility reexport, while
  downstream consumers can select the smaller evaluation backend and
  composition directly.
- Checkpoint: the DOM-content contract/client/server feature trio follows the
  same additive path. The aggregate backend compatibility-reexports the
  extracted adapter and preserves its prior DOM tests, while isolated consumers
  select the smaller backend and composition directly. No v1 default, API, or
  package was replaced.
- Checkpoint: Page screenshot adds the same no-default-features contract,
  client, and Chromium-server trio. The aggregate backend continues to expose
  the moved service, including its existing Locator-screenshot reuse, while an
  isolated consumer can select the smaller backend and composition. No v1
  default, API, or package was replaced.
- Checkpoint: navigation now follows the same extraction rule. The dedicated
  `pwright-backend-cdp-navigation` owns its two-feature descriptor, bounded
  resource profile, event wait state, and injected-CDP services. The aggregate
  backend preserves its public constructors, cancellation, and native-access
  behavior through a thin compatibility wrapper, while minimal consumers never
  link the aggregate crate. A direct dedicated-Chrome test proves the leaf can
  navigate a real target without acquiring or closing the external browser.

### DT-073: Node compatibility target

- Question: What does Playwright compatibility mean for a Node consumer?
- Decision: track import, shape, value, lifecycle, and real-browser parity as
  separate evidence levels. Public compatibility does not include Playwright's
  private channel protocol or internal classes.
- Evidence: `node-ffi-compatibility-design.md` defines C0 through C4 and the
  explicit unsupported-operation behavior.

### DT-074: Native binding boundary

- Question: Which logic may cross into JavaScript?
- Decision: the JS facade owns only API ergonomics, EventEmitter integration,
  call-site stack capture, and artifact loading. A thin Node-API crate converts
  values; a separately testable safe Rust host owns every browser operation.
- Gate: no browser/CDP implementation may enter the JS package or N-API crate.
- Checkpoint: `pwright-node-host`, `pwright-node-addon`, and `packages/pwright`
  now implement the first attach/topology slice. A dependency check rejects
  N-API and legacy bridge/gRPC server dependencies in the safe host. The addon
  contains conversion/delegation only. Node/browser behavior is tested against
  pinned upstream Playwright; this is not full API compatibility.
- FFI checkpoint: a safe-host budget owns async execution and settlement
  tickets. The addon bounds borrowed JS strings before copying, charges actual
  boxed-future size before allocation, and retains native-result leases through
  JS conversion. A large by-value connect future exposed V8 callback-stack
  exhaustion; boxing before N-API generic layers fixes the real-Node failure.

### DT-075: Embedded async ownership

- Question: How do Promise progress, cancellation, and shutdown remain explicit?
- Decision: each connected session owns its Rust driver task, cancellation
  source, terminal state, and bounded queues. Explicit close joins the task;
  JavaScript GC sends no protocol traffic.
- Gate: abandoned Promises, timeout races, and Node environment teardown need
  deterministic tests before VS1 completion.
- Checkpoint: explicit close joins the topology driver and the new CDP
  `io_finished` barrier. Worker teardown exposed a borrowed-native-object
  lifetime defect; async exports now clone owned Rust state before scheduling.
  Repeated real-browser close shows zero ledger leases/scopes/event bytes.
  Timeout races and finer environment-cleanup instrumentation remain open.
- VS2 checkpoint: commands now use controlled dispatch, explicit Tokio
  deadlines, bounded cleanup, and per-session active-operation tracking. Real
  stalled-response cases prove TimeoutError and close-during-navigation
  cancellation; close reports zero pending operations and cleanup jobs. Native
  finalization requests cancellation independently of in-flight Arc ownership.
- Worker checkpoint: environment cleanup now joins Rust shutdown before the
  Node runtime teardown hook. Graceful close drains abandoned-command backend
  cleanup before closing CDP; actual peer loss skips impossible native cleanup.
  Destroying a Worker during stalled navigation now cancels the server-observed
  request, both in isolation and in the full lane, with native counters at zero.
  FFI resolver/output retention was a separate open budget (NODE-011).
- FFI checkpoint: the current async exports now share environment-wide limits
  of 16 connect, 32 command, 16 close and 16 disconnect-wait settlements, with
  64 MiB reserved native payload/future bytes. Close has independent capacity.
  Leases survive Rust completion until JS conversion or environment reclamation;
  success and error outcomes follow the same rule. Environment teardown joins
  Rust execution and clears queued payloads without JS callback delivery.
  `pwrightFfiMetrics()` reports this separately from per-host runtime and
  metadata counters. General event/binary budgets and RSS remain open.

### DT-076: Event delivery and backpressure

- Question: How do Rust events become EventEmitter events without an unbounded
  cross-thread queue?
- Decision: listener counts control Rust subscriptions. Rust retains the
  authoritative bounded queue and uses a finite, nonblocking Node-API
  thread-safe-function queue. Overflow is observable under a selected loss
  policy.
- Gate: first-listener, last-listener, overflow, ordering, and environment-close
  behavior need direct tests.
- Metadata publication checkpoint: a paired blocked-JS fixture reproduced
  NODE-009's early URL visibility. Native and published URL views now live in
  Rust with a 256-item ordered queue and fixed-watermark callback publication.
  Existing watch admission carries metadata readiness and disconnect; no new
  unbudgeted task or JS URL cache is introduced. Command/error/close callbacks
  publish their preceding watermark before conversion. Stale fences cannot
  regress state; native, published and pending URL bytes share the 2 MiB budget.
  Eight safe-host tests exercise admission, ordering and callback-free teardown.
  The first expanded lane passes 31/31, including the formerly failing paired
  within-turn URL assertion. General public events and listener-driven fan-out
  remain open; metadata readiness is not a substitute for their design.
- Verification extension: two lanes pass 32/32 including Worker destruction
  with a target-local unpublished URL revision. The host suite now passes
  29 release tests. FFI `queuedCommands` is separate from total `queued`, so
  metadata notifications cannot inflate command-pressure evidence. No general
  event, full compatibility or RSS claim is implied.
- Final rebuilt-addon verification: 32/32 live cases pass again without skips,
  TODOs, native panics or environment-cleanup diagnostics, including exact-Frame
  publication probes and the separate queued-command counter.

### DT-077: Node object identity and lifecycle

- Question: How do wrappers preserve Playwright identity without trusting GC?
- Decision: one JS facade is cached per session/object ID; Rust remains the
  ownership authority. Close/dispose is explicit and idempotent, and parent
  disposal invalidates descendants before later FFI calls.
- Gate: duplicate announcements, stale wrappers, recursive disposal, and cache
  eviction need differential coverage.
- Checkpoint: no-options Context `newPage` and ordinary Page `close` now select
  existing Rust services. The eight-entry native driver queue serializes them
  with topology publication; Page metadata is reserved before target creation
  and the returned descriptor owns that exact snapshot across retirement.
  Repeated/concurrent close and concurrent creation have paired browser tests.
  Playwright-style foreign-Context Page attribution is explicitly enabled only
  by the Node host; generic Rust topology keeps strict exclusion by default.
  Before-unload, public close events and shutdown during creation remain open
  under NODE-012. The following Context checkpoint adds owned disposal.
- Verification: the final rebuilt lane passes 37/37, including Page close
  cancelling its pending navigation without disconnecting the Browser. An
  earlier lane had one candidate-attach timeout in fixture setup; NODE-013
  preserves it as unresolved and adds bounded initialization-phase diagnostics.
  A subsequent green lane is not evidence of that timeout's root cause.
- Context checkpoint: explicit owned Context creation/close, default-Context
  attachment disconnect, sibling isolation and descendant navigation
  cancellation are wired through the same serial Rust driver. Native inventory
  checks verify exact Context/Page disappearance after browser close and Worker
  termination. An optional pre-resume session initializer supplies owned-Context
  viewport, window, focus and media defaults. First-document snapshots expose
  discrepancies without an observation connection changing those defaults.
  Download initialization, headless fonts and Context options remain NODE-014;
  the following DT-079 checkpoint adds the shared fixture adapter (NODE-005).
  The rebuilt 44-case real-Chrome lane passes without skips/TODOs; safe-host
  release tests pass 39 cases and the aggregate composition passes 197 unit
  cases. Exact Context dependency closures and strict compile/lint checks pass.

### DT-078: Errors and binary values

- Question: How are failures and large byte results represented across FFI?
- Decision: Rust returns structured bounded errors that map to stable JS error
  subclasses; screenshots/artifacts use bounded owned Buffers, initially with
  an explicit copy when required. Unbounded base64 is forbidden.
- Gate: error class/code/message parity, call-site stacks, maximum-size Buffers,
  and teardown during rejection need tests.
- Checkpoint: queued errors and failed JS conversion release the same FFI
  ticket leases as successful results; oversized UTF-8 arguments are rejected
  before copying or native navigation. Structured error parity and binary
  payload ownership are still separate, unfinished work.

### DT-079: Differential oracle

- Question: How can the Node package prove that a similarly named method has the
  same semantics as Playwright?
- Decision: extend the existing `tests/` fixture corpus with a Node-FFI adapter
  and run it beside the upstream Playwright and legacy gRPC adapters. Classify
  value, error, ordering, lifecycle, timeout, resource, and unsupported deltas.
- Gate: no blanket golden update; every accepted discrepancy needs scoped
  evidence and a review condition.
- Checkpoint: six package-local real-Chrome cases passed three consecutive
  runs against Playwright 1.58.2, preserving unrelated browser pages. Native
  panic diagnostics fail the lane even when JS assertions pass. Shared-fixture
  adapter work remains open; local issues NODE-001 through NODE-008 in the
  Node design document explicitly bound the current claim.
- VS2 checkpoint: the extended sixteen-case lane passed navigation response
  metadata under all four wait policies, Page/Frame reads, document replacement,
  same-document and about:blank null responses, timeout, and active close. Full
  VS2 is still open: NODE-009 is native synchronous URL state; NODE-010 records
  the main-world versus utility-world title discrepancy with a paired TODO test.
- Current extended lane: eighteen cases, seventeen passing and one executable
  NODE-010 TODO; the known title mismatch remains visible in test output and
  must not be counted as parity. The Worker-in-flight cancellation case passes.
- Verification: after isolating fixture discovery from unrelated Page
  navigation, three successive runs each produced 17 passes plus the same
  explicit TODO (44.62 s / 61.61 s / 46.75 s). Isolated Worker teardown and the
  safe Rust live close test also passed. No full compatibility or RSS claim.
- DOM-read 1.1 checkpoint: NODE-010 is now resolved by a fixed `FrameTitle`
  command on the shared utility-world route, with codec, server and Rust client
  bindings. The prototype regression no longer has a TODO; it and the new
  concurrent title/content case pass, giving nineteen passing Node cases.
  The former title-only evaluation profile is removed. The standalone DOM-read
  dependency proof remains fifteen packages; URL state, FFI budgets and the
  broader Node compatibility goal remain open.

The preceding differential checkpoint now also includes native synchronous
Page/main-Frame URL snapshots. Three 26-case live lanes passed without
skips or TODOs, covering initial fragments, command-return ordering, external
document/history/hash navigation and retained URLs after browser/Page close.
Per-host metadata entry/byte leases outlive active routes only while wrappers
remain retained and have separately reported metrics. NODE-009's JS-turn and
public-event ordering gate remains open with NODE-007; this is not full
compatibility. NODE-011 now has bounded current-export admission and queued
result/error ownership. Four added stress cases exercise a blocked JS loop,
saturated command slots with close headroom, oversized UTF-8 arguments and
Worker destruction with undelivered results. The host release suite passes
20 tests, including ten FFI budget/reclamation cases. These resource tests
supplement the paired oracle cases; they are not an RSS or full-parity claim.
Two final-profile live lanes pass 30/30 with zero skips/TODOs; the latter uses
an over-800,000-byte UTF-8 DOM result in the blocked-JS pressure case. Workspace
release check, strict host/addon Clippy, seven offline tests, TypeScript and the
221-package host dependency check also pass.

#### Shared-corpus checkpoint

The shared-corpus integration now selects all 19 existing API suites / 57 cases
through one catalog shared with the legacy golden runners. Both paired sides
own fresh Contexts and always attempt Page/Context/connection cleanup. The Node
adapter calls only the public FFI package, checks its resource counters after
close, and explicitly rejects missing operations. Static fixtures come from a
verified `serve-lib` route; their bytes are checked before browser execution.
Original fixture functions, HTML and golden snapshots remain unchanged.

Three complete runs each record 6 matches, 49 unsupported cases and
2 oracle errors, with zero value mismatches, candidate/setup/cleanup errors or
native panic diagnostics. The full gate intentionally exits 1. Thirty runner
unit tests cover classification, caught unsupported errors, failed cleanup,
budgets and catalog completeness. No unsupported operation or failed oracle is
counted as parity. The existing package-local 44-case lifecycle checkpoint is
separate evidence, not 44 of these 57 shared cases.

NODE-015 records the two oracle history timeouts: an independent bounded probe
observes BFCache restoration and a complete restored document while Playwright
1.58.2 still waits for `load`. Keep this diagnosis separate from the paired gate;
do not modify the shared Chrome profile or weaken `waitUntil` to erase it.
NODE-005's adapter infrastructure is present, but exposing and verifying the
remaining Rust-backed APIs is still open. See [the runner guide](../tests/README.md).

#### By-value evaluation checkpoint

Page/main-Frame `evaluate` now delegates through Node-API to the existing typed
Rust evaluation capability. The conversion layer owns bounded input/output
trees, preserves cyclic values and typed arrays, and keeps its 8 MiB result
reservation until JS conversion or environment reclamation. The safe host
explicitly selects the evaluation profile; title remains a separate fixed read.

The paired lane first exposed and then verified fixes to global eval scope and
Error stack prefixes in the reusable Rust backend. Its initial 16 cases passed;
the historical expanded 24-case lane passed 23 and failed NODE-017, immediate
TypedArray mutation timing. Keep that assertion and its original values: the
official local transport aliases bytes until asynchronous dispatch, while our
native conversion snapshots them. A bounded deferred binary conversion design,
including detachment/microtask/Worker tests, is the next gate, not a borrowed
pointer across Rust awaits. NODE-018 records other unimplemented value edges.

Bounds, blocked-JS results, close admission, and Worker destruction of both
running and queued evaluations are tested. Host 41 / evaluation backend 9 /
handle-evaluation backend 17 / capability 6 / codec 5 release tests, strict
Clippy, Node/shared types and the unchanged 221-package host boundary pass.
Neither the package-local lane nor a few additional shared matches close the
complete Node compatibility, distribution, or process-memory goal.

The unchanged 57-case shared corpus now records 12 matches, 43 unsupported and
the same 2 oracle history errors, with no value/candidate/setup/cleanup errors
or native panic. The six added matches use public evaluation, not new adapter
browser logic. The complete gate remains nonzero; the original 44-case package
lifecycle/resource lane passed after installing evaluation.

#### Deferred-binary conversion checkpoint

NODE-017's five reproduced timing failures now match the pinned oracle using
two Node-thread phases: snapshot ordinary structure at admission, retain bounded
fixed binary views, then copy owned bytes at an injected `setImmediate` before
Rust execution. A non-Send environment registry owns at most 32 prepared calls,
under existing 8 MiB-per-call / 64 MiB total FFI admission. Its 1 MiB aggregate
backing-capacity limit counts full buffer maxima, not only view lengths.
Conversion metrics and cleanup are distinct from queued native output; the
environment drops unstarted conversions before joining the Rust task tracker.
No JS reference or borrowed byte pointer enters Rust awaits.

The expanded 38-case evaluation lane passed once, including prepared/running/
queued Worker teardown, binary pressure, getter failure, detachment and resize.
The final rebuilt rerun instead reports 35 passes and three Worker setup
timeouts during candidate topology initialization (NODE-013). The lifecycle
rerun also fails: 17 passes and one Worker failure were reported before the
180-second supervisor deadline. These failures remain evidence; neither lane
is currently claimed green. The last 12/43/2 shared matrix predates this change
and was not rerun after those failures.

Release host tests (41), strict host/addon Clippy, workspace release check,
package/shared types, seven package tests, 30 runner tests and the unchanged
221-package safe-host boundary pass. Next gate: bounded attachment-stage
diagnostics and a reproducible NODE-013 case, then full lifecycle/evaluation/
shared reruns. Do not increase deadlines, reset shared Chrome, or equate zero
Rust counters with reclaiming Chrome worlds or RSS (NODE-016). NODE-018 and
the remaining API/distribution goals stay open.

#### Attachment diagnostic checkpoint

A bounded, test-only command timeline now reproduces NODE-013 outside Node-API.
An initial standalone host attach took 64 ms; holding a Playwright connection
caused main-thread, Worker and independent Rust host attempts to reach the same
five-second topology deadline. The last unanswered commands differed between
`Runtime.enable` and `Page.getFrameTree`; the preceding timeline contains many
slow requests rather than one consistently stuck operation.

The Nagle hypothesis was tested with separately built release probes under one
oracle connection and an unchanged target-ID set. Off/on/on/off took
2.046/2.200/2.116/2.132 seconds. No consistent improvement was demonstrated, so
the speculative production socket option was removed. A later final probe
passes at 3.2–3.4 seconds under oracle overlap; this is variable live state, not
a verified fix or replacement for the failed full regression lanes.

Next design must distinguish target-local/browser scheduling latency from the
serial initialization path. Review Playwright's parallel initialization against
our readiness, Context-default, utility-world, cancellation and event-ordering
contracts before batching independent requests. Any concurrency must fit existing
admission or add an explicit profile bound; no detached task or unbounded target
work queue. Fixture selection should use known ownership evidence rather than
probing unrelated Pages merely to find its own marker, without hiding failures
on the actual fixture. Neither change is implemented or declared verified here.

#### Paired-prerequisite implementation checkpoint

The subsequent implementation filters test-owned fixture URLs before probing
their unique title markers, including Worker handoffs. It also adds explicit
`with_parallel_session_prerequisites()` selection to the topology production
profile, selected by the Node host only. Frame-tree acquisition and Runtime
enable share one fixed two-future join after Page enable/Context defaults;
utility worlds, resume and PageReady still await both. Default consumers stay
serial. A second response expansion is declared, selection requires two pending
slots, and there is no new task, retry or deadline change. Deterministic tests
cover concurrency, both failures, owner cancellation and profile accounting.
Release/browser verification is recorded in `.work/node-paired-init.nHmvUx/`:
composition 207 passed, host 41 passed, strict Clippy/workspace checks passed;
the rebuilt addon passes lifecycle 44/44 and evaluation 38/38 and completes the
unchanged shared matrix at 12 matches / 43 unsupported / 2 oracle errors.
The command probe proves real overlap without unanswered requests. These fresh
regressions preserve older failures and do not close NODE-013's intermittent
root cause or claim full API/RSS parity. Next expand the public Locator API over
typed Rust capabilities; retain the three regression gates while replacing
unsupported adapter operations with actual implementations.

#### Three-way registration checkpoint (2026-09-07)

NODE-013 remains open. Fresh bounded traces show 32 ms for the standalone Rust
host and 3,029 ms with upstream Playwright simultaneously attached; all 28
commands reply. Delays occur across target-local initialization commands, not
one proven FFI deadlock. The same environment has a mostly idle Node event loop
and active Chromium renderer CPU, which is correlation, not a root-cause proof.

The Node profile now explicitly selects a fixed three-way join of Frame Tree,
Runtime enable and empty utility-world script registration. Context defaults,
world creation, recursive attachment, resume and publication retain their
barriers. Default consumers and the paired-only profile are unchanged. Three
pending slots and a third response expansion are required; no new task, retry
or timeout change. Record fresh release/browser gates in the
[canonical checkpoint](node-ffi-compatibility-design.md#node-013-three-way-registration-checkpoint-2026-09-07).
This does not complete public Node Locator.evaluate or overall compatibility.

#### Rust-backed Node Locator query/read checkpoint

VS3 now has immutable Rust-owned query values, Page/main-Frame bindings and
fourteen typed read operations. Ordinary CSS, nth/first/last, and/or, descendant
chaining and string/has/hasNot/visible filters use existing Rust selector
composition; no selector program or browser algorithm is assembled in Node.
Reads preserve strictness, missing attributes, collection order and fresh DOM
resolution. Actions, user-facing selectors, RegExp filters and the complete
Playwright selector language remain open; VS3 is not complete.

Query admission is independent from active browser work: 1,024 recipes/copies,
8 MiB total, depth 32 / 256 nodes / 64 KiB strings per recipe. Weak host/Frame
identity prevents cross-connection composition without keeping Rust browser
I/O alive. Reads reserve command copies and 6 MiB settlement allowance; count
and byte pressure, GC and Worker cleanup have explicit tests. The Locator
backend now reuses the command-free native-error classifier after live testing
exposed a closed-session error being reported as generic Backend. Narrow proof
closures become 10 and 18 packages; the Node host closure stays 221.

The first expanded comparison exposed same-Frame command serialization:
`title()` waited behind a pending Locator read, unlike Playwright. NODE-020
retains that ordinary comparison. Its subsequent scoped fix explicitly selects
eight target permits and eight DOM reads per Frame in the Node profile, within
unchanged shared command/byte limits. Other Rust consumers remain serial.
The original lifecycle rerun also captured a first-document window-size
discrepancy (NODE-014), which this scheduler change does not resolve.
Latest artifact hashes, full lane outcomes and preserved failed runs belong
in the [Node evidence record](node-ffi-compatibility-design.md).

The closed-session fix's rebuilt artifact passes lifecycle 44/44 and evaluation
38/38, with Locator 18/19 retaining NODE-020. The unchanged shared corpus now
has 22 matches, 33 unsupported cases and 2 oracle errors, with no value,
candidate, setup or cleanup errors. This is ten more matches, not completion
of VS3 or DT-079. No new commit or push was made.

The concurrency-stage addon passes the expanded Locator lane 22/22, including
the original NODE-020 comparison, evaluate creating a waited-for element,
independent same-Frame reads, and closing a Page with ten active/queued reads.
Lifecycle 44/44 and evaluation 38/38 also pass on that artifact; the completed
shared matrix remains 22 matches / 33 unsupported / 2 oracle errors, without
value, candidate, setup, or cleanup errors. Runtime tests
retain global admission, deadline, subtree and independent future-drop cleanup
proofs. This is a bounded concurrency checkpoint, not completion of selector,
action, arbitrary input/navigation ordering, or distribution work.

#### Rust-backed Node Locator action checkpoint

Thirteen public action entry points now reuse existing typed Rust operations;
the Node facade only validates shapes and the addon only converts bounded
values. Check/uncheck, clear and double-click compose over SetChecked, Fill
and typed 2.x Click respectively. The shared adapter replaces eight action stubs
without changing shared fixtures/specs/goldens; Page Keyboard and sequential
typing remain separate unsupported operations.

Native action admission is 6 MiB within the existing shared FFI ceiling, with
bounded strings/descriptors and explicit unpaired-UTF-16 rejection. Inspection
also moved Fill's focus/selection/insertion onto the existing Page input owner;
clear records Delete down/up and repairs failed release. Backend release tests
prove exclusion under a held input transaction and key-up repair.

The initial action artifact expands the unchanged shared corpus from 22 to 36
matches, with 19 unsupported cases and two oracle errors. The final
terminal-completion artifact completes 31 matches, 19 unsupported, two oracle
errors and five attachment setup errors; older matches do not replace these
failed setups. The fifth live lane preserves NODE-021's missing utility-world route after Enter
navigation and NODE-022's timeout modifier-state discrepancy. The latter exposes
Playwright retaining Shift where Rust's accepted cancellation contract releases
it; policy remains unresolved and the assertion remains ordinary/failing.
Regression also exposed NODE-023: ten cancelled Locator reads attempted ten
extra completion fences, overflowing the existing queue of eight and replacing
TargetClosed with ResourceExhausted. A reproduced exact error identifies this
source. Terminal results now skip redundant publication; other outcomes retain
their fence and its errors. Five safe-host tests cover the policy without new
dependencies or queue growth. Final full reruns pass Locator 22/22, evaluation
38/38 and lifecycle 44/44; actions remain 22/24. Artifact-specific results,
including failed runs and filtered diagnostics, belong in the Node evidence record.

The subsequent NODE-021 entry-readiness component is implemented in Rust and
selected only by the Node profile. It waits inside controlled admission and
the original deadline, with eight waiters, explicit resource leases, exact
world rechecks and lifecycle cancellation; defaults retain immediate
call-construction snapshots. Nineteen new tests bring composition coverage to
227 passing release tests (14 existing opt-in browser tests ignored).

Final addon `1c8ee9ea...d9df65` passes actions 25/26, Locator 22/22 and evaluation
38/38. Original Enter and consecutive-document comparisons pass; NODE-022
remains failing. Lifecycle completes 42/44, preserving a reverse-direction
NODE-014 outer-window observation and a new post-disconnect controller
navigation interruption (NODE-024). The separate bounded ordering diagnostic
passes three pairs but does not fix or replace that failing full gate.
Shared results are 36 matches / 19 unsupported / 2 oracle errors, with zero
value/candidate/setup/cleanup errors. Full artifact-specific evidence remains
in the [Node design](node-ffi-compatibility-design.md#world-readiness-artifact-evidence-2026-09-07).

VS3 and the full Node goal remain incomplete. Next isolate NODE-024 navigation
publication/loader ordering and NODE-014 native-window readiness without
silently retrying operations; decide NODE-022 compatibility policy separately
from accepted Rust input cleanup. Readiness after backend dispatch, remaining
selector/Handle/event/artifact coverage, narrow distributions and RSS
comparison are still open.

The semantic-query checkpoint exposes all seven `getBy*` constructors on Page,
main Frame and Locator through the existing typed Rust selector model. Strings,
RegExp source/flags, pinned role options and relative composition remain lazy
Rust query values, charged before copying. Six new host tests cover typed leaves,
regex flags, implicit test-id bytes, role errors and reservation return. No new
dependency or browser operation is added to the JS facade.

The expanded 32-case Locator lane caught two existing runtime differences:
content-permitted role names incorrectly preferred title, and global role-name
regex skipped matching siblings. Both have scoped fixes and mandatory paired
regressions. Runtime version 7 now supplies the symbol/version to all 22 Locator
script guards, replacing independently hardcoded versions. The final release
addon `edd8ba96...b0c9b7` passes Locator 32/32, evaluation 38/38 and lifecycle
45/45; actions remain 25/26 (NODE-022). The new caller-held response test proves
load navigation stays pending until response completion while independent
reads remain usable. NODE-014 and NODE-024 did not reproduce in this run;
neither is fixed by this selector work. Failed intermediate artifacts remain
in the [Node evidence record](node-ffi-compatibility-design.md).

The same final binary completes the unchanged shared corpus at 41 matches /
14 unsupported / 2 oracle errors, with no value/candidate/setup/cleanup errors
or native failure. Five new matches come from public text/role adaptation;
the gate still exits 1. Release units (100 Locator, 4 selector runtime, 68 host),
strict Clippy, workspace check, type/offline/boundary checks and 30 umbrella
profiles pass; Locator-only/composition closures remain 10/18 packages.

RegExp filters, Locator evaluation/Handles, child Frames, selector configuration,
complete accessible names, event/artifact APIs, narrow native distributions and
RSS comparison remain open. Shared `getByLabel` observation still needs
Locator.evaluate; public label queries are separately tested, not missing.

The next Locator.evaluate prerequisite now has independent Rust resolution and
continuation leaves (five/seven-package normal closures). They strictly select
one utility-world element into a unique CDP object group, reserve native/byte/
cleanup capacity before I/O, release on cancellation and fail closed after
failed/abandoned cleanup. Optional strict waiting now uses injected exact-world
routes, pacing and one original deadline/lifecycle signal. A separate shared
counter enforces waiter capacity before route lookup; stale selected elements
are released before retry. Release tests pass 15 backend / 32 composition cases;
two real Chrome tests verify semantic lookup, delayed hidden elements, document
replacement, timeout/close, detached-node identity and physical release.
This is not yet the Node method: pinned 1.58.2 resolves first, then
serializes source/arguments and evaluates in main world, with timeout applying
only to resolution. Production route integration, delayed conversion and paired evaluation
gates remain in [the focused design](knowledge/pwright-locator-evaluation.md).

Rebuilt Node addon `db8f7337...b7ea84`, after the CDP primitive extension, retains
32/32 Locator, 38/38 evaluation, 45/45 lifecycle, 25/26 actions and shared
41 matches / 14 unsupported / 2 oracle errors. No extra shared match is claimed:
the Node profile does not install the resolution leaves yet. NODE-022 and the
remaining selector/evaluation/distribution/RSS gates stay open.

The next independent 15-package leaf adopts that selected element into main
world, retaining both native wrappers in the original object group, and
consumes it for one Rust value evaluation without re-querying or retrying user
code. Its additional native/byte owner and continuation are admitted before
adoption and retained through group release. Release tests pass 24 cases and
one real Chrome test, including detached identity, main-world access, typed
values, JavaScript failure and cancelled/unpolled execution. Failed or
abandoned cleanup closes admission; zero local counters alone are not proof.

Shared CDP/Handle extensions required another Node build (`42a26fd0...94e94`).
All five full lanes finish: Locator 32/32, evaluation 38/38, lifecycle 44/45
(NODE-014), actions 25/26 (NODE-022), and unchanged shared 41/14/2. No additional
shared match is claimed. Node does not install the new leaf: exact production
Frame-world pairing/invalidation, controlled-runtime lifecycle, deferred FFI
conversion and paired public Locator.evaluate tests remain required.

Production pairing now has a separately installed `ChromiumFrameWorldAccess`
port over the actual atomic Frame route table. Pending and returned snapshots
share bounded admission, preserve all route resource owners, and use existing
readiness notifications for sticky invalidation. World routes are filtered by
the authoritative renderer session during OOPIF/swap transitions; late old-
renderer announcements cannot replace the current world. Release tests pass
40 topology / 247 composition cases, with 20 new snapshot cases and three new
topology cases. One real Chrome test proves main-world navigation, utility
isolation, child detach, actual old-world rejection and owned Context cleanup.
This completes the access-port prerequisite, not the public method: connecting
selection/adoption to that port and controlled-runtime/FFI phases remains open.

The shared topology change is rebuilt as `72644cf9...b9ddcd`. Serial regression
records Locator 32/32, evaluation 38/38, lifecycle 44/45 (NODE-014), actions
25/26 (NODE-022) and shared 41/14/2. An earlier overlapping package/shared run
on the same binary produced multiple attachment errors and 11 shared setup
failures. Both reports are retained; the serial experiment is not an
attachment-churn fix or full compatibility. See
[the artifact-specific checkpoint](node-ffi-compatibility-design.md#production-paired-world-access-checkpoint-2026-09-07).

The independent evaluation leaf now connects paired strict selection and
adoption under the original resolution deadline. Its retained route guard
follows native cleanup rather than dropping with a cancelled query/phase.
Generated async allocations are size-checked before boxing; strict/cleanup
errors remain terminal, and no user code is replayed. Release units pass
37 resolution / 41 evaluation cases; two serial Chrome cases include the actual
production paired port and delayed hidden/detached identity. The normal closure
remains fifteen packages. Node host assembly, controlled-runtime lifecycle,
session-retirement cleanup and deferred FFI conversion remain open; its existing
addon is unchanged. See [the preparation checkpoint](node-ffi-compatibility-design.md#paired-element-preparation-checkpoint-2026-09-07).

Preceding prerequisite: explicit `ControlledDispatcher::dispatch_with` shares
actual runtime state and policy with installed commands. Nine new unit cases
and a real Frame-lane preparation/timeout test pass. The adapter is test-only;
prepared-owner lifecycle and deferred Node conversion remain open. Rebuilt addon
`3ff763b2...7594ba` passes Locator 32/32 and evaluation 38/38, but lifecycle is
25/45, actions 0/26 and shared has 57 setup errors. Independent Rust probes
reproduce event-item exhaustion during Runtime.enable after over 1,000 execution-
context announcements (NODE-025). Preserve this failure and explicit caps;
investigate consumption/world retention before another full gate or RSS claim.
See [the detailed checkpoint](node-ffi-compatibility-design.md#controlled-preparation-dispatch-and-attachment-pressure-2026-09-07).

Preceding pressure checkpoint: Node explicitly selects conservative foreign-world
event admission before retained CDP leases/fanout. Default profiles and caps are
unchanged; six reader and seven classifier tests pass. A live Rust probe omits
1,440 irrelevant events, attaches in 381 ms and drains recorded resources on
close. Rebuilt `ab1a0857...6f7d9a` still fails full conformance: Locator 32/32,
evaluation 38/38, lifecycle 39 passes reported before its watchdog kill, actions
17/26 and shared 33 matches / 11 unsupported / 2 oracle errors / 11 setup errors.
Both oracle and candidate attach timeouts remain unclassified; NODE-022 persists.
The scoped NODE-025 implementation does not close native-world/RSS or public
Locator.evaluate integration gates. See
[the exact evidence and next work](node-ffi-compatibility-design.md#optional-world-event-admission-checkpoint-2026-09-07).

Latest host checkpoint: `HostLocator::evaluate_with` installs strict paired
selection/adoption and delayed conversion inside one controlled Frame command,
replacing the escaped prepared-owner/two-command handoff. The same runtime
governs all phases, with only the resolution timeout ending at preparation.
Nine new unit cases pass (77 host total); Chrome is 4/5, with a new NODE-026
native Context-close cleanup failure. The host now selects a 224-package normal
closure with explicit budgets; independent 5/7/15 and 30 umbrella proofs pass.
The rebuilt Node lanes are 32/32 Locator, 38/38 evaluation, 45/45 lifecycle,
25/26 actions and shared 41/14/2. No new public method or label match is claimed.
That checkpoint's NODE-026 failure is now fixed by optional bounded native
destruction receipts. Twelve new deterministic cases pass; the expanded six-crate
release run is 441 passing cases. Two complete real-host runs pass 5/5 each, plus
an isolated Context-close repeat. Proof clones retain snapshot admission, and
detach/error text never substitutes for acknowledged destruction. Final review
adds a connection-epoch guard and a reused-identity test (442 release cases,
thirteen new). The guarded host run remains 4/5 due to a 5 s attach setup failure
before queue assertions, not the repaired Context-cleanup case. An isolated
queue diagnostic passes, without replacing the failed complete gate.
The final guarded addon also records Locator 29/32, evaluation 17/38, lifecycle
watchdog after 11 passes / 5 failures, actions 25/26, shared 41/14/2. Thus the
whole goal remains non-green despite the scoped cleanup fix. Investigate the
remaining attachment/world-retention boundary without masking it by restart.
Next feature: admitted Node references,
delayed conversion/settlement and paired public timing/Worker tests. See
[the current record](node-ffi-compatibility-design.md#native-destruction-receipt-checkpoint-2026-09-07).

### DT-080: Node distribution composition

- Question: Does a full compatibility package collapse Rust's opt-in crate
  architecture?
- Decision: no. The differential package selects one explicit broad profile;
  the safe host must become feature-gated to produce narrower packages later.
  Independent native addons do not share live Rust objects.
- Gate: dependency proofs must show omitted capability crates are absent from a
  narrow build.
- Checkpoint: the current prototype still selects aggregate Chromium
  composition; title no longer depends on evaluation, which is now explicitly
  installed for public by-value `evaluate`.
  No narrow-host build or
  per-platform prebuilt distribution is claimed yet.

## Ordering and gates

```text
Evidence
  -> product boundary and measurable success
  -> public behavioral contract
  -> client/runtime and protocol semantics
  -> capability composition
  -> backend mapping
  -> operational limits and security
  -> conformance, versioning, and migration
  -> implementation proposal
```

- Gate A: DT-001 through DT-013 must finish before approving an architecture.
- Gate B: DT-020 through DT-035 must finish before approving protocol or API
  implementation.
- Gate C: DT-040 through DT-063 must finish before approving crate topology or
  a runtime vertical slice.
- Gate D: DT-070 through DT-072 must finish before implementation is treated as
  production-directed rather than exploratory.

## Decision record template

Every completed design task should produce a short decision record containing:

- Context and evidence
- Requirements and non-goals
- Alternatives considered
- Decision and rationale
- Consequences and risks
- Resource and compatibility impact
- Validation plan
- Unresolved questions
