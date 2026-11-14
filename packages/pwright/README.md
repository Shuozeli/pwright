<!-- agent-updated: 2026-10-21T03:12:15Z -->

An optional `official-conformance-page-check-light-v1` addon adds Rust-owned,
non-shadow-piercing `id:light` and `data-testid:light` page.$/$$ queries,
without enabling them in the default or prior page-check profile. Its fresh
source-bound run of the complete pinned
[`selectors-misc.spec.ts`](../../.work/official-conformance/results/core-selectors-misc21-light-candidate-20261021-v1.json)
passes 11/21, up from the frozen 10/21 candidate; the same Oracle remains
21/21. Only the open-shadow-root case changed. Ten candidate-only failures
remain. The later live Rust opt-in source adds `data-test-id:light` and
`data-test:light` with 92/92 enabled and 87/87 default-off release selector
tests, but the frozen v20 addon lacks them and there is no official browser
rerun yet. Locator/ElementHandle/evaluation operations and large browser-side
query resource bounds are not yet supported/proven.
The independently replayed immutable
[v20 candidate-only receipt](../../.work/official-conformance/outcomes/core-selectors-misc21-light-strict-paired-v20/promotion-receipt.json)
(SHA-256 `2889851a3db6106bb726018ade7b8f2f96e72852496d40123ea825a80f3e5bd9`)
changes one candidate row and no Oracle rows; 39 verifier/mutation tests pass.
At v20, Core totals were candidate 4,660 expected / 88 unexpected / 21
skipped / 43,708 not-run and Oracle 4,439 / 11 / 20 / 44,007.

The complete pinned Chromium Core `frame-frame-element.spec.ts` file now has
six unchanged source-bound cases per dedicated-CDP lane:
[candidate](../../.work/official-conformance/results/core-frame-frame-element6-candidate-20261021-v1.json)
0/6, [Oracle](../../.work/official-conformance/results/core-frame-frame-element6-oracle-20261021-v1.json)
6/6. Every candidate case first fails at the absent `Frame.frameElement()`
API; its identity, detached-frame, frameset, and shadow-root semantics have
not yet been exercised in Pwright. The independently verified immutable
[v19 paired receipt](../../.work/official-conformance/outcomes/core-frame-frame-element6-strict-paired-v19/promotion-receipt.json)
(SHA-256 `9303f3e34a31f15b92f9f834b734392c88b8589c5628a3a118ba379d3d92ab4b`)
changes only those six rows per lane. At v19, Core totals were candidate 4,659
expected / 89 unexpected / 21 skipped / 43,708 not-run; Oracle 4,439 / 11 /
20 / 44,007. This is not broad Playwright parity.
The new default-off `frame-element` profile uses a Rust-owned parent-session
DOM owner lookup and a thin `Frame.frameElement()` binding. Backend 5/5,
composition 4/4, Node adapter/contract 190/190, native-load, and TypeScript
gates pass. A source-consistent frozen addon has SHA-256
`440ec056a5cfceeacbf851c698ee24c33c3db063ad92262c215d996a568db6a0`.
The unchanged complete six-case candidate [first run](../../.work/official-conformance/results/core-frame-frame-element6-rust-owner-candidate-20261021-v1.json)
was 5/6 because frameset failed during `page.goto` before calling the new
method. Three identical-artifact full-file [v2](../../.work/official-conformance/results/core-frame-frame-element6-rust-owner-candidate-20261021-v2.json),
[v3](../../.work/official-conformance/results/core-frame-frame-element6-rust-owner-candidate-20261021-v3.json),
and [v4](../../.work/official-conformance/results/core-frame-frame-element6-rust-owner-candidate-20261021-v4.json)
runs were each 6/6; all provenance sidecars verify. The independently replayed
immutable [v21 candidate-only receipt](../../.work/official-conformance/outcomes/core-frame-frame-element6-rust-owner-strict-paired-v21/promotion-receipt.json)
(SHA-256 `3f717d16e9cdca75e601ecdcea8185c82df9c42a6c4beebb94cb67ea76f5324d`)
credits six latest green observations but retains the first pre-method failure
and unestablished root cause. It marks frameset unstable, with no parity claim;
47 verifier/mutation tests pass. At v21, Core totals were candidate 4,666
expected / 82 unexpected / 21 skipped / 43,708 not-run and Oracle 4,439 /
11 / 20 / 44,007.

The complete pinned 13-case `frame-hierarchy.spec.ts` file has a
[source/ID/asset-pinned manifest](../../tests/official/playwright-core-frame-hierarchy-public-chromium-v1-manifest.json)
and four passing source-only checker tests. The unchanged full-file
[candidate](../../.work/official-conformance/results/core-frame-hierarchy13-rust-owner-candidate-20261021-v1.json)
is 8/13 versus [Oracle](../../.work/official-conformance/results/core-frame-hierarchy13-oracle-20261021-v1.json)
13/13; both source-bound provenance sidecars verify. Four REDs have confirmed
missing opt-in frame-event exposure; the fifth closes at `page.goto` before
its event assertions, with cause still unproved. The independently replayed
immutable [v22 paired receipt](../../.work/official-conformance/outcomes/core-frame-hierarchy13-rust-owner-strict-paired-v22/promotion-receipt.json)
(SHA-256 `535148fa2b02b5af9e5519604cebef296f6e7daa1ff3c43dcba79139f933270a`)
changes only the 13 target rows per lane, preserves 48,464 other raw rows per
lane, and passes 48 verifier/mutation tests; v21 instability remains inherited.
Current v22 Core totals are candidate 4,674 expected / 87 unexpected / 21
skipped / 43,695 not-run and Oracle 4,452 / 11 / 20 / 43,994. This is not
frame-hierarchy parity.
A default-off `frame-events` bridge and separate `frame-detach-events` leaf
now compose with `frame-element` in the explicit
`official-conformance-page-frame-hierarchy` profile. Its corrected frozen
addon SHA-256 is
`1681505babfcf49f0e62233537f471af911fd73c247cef284ced648543c8c332`;
Node 225/225, host release 598 passed / eight ignored, and native load 1/1
pass. The first combined profile omitted `frame-element`: hierarchy 13/13,
frameElement 0/6 at the capability gate. On the corrected snapshot, unchanged
full-file attempts are hierarchy 10/13 then 12/13 and frameElement 5/6 then
6/6. All six raw provenance sidecars verify, but pre-assertion `TargetClosed`
and network-frame/topology teardown failures remain unstable. Four focused
release tests prove both independent-cursor orderings can leave a Request's
creation Frame available while live topology lacks it; which ordering/effect
caused each browser failure remains untraced. The Node facade now retains a
previously known canonical `Request.frame()` wrapper after detach without
new `Page.frames()` enumeration; its focused suite passes 87/87. This does
not fix network-before-topology publication or prove browser parity.
The v22 strict ledger is not superseded; no frame parity claim follows.

The complete pinned 11-case Chromium Library Context page-event file has a
[source/ID/asset manifest](../../tests/official/playwright-core-browsercontext-page-event-public-chromium-v1-manifest.json)
and 4/4 source-only checker. Two unchanged dedicated-CDP Oracle runs each
pass 6/11; the same frozen page-check candidate passes 10/11 then 9/11.
All four provenance sidecars verify. The opener timeout is shared, while a
candidate Ctrl-click timeout appears on the second run. The adapter's Oracle
is not all-green, candidate stability is unproved, and no v22 promotion or
full-file parity is claimed. The CDP endpoint became unresponsive after the
second candidate run and was left running pending recovery direction.

The complete pinned Chromium Core `selectors-misc.spec.ts` file ran all 21
unchanged cases once per source-bound dedicated-CDP lane: the
[candidate](../../.work/official-conformance/results/core-selectors-misc21-candidate-20261021-v1.json)
passes 10/21 and the
[Oracle](../../.work/official-conformance/results/core-selectors-misc21-oracle-20261021-v1.json)
21/21. Both provenance sidecars verify, with no skipped/flaky cases or
top-level errors. Eleven candidate-only REDs remain. The independently
verified immutable [v18 paired receipt](../../.work/official-conformance/outcomes/core-selectors-misc21-strict-paired-v18/promotion-receipt.json)
(SHA-256 `0bcdeef5540013619a32bc6e5eabf9f456b2b7195dd5f05d2b1bb3e17054c261`)
updates exactly 21 rows per lane, preserving 48,456 off-target raw rows per
lane byte-for-byte. At v18, Core totals were candidate 4,659 expected / 83
unexpected / 21 skipped / 43,714 not-run; Oracle 4,433 / 11 / 20 / 44,013.
The adapted CDP result is not broad Playwright parity.

The complete pinned Chromium Core `selectors-css.spec.ts` file now has
source-bound dedicated-CDP raw results: frozen clean candidate 14/27,
Playwright Oracle 27/27, with all 27 unchanged cases executed once per lane.
The [candidate](../../.work/official-conformance/results/core-selectors-css27-candidate-20261021-v1.json)
and [Oracle](../../.work/official-conformance/results/core-selectors-css27-oracle-20261021-v1.json)
provenance sidecars verify; 13 candidate-only REDs remain. The independently
verified immutable
[v17 paired receipt](../../.work/official-conformance/outcomes/core-selectors-css27-strict-paired-v17/promotion-receipt.json)
updates exactly 27 rows per lane and preserves 48,450 other raw rows per
lane. At v17, Core totals were candidate 4,649 expected / 72 unexpected / 21
skipped / 43,735 not-run; Oracle 4,412 / 11 / 20 / 44,034. The CDP fixture is
adapted and does not establish broad parity.

An evaluation cleanup change now skips `releaseObjectGroup` only when CDP
returns an inline string with no remote object ID or exception details; the
retained/exception paths still await cleanup. The clean release addon passes
21 Rust and 15 Node tests, and the unchanged complete Core dialog file passes
[10/10 in a source-bound candidate rerun](../../.work/official-conformance/results/core-page-dialog10-candidate-sync-clean-20261021-v1.json).
The historical 9/10 candidate RED and 10/10 Oracle in the v15 receipt remain
unchanged. The independently verified immutable
[v16 candidate-only successor](../../.work/official-conformance/outcomes/core-page-dialog10-strict-paired-v16/promotion-receipt.json)
changes only that candidate RED row, preserving 48,476 other candidate rows
and all Oracle JSONL/summary bytes. At v16, Core totals were candidate 4,635
expected / 59 unexpected / 21 skipped / 43,762 not-run; Oracle 4,385 / 11 /
20 / 44,061. This narrow dialog result does not close cancellation/object-group
memory bounds or prove full-corpus compatibility.

The complete pinned Chromium Core `page-dialog.spec.ts` file now has a strict
paired candidate/Oracle result: frozen candidate 9/10, Oracle 10/10. The
candidate-only RED is a 30-second open-alert `page.evaluate` timeout, not a
verified context-close failure. The independently verified immutable
[v15 receipt](../../.work/official-conformance/outcomes/core-page-dialog10-strict-paired-v15/promotion-receipt.json)
preserves 48,467 off-target raw rows per lane; cumulative Core totals are
candidate 4,634 expected / 60 unexpected / 21 skipped / 43,762 not-run and
Oracle 4,385 / 11 / 20 / 44,061. This adapted CDP result does not prove
unadapted public-API parity or full-corpus execution. The `frame-events`
release frame-tree unit gate exits zero with 80 passed and two dedicated-Chrome
tests ignored.

The optional `history-navigation` profile now has a Node
`Page.goForward()` adapter over the new Rust `GoForward` command, alongside
the existing `Page.goBack()`. Focused browserless Node adapter/contract tests
pass 220/220 and TypeScript accepts the API. The release native addon builds
under both isolated `history-navigation` and full
`official-conformance-page-check` profiles. The unchanged upstream Core
HistoryAPI case, including `goForward`, passes in both source-bound
[candidate](../../.work/official-conformance/go-forward-abi129-browser/candidate-history-api.json)
and [Oracle](../../.work/official-conformance/go-forward-abi129-browser/oracle-history-api.json)
Chromium lanes, 1/1 each. The cross-document `@smoke` case fails in both
lanes at different steps in that earlier focused run. A later pinned full
`page-history.spec.ts` Chromium run covers all 16 unchanged cases in each
lane: candidate 11/16, Oracle 13/16; in this run `@smoke` passes only in the
candidate. The immutable [v14 paired receipt](../../.work/official-conformance/outcomes/core-page-history16-strict-paired-v14/promotion-receipt.json)
preserves all 48,461 off-target raw rows per lane and records three
candidate-only REDs, two both-lane REDs, and one Oracle-only RED. Its adapted
CDP fixture is not proof of unadapted or complete history parity.

The separate ABI129 Python binding passes the unchanged official history-API
case 1/1 on an installed wheel, with its
[strict focused receipt](../../.work/official-conformance/outcomes/python-navigation-history-api-chromium-strict-paired-v4/promotion-receipt.json).

The default-off `fill-navigation-commit` capability, selected by
`official-conformance-page-fill-navigation-commit`, now passes the complete
pinned Chromium Core `Page.fill` file: 45/45 in a frozen candidate and 45/45
in the Playwright Oracle. The immutable
[v13 paired receipt](../../.work/official-conformance/outcomes/core-page-fill45-strict-paired-v13/promotion-receipt.json)
preserves the prior 44/45 candidate RED and changes only its navigation-context
row after the no-binding-source publication-drain guard. All other candidate
and Oracle ledger rows remain unchanged. Both lanes adapt launch to
`connectOverCDP` on dedicated Chrome 155, so this is bounded browser execution,
not unadapted or full Playwright public-API parity. The guard remains global;
cross-Page Page-binding scoping still needs repair. See the
[conformance record](../../docs/upstream-playwright-conformance.md).

The default-off `select-option-navigation-commit` feature is available alone
or through `official-conformance-page-select-option-navigation-commit`, which
adds it to the existing full select-option conformance profile without
changing older profiles. A successful select may trigger navigation during
`input`/`change`; the host now skips the post-action Page-binding publication
evaluation when no binding is registered, pending, or early. The complete
pinned 32-case official Core Chromium `Page.selectOption` file passes 32/32
with a frozen candidate and 32/32 with the Playwright Oracle. The immutable
[v12 paired receipt](../../.work/official-conformance/outcomes/core-page-select-option32-strict-paired-v12/promotion-receipt.json)
records one candidate RED-to-GREEN successor to historical v11, not a
rewrite of that evidence. Both lanes adapt the launch fixture to
`connectOverCDP`, so this proves a bounded browser-backed slice, not
unadapted or complete Playwright public-API parity. The binding-source guard
is global; a binding on another Page may still cause an unwanted evaluation
on a navigating Page. See the [conformance record](../../docs/upstream-playwright-conformance.md).

The default-off `bidi-startup-failure` profile is a failure-only diagnostic
for `BrowserType.launch()` with an explicitly supplied absolute executable.
It selects only `bidi-chromium`, `bidi-chrome-canary`,
`bidi-chrome-stable`, `moz-firefox`, `moz-firefox-beta`, and
`moz-firefox-nightly`. Build it with
`CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 PWRIGHT_NATIVE_PROFILE=bidi-startup-failure pnpm run build`
and select the same profile at runtime. Its Rust observer creates an owned
temporary profile, watches the child for at most two seconds with 3 KiB of
combined logs, and kills/reaps its process group before returning a launch
failure. It accepts inherited `firefoxUserPrefs` on this narrow route but
does not apply them or establish a browser protocol. It cannot return a
usable Browser. Local Rust leaf release tests pass 8/8, the Node adapter
46/46, native integration 3/3, native ABI/load smoke, full browserless package
829/829, strict TypeScript, and focused addon Clippy. Full-dependency strict
Clippy remains blocked by two pre-existing `pwright-node-host`
`too_many_arguments` warnings. The expanded package-boundary check passes
locally: the feature adds only its addon marker and browser-neutral startup
leaf beyond `playwright-trace`, without changing older profiles. The
selected-profile CI step has not run remotely. The six unchanged official
`browsertype-launch.spec.ts:64` assertions were rerun in isolated candidate
and Playwright oracle lanes: the
[strict v3 capture](../../.work/official-conformance/results/core-dummy-node-bidi-strict-v3/)
has 12/12 single-attempt `passed`/`expected` outcomes with real exit-1 dummy
Node descendants and profile cleanup. This is adapted, failure-only public
method evidence; it does not establish a usable Browser, browser execution,
broad public-API parity, or WebDriver BiDi protocol parity. The v9 ledger
still records the historical frozen candidate as RED. Its immutable
[v10 successor](../../.work/official-conformance/outcomes/core-dummy-node-bidi-strict-paired-v10/promotion-receipt.json)
is now published and independently verified: exactly six candidate rows move
from `unexpected` to `expected`, while the other 48,471 raw rows per lane
remain unchanged. The v10 receipt explicitly denies Browser, broad public-API,
and BiDi-protocol parity. Its clean-CI tooling gate passes 35 pure tests
locally, but cannot replay ignored evidence; remote CI has not run. See the
[conformance ledger](../../docs/upstream-playwright-conformance.md).

The default-off `playwright-server-connect-session` profile now creates a
supervised Playwright Server Browser in Rust and exposes a narrow Node
Context/Page lifecycle: empty-option `Browser.newContext`,
`BrowserContext.newPage`, `Browser.newPage`, and explicit `BrowserContext.close`.
An implicitly owned Page closes its Context on `Page.close`; a Page created on
an explicit Context still rejects `Page.close`. Its graph-backed wrappers reject disposed or recycled
GUIDs; Rust revalidates parent generations after creation replies. Selected
release build, strict Clippy, native load, native-class
boundary, TypeScript, 18 adapter tests, and four earlier Tailscale-bound synthetic
HTTP-to-WebSocket native tests pass, including a 160-event burst that leaves
the remote Browser connected. One additional selected real-addon Tailscale
fixture passes explicit Context close and implicit-Context Page close 2/2,
leaving Browser connected. The binding drops unsupported ordinary protocol
events after Rust validation and graph projection; it does not expose an event
stream. On whole-session termination it emits Page `close`, Context `close`,
then Browser `disconnected` once. The pinned Core line-382 fixture-adapted
[RED](../../.work/official-conformance/results/core-connect-child-close-synthetic-v1-red-v1/attestation.json)
and [GREEN](../../.work/official-conformance/results/core-connect-child-close-synthetic-v1-green-v1/attestation.json)
captures establish this narrow behavior, not official ledger credit.
Context-owned Page close, navigation/evaluation, rich options,
arbitrary externally initiated child-close notifications, and externally
created object wrappers remain unsupported. A separately verified,
fixture-adapted Core lifecycle diagnostic passes two unchanged assertions in
each Pwright/oracle lane; it is not an unchanged official-server pass or
ledger credit. See the
[boundary record](../../docs/knowledge/pwright-playwright-server-connect.md).
The separate Core HTTP-redirect timeout [v1 RED](../../.work/official-conformance/results/core-server-connect-redirect-timeout-strict-v1/core-server-connect-redirect-timeout-strict-v1-second-20261012/attestation.json)
and independently verified [v2 GREEN](../../.work/official-conformance/results/core-server-connect-redirect-timeout-strict-v2/core-server-connect-redirect-timeout-strict-v2-fixed-20261012/attestation.json)
diagnostics move the candidate from 0/9 to 9/9 against oracle 9/9 on unchanged
assertions. HTTP(S) discovery defers reserved WebSocket header rejection until
before upgrade; direct WS(S) still rejects before I/O. This fixture-adapted
internal test earns no official ledger credit.
The historical independent [remote-Page waiter RED](../../.work/official-conformance/results/core-connect-waiters-synthetic-v1-red-v5/attestation.json)
selects three unchanged Core assertions: oracle 3/3, candidate 0/3 after
successful Context/Page creation. The candidate fails at its first missing
waiter method; none of its assertions or close-order checks is reached.
This browser-free, server-fixture-adapted internal diagnostic does not grant
official ledger credit.
The default-off Rust
[`pwright-playwright-server-page-events`](../../crates/pwright-playwright-server-page-events/src/lib.rs)
leaf now projects bounded, typed Page-attributed events and terminal notices
(16/16 release tests). The existing `playwright-server-connect-session` Node
profile still drops ordinary remote events. A separate
`playwright-server-connect-page-events` profile composes the selected Rust
session projector, typed Node notices, generation-fenced Page identity and
acknowledged request/response subscriptions; its addon builds and passes a
native-load check. The JS adapter passes 25/25 focused tests, including early
close-publication and terminal-predicate regressions; the package suite now
includes it and passes 816/816. Strict TypeScript checks pass. Rust rejects
command sends to a target GUID recycle already observed by its actor, but not
an unseen remote race. The independently verified
[v5 GREEN diagnostic](../../.work/official-conformance/results/core-connect-page-events-synthetic-v2-green-20261020-v5/attestation.json)
passes those three unchanged Core teardown assertions 3/3 in both oracle and
real-addon candidate lanes, plus a separate positive Request→Response→Download
waiter and Page-listener probe in each lane. Its server launcher is replaced
by a browser-free synthetic fixture: this is not real Playwright Server/browser
or full public-API parity proof, and no official ledger row is promoted.
The earlier Core waiter RED remains historical evidence.

The default-off `cdp-handshake-headers` profile passes bounded custom headers
through the Node facade and N-API into Rust-owned CDP HTTP discovery and
WebSocket upgrade. Build it with
`PWRIGHT_NATIVE_PROFILE=cdp-handshake-headers CARGO_BUILD_JOBS=1 CARGO_NET_OFFLINE=true pnpm run build`
and select the same profile at runtime. The release addon at
`native/cdp-handshake-headers/pwright.node` has SHA-256
`82b11e24df70eec254aaa4e751dfa978422265a9ae517932eb89c781160fabc9`;
native load passes 1/1, facade adapter checks 10/10, and
`pnpm run test:cdp-handshake-headers-native` checks the real bounded N-API
path 3/3 without a browser. The normal Node browserless suite passes 783/783,
strict TypeScript passes, and the selected profile adds no normal dependency
packages. This is not official Core browser evidence; at that header-only
checkpoint, default WebSocket `User-Agent` compatibility remained open.
The complete Page-plus-headers profile
`official-conformance-page-cdp-handshake-headers` has a release artifact at
`native/official-conformance-page-cdp-handshake-headers/pwright.node` (SHA-256
`dfe9607df2c7cfa16d304338e88eb2fa01302b4736e4116c0c67b655a3482769`).
Its selected and default browserless suites each pass 783/783, strict
TypeScript passes, native load passes 1/1, and the header N-API boundary
passes 3/3. The 39-feature selected composition has no additional normal
Cargo packages beyond the 338-package Page base. No official Core test
result is implied by these local checks. A separate strict, browser-free run
of two unchanged upstream Core header assertions passed the explicit WS/HTTP
case in both lanes and found an earlier real candidate failure in the
default WebSocket `User-Agent` (received `undefined`; oracle passed). The
paired result is
recorded in `../../.work/official-conformance/outcomes/core-cdp-handshake-headers-strict-paired-v1/`.
Two more unchanged official, browser-free CDP discovery URL-query assertions
pass 2/2 in both lanes; their later strict successor is at
`../../.work/official-conformance/outcomes/core-cdp-url-query-strict-paired-v1/`.
The opt-in `browser-registry` profile's public `BrowserType.executablePath()`
also passes the unchanged official Chromium, Firefox and WebKit assertions
3/3 in each independent lane with the current browser cache. A versioned
official-test adapter supplies only the upstream fixture's private
`_playwright` backpointer; the path policy remains native. The strict
paired successor, which changes exactly those three prior unexpected rows,
is `../../.work/official-conformance/outcomes/core-browser-type-owner-strict-paired-v2/`.
Its original source-ledger classification is preserved, but the corrected
invocation classification is `implementation-internal`/`private-client-access`
because the official fixture needs that private backpointer. The separate
immutable seal at
`../../.work/official-conformance/outcomes/core-browser-type-owner-strict-paired-v2-seal-v1/attestation.json`
pins the verifier and promoter source hashes and records
`publicApiParityEvidence=false`; the 3/3 result is not unadapted public API
parity or browser-launch proof. The separate seal-v1 attestor has manual Ruff
`EXE001`/`B023` debt and must be versioned and resealed before a fix.
The default-off `cdp-default-user-agent` Rust capability now adds Playwright
1.58.0's default only when no case-insensitive explicit `User-Agent` is
present, preserving even an explicit empty value. The tested Page composition
is `official-conformance-page-cdp-default-user-agent`; its release addon at
`native/official-conformance-page-cdp-default-user-agent/pwright.node` has
SHA-256 `92f039f5530075841ca7d59ff523c1f6b7a5ec4884a29ff20042ef3c95826fb2`.
Focused Rust tests pass 7/7, real N-API boundary tests 4/4, selected and
default browserless suites each pass 788/788, strict TypeScript and release
checks pass, and normal Cargo packages remain 419 versus 419 for the prior
Page-plus-header composition. The newer strict, browser-free official Core
`connect-over-cdp.spec.ts:299` test passes 1/1 in both candidate and oracle;
the candidate target moves from `unexpected` to `expected` in the latest
48,477-row paired bundle at
`../../.work/official-conformance/outcomes/core-cdp-default-user-agent-strict-paired-v2/`.
Candidate is **4,538 expected / 53 unexpected / 11 skipped / 43,875 not run**
(SHA-256 `b69852a0f582958f563cecdf055bda37373a3e222378fb4efdb1cea98ac79824`);
oracle is **4,285 expected / eight unexpected / ten skipped / 44,174 not run**
(SHA-256 `4198bf312aea62aada3e5f99f0eeb2daca13c980657d01fb81c1141a915b08a8`).
The original UA verifier now rejects the evolved live package, while a
separate [UA snapshot seal](../../.work/official-conformance/outcomes/core-cdp-default-user-agent-strict-paired-v2-seal-v1/attestation.json)
(SHA-256 `e19b9a8821981cb6f9179de210a43696d34b7c2cf3ccced9ee381a5574370cff`)
reverified the unchanged UA sidecars and ledgers against the frozen
capture-time candidate source.
Only the fixture URL host changed to the same-port full Tailscale MagicDNS
name; the assertion did not change and no Chrome ran. The private-helper
import keeps the row `implementation-internal`, with
`publicApiParityEvidence=false`; this is not full conformance or unadapted
public API parity. The UA verifier/promoter suite passes 21/21 and is in CI;
the promoter is Ruff lint-clean but its sealed source has one manual
`ruff format --check` failure pending versioned recapture.

The default-off `cdp-discovery-errors` and `cdp-ws-close-reason` Rust features
now compose into `official-conformance-page-cdp-errors`. Its release addon at
`native/official-conformance-page-cdp-errors/pwright.node` has SHA-256
`0d7dcb931c42d5e912b594ec20fd12e202cc76590c7cab93187fe6829388f5c6`.
The post-fix browser-free official Core CDP error slice reruns unchanged
`connect-over-cdp.spec.ts` assertions at lines 399, 411 and 572. Both the
independent Playwright 1.58.0 oracle and Pwright candidate pass **3/3**;
the candidate ran from a frozen package snapshot. The strict v3
[candidate sidecar](../../.work/official-conformance/results/core-cdp-error-slice-strict-v3/candidate-v1.strict.json)
and [oracle sidecar](../../.work/official-conformance/results/core-cdp-error-slice-strict-v3/oracle-v1.strict.json)
pass independent verification. A separate [snapshot seal](../../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v3-seal-v1/attestation.json)
(SHA-256 `87d852a37096b7b287c7c9c3090f4d46704deb8ffd8e2de93eb70602f4143841`)
replays the unchanged pair without relying on the mutable live addon path.
The immutable [paired receipt](../../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v3/promotion-receipt.json)
has SHA-256 `1f65ad5e2d26ec874fa7c386439c2ef28be58335233cfd5bf4d1f397356d555d`.
Only the three target rows change per lane from v2; the other 48,474 of
48,477 rows remain byte-identical. Candidate is **4,541 expected / 53
unexpected / 11 skipped / 43,872 not run** (ledger SHA-256
`400f1e29f84da3f83076bd97d89ebcf76f20ae4cec48e5c518337fb1f7c56736`);
oracle is **4,288 expected / eight unexpected / ten skipped / 44,171 not
run** (ledger SHA-256
`4c4a397cd098fd28e42b4e579217d808a034b8657a21e09228a1a7fd9dec844b`).
The source classification remains `implementation-internal`, and the receipt
records `publicApiParityEvidence=false`: no full Core or public API parity is
implied. Host Rust tests pass 14/14 plus two WebSocket close-diagnostic tests;
selected/default browserless Node suites pass 791/791 each, native-load and
TypeScript pass, and the addon package-name closure is 287 versus 287.

The historical red v2 Core CDP error slice runs three unchanged official
`connect-over-cdp.spec.ts` assertions at lines 399, 411 and 572. Candidate
fails all three genuinely, while the independent Playwright 1.58.0 oracle
passes all three; the strict v2 [candidate sidecar](../../.work/official-conformance/results/core-cdp-error-slice-strict-v2/candidate-v1.strict.json)
and [oracle sidecar](../../.work/official-conformance/results/core-cdp-error-slice-strict-v2/oracle-v1.strict.json)
were independently verified. The v2 immutable 48,477-row successor has a
[candidate ledger](../../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v2/candidate.jsonl)
at **4,538 expected / 56 unexpected / 11 skipped / 43,872 not run**
(SHA-256 `c6304c593856a8a94b24ae173ea385609016139989dd079fc4ef4878f811facd`)
and an [oracle ledger](../../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v2/oracle.jsonl)
at **4,288 expected / eight unexpected / ten skipped / 44,171 not run**
(SHA-256 `6478b726429f40bec7d8c22f943442c57345f246070f2c384386ba33cff376d2`).
Only those three previously `not-run` rows change per lane; all other 48,474
rows match the preceding UA v2 ledgers byte-for-byte. The
[promotion receipt](../../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v2/promotion-receipt.json)
has SHA-256 `92c6720100cfbecdef6ad36e3bd8fdaf3f327251b4200f83dafeb0ef893e4cb4`.
The original `implementation-internal` classifications remain, with
`publicApiParityEvidence=false`; these three discrepancies are not full
conformance or unadapted public API parity. The earlier UA v2 checkpoint
above remains historical.
The original v2 verifier now rejects its historical candidate-source hash
against the newer live package. A separate [capture-time source seal](../../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v2-seal-v1/attestation.json)
(SHA-256 `65e96b04c6e1cc0edcad15c63dd4ae67ecb4fca9441716a390de52016fef6d7b`)
reverifies the original reports, ledgers and receipt against a frozen package
snapshot with the exact recorded source digest. Its [attestor](../../scripts/attest_playwright_core_cdp_error_slice_strict_v2_snapshot.py)
passes along with four snapshot-tamper controls. The post-fix v3 run above
uses a frozen candidate package snapshot.

The companion Python ABI121 browser-free dummy-engine successor is at
`../../.work/official-conformance/outcomes/python-dummy-engine-abi121-strict-paired-v1/`:
four unchanged Firefox/WebKit async/sync launch-failure cases pass **4/4** in
the official oracle and **0/4** in the candidate. The candidate's four
`UnsupportedError` results remain failures, not skips. The 6,526-row paired
ledgers are candidate **1,529 passed / 109 failed / three errors / 11 skipped /
4,874 not run** (SHA-256 `dd0609e85a704f97c55762587126c0790d80a444bc116cc9f7e3f55c8d0efb9d`)
and oracle **1,303 passed / four failed / 11 skipped / 5,208 not run**
(SHA-256 `f15b5292a64e27607bc632074fcac26457f879decf569ff8eb616435588e9b95`).
The new dummy-engine evidence helpers have an open manual Ruff
`EXE001`/`I001`/`F401` debt; the hash-pinned capture sources cannot be edited
in place without recapture.

The default-off `cdp-session` profile exposes Rust-owned auxiliary Chromium
Page/main-Frame CDP sessions through the Node N-API and JS facade. A separate
default-off `browser-cdp-session` feature now source-implements independent
Browser-root CDP sessions, sharing the bounded native registry but not Page
ownership. Its `Target.targetCreated` fence includes first-match-wins CDP
discovery filters that select Page targets, and JS post-create event-flush
failures compensate the new Page or Context. Selected release host source and
test compilation pass; focused CDP host release tests pass 32/32. The
topology-driver release test target remains unverified after a memory-pressure
interruption. Official browser behavior also remains unverified. The normal
browserless Node list passes 773/773, including four
new post-create cleanup cases, with strict TypeScript. Both CDP leaf release
checks and their zero-added-package dependency proof pass on the current
source; a combined native artifact still must be rebuilt.

The official Core session fixture requires `init-scripts`, so the separate
`official-conformance-page-cdp-session` profile composes both CDP leaves onto
the existing Page test profile without expanding the standalone leaf. The
earlier release artifact at
`native/official-conformance-page-cdp-session/pwright.node` has SHA-256
`958155e45ca6162e05f2d6f13d4a13f6d76953af7658d8b1677bc6ce8c68ec9a`;
native-load smoke, 763/763 browserless tests, strict TypeScript, and the
zero-added-package dependency check passed at that older checkpoint. It does
not attest the new Browser-root feature and must be rebuilt. The 12 unchanged
official Core CDPSession cases have only been collected, not run in a browser.
`Page.coverage` is still missing from that file's public surface.

Rust selector planning now source-implements the typed CSS/text `*` capture
used by four pinned Core `$eval`/`$$eval` cases. It preserves the capture
boundary across later filters and descendant Locators, then materializes the
matching ancestor with deduplication. The current selected release host source
and test compilation pass with handles and selector registration; the focused
selector module passes 89/89 release tests. A rebuilt native artifact and unchanged
official browser cases remain pending. No ledger credit is claimed.

The selected `handles` profile now exposes `Page.waitForSelector()` and
`Frame.waitForSelector()` through a Rust-owned Locator wait and optional
retained ElementHandle publication. It supports attached, detached, visible,
and hidden states, selector strictness, and inherited Page/Context timeouts;
hidden/detached return `null`. Typed and Browser-program selectors (including
XPath) use a single state-and-element observation before positive-state Handle
publication. Browserless Node package tests pass **738/738**
and strict TypeScript checks pass. The cumulative release addon passes its
native-load smoke (SHA-256
`905b9b7dd8bc171e0aab2d401899730da640854d7f10d22084d6ac6dc676c533`).
It limits simultaneous selector waits to 24 per native environment, reserving
eight of 32 FFI Command slots for operations that may unblock them. Focused
release quota tests pass 3/3; browser-backed liveness remains unverified.
The pinned official Chromium
`page-wait-for-selector-1` file has not been rerun on this source, so its
17 candidate rows retain their earlier outcomes. In particular,
Playwright CSS `:visible`/`:nth-match`, Registered/Layout selector plans, and
full upstream element-preview history are not provided by this entrypoint.
The separate opt-in
`element-handle-wait-for-selector` leaf now scopes a wait to the exact retained
Handle and inherits its creation-world Page timeout, but has no official
browser pass credit yet.
For `Frame.waitForSelector`, a confirmed detached Frame now maps the known
CDP execution-context-loss errors to Playwright's `Frame was detached` error;
active/unknown Frames and Page calls retain their original errors. Browserless
Node tests pass **738/738** with the cumulative source changes; the official browser
case remains unverified.

The independent opt-in `element-handle-scroll-into-view` profile adds
`ElementHandle.scrollIntoViewIfNeeded({ timeout })` without selecting the
larger DOM Handle feature. Node forwards the original retained Handle to
Rust; it never re-queries a selector. An omitted timeout now resolves from
the owner Page/Context's Rust-held default, while explicit zero disables the
deadline. The selected native binary must carry its scroll-timeout-inheritance
marker and method; an unselected binary fails closed.
`official-conformance-page-retained-scroll` composes this one feature
onto the existing full Page profile without changing that profile. Browserless
package tests pass **722/722**, strict types and artifact
contract checks pass. The isolated release addon builds and its native-load
smoke plus focused contract tests pass; an official browser rerun is still
required. These checks confer no conformance credit.

The opt-in `element-handle-dom` profile now exposes `ElementHandle.focus()`
through the existing Rust exact-Handle focus operation. The Node wrapper
forwards the retained native owner without constructing a selector, and an
unselected profile fails explicitly. This initial surface accepts no options;
passing options fails closed until the timeout/no-wait semantics are
implemented. All **693/693** browserless Node package tests, strict types,
and selected-addon release check / Clippy pass. The rebuilt full-Page profile
includes this leaf, but the official Core `elementhandle-misc` focus case still
needs a browser-backed candidate rerun; no strict-ledger credit is claimed.

The separate opt-in `element-handle-select-option-value` profile exposes only
`ElementHandle.selectOption(value: string, { timeout? })`. Rust evaluates the
original retained Handle, waits for a visible/enabled `<select>` and matching
enabled value or label, then dispatches `input` followed by `change` and
returns the selected value. It never reconstructs a selector or follows a
replacement element. Inputs and selected output are limited to 64 KiB UTF-8;
the target may have at most 4,096 options. The operation shares the bounded
selector-wait admission and inherits the owner's Page/Context timeout when
omitted. Descriptor, option-Handle, `force`, and navigation-commit semantics
are **not** implemented and must not be claimed as Playwright parity. The
`official-conformance-page-retained-select-option-value` profile composes this
leaf onto the retained-wait Page profile. Browserless Node tests pass 738/738
and strict types and dependency-boundary checks pass. The cumulative release
addon builds and passes its isolated native-load smoke; its SHA-256 is
`bae00851f127108b003264f46a4ea8982a9ee4ad1b3de1ab3b70ef2335faaa06`.
Focused release Rust tests pass 5/5. An official browser rerun is still
required, and these checks confer no conformance credit.

The opt-in `official-conformance-page-goto-burst-20` native profile builds
the existing 38-feature official Core Page union plus only
`frame-navigation-burst-20`. From this package directory, run
`PWRIGHT_NATIVE_PROFILE=official-conformance-page-goto-burst-20 pnpm run build`
and select the same profile at runtime; `pnpm run test:native-load` checks its
ABI, capability markers and retained dispatch port without a browser. The
release addon SHA-256 is
`3dfb2ac8698de70f5b2416688b2d15de879251d9d7bf5c5549474ecb9fbd5c83`.
The separate official and burst profiles retain their existing feature sets
and artifact paths. Browserless package units pass 565/565, strict TypeScript
and dependency-boundary checks pass, and the composite and both prior
artifacts pass native-load 1/1 each. The pinned 64-case Page.goto file has
not been rerun with this composite artifact.

The opt-in `playwright-trace` native profile adds only the Playwright
`tracesDir` launch-directory boundary: a nonempty caller path is prepared in
Rust before executable validation, profile allocation or process startup,
and is not deleted after a failed launch. Build/select it with
`PWRIGHT_NATIVE_PROFILE=playwright-trace CARGO_INCREMENTAL=0 pnpm run build`,
then use the same profile when importing the package. The default and legacy
`owned-launch` profiles stay unchanged. A browserless native suite verifies
the directory and cleanup behavior (4/4); the pinned Core dummy-process
diagnostic passes 1/1 in both official and selected candidate lanes, but is
not strict-ledger parity evidence. Context trace start/stop, `.network` and
ZIP output are not implemented. See [the conformance record](../../docs/upstream-playwright-conformance.md).

The new opt-in `persistent-context-preflight` profile moves ordered
certificate-shape and browser-argument decisions into the independent,
dependency-free `pwright-persistent-context-preflight` Rust leaf. JavaScript
still performs `Promise.all` certificate file reads, boxed-string conversion,
truthiness mapping and public `Error` construction. Build it with
`CARGO_BUILD_JOBS=2 PWRIGHT_NATIVE_PROFILE=persistent-context-preflight pnpm run build`,
then use the same profile when importing. The default profile retains its
existing JS preflight; no successful persistent Context or browser launch is
claimed. Default and opt-in profile package tests each pass 512/512, the real
native bridge passes 4/4, and the pinned v1.58.0 browserless public-method
comparison covers 16 rejection and three accepted-preflight inputs in the
opt-in profile. Classic Firefox now rejects `-juggler*` in that Rust profile;
`moz-*` BiDi Firefox does not. The default JS profile deliberately retains
its earlier behavior, including its remaining `-juggler*` discrepancy. The
dependency proof adds only this pure Rust leaf; the addon still carries its
pre-existing host tree.

Current checkpoint (2026-10-10): `firefox` and `webkit` expose `name()` beside
`chromium`; the selected `browser-registry` profile also exposes synchronous
`BrowserType.executablePath()` for all three engines using Rust-owned expected
path metadata. It does not install or launch a browser. Build/select it with
`CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 PWRIGHT_NATIVE_PROFILE=browser-registry pnpm run build`,
then the same profile when importing; `pnpm run test:browser-registry-native` checks its native path
surface. The opt-in `owned-launch` profile supports fresh Chromium
`BrowserType.launch()` only with an explicit existing executable and a
temporary profile. Firefox/WebKit positive launch, valid
`launchPersistentContext()`, `launchServer()` and `connect()` remain unsupported;
pre-spawn rejection paths now include persistent-Context certificate and
engine-argument validation in the default JS or opt-in Rust leaf. Invalid inputs can match
Playwright before acquisition; valid persistent launch still stops at the
unsupported boundary. Default CDP attach is separate.
Each of default, owned-launch and browser-registry passes 512/512 browserless
package units, with type and dependency-boundary checks green. Fresh-process
public-method comparisons match pinned upstream `playwright-core@1.58.0`
13/13, including import-time environment and cwd snapshots. They are not an
official core449 pass because that unchanged test touches upstream private
state first.
Registry paths do not prove an installed browser, and the Node addon retains
its pre-existing host dependency tree. Full official conformance remains open;
see [the conformance ledger](../../docs/upstream-playwright-conformance.md).
The separate `pnpm run test:persistent-context-preflight` suite covers boxed
JavaScript string arguments, empty boxed certificate strings and Firefox
`-juggler*` ordering. With `PWRIGHT_PINNED_PLAYWRIGHT_CORE` pointing to the
pinned upstream package, the selected profile passes 23 focused checks with
one intentional default-only skip; the default profile passes 15 with nine
intentional opt-in skips. `pnpm run test:persistent-context-preflight-native`
checks its release binary. None of these checks proves valid persistent
launch or adds an official-test outcome.
Older checkpoint records below are historical.

The subsequent exact-source Core454 Firefox/WebKit invalid-numeric-proxy
browserless slice matches oracle 2/2. All four schema-2 sidecars verify, and
the 48,477-row strict ledgers now record candidate 1,685 expected / seven
skipped / ten unexpected / 46,775 not run and oracle 1,388 expected / six
skipped / nine unexpected / 47,074 not run. This is pre-spawn validation
parity, not positive proxy or browser support.

`Page.dispatchEvent(selector, type, eventInit?, { strict?, timeout? })` now
delegates to the Rust-owned Locator event action (first match by default), and
`ElementHandle.dispatchEvent(type, eventInit?)` dispatches on the retained
element owner. Plain event initializers use the atomic Locator evaluation
path; live JSHandle/DataTransfer initializers use a same-world owned-argument
path. Rust maps DeviceOrientationEvent and DeviceMotionEvent fields. The
distinct opt-in `official-conformance-page-dispatch-event` native artifact
SHA-256 is
`ff8e5db65ab83dca3e7f53fb22b493c81167ece9f7afcdb40e1424d2327ef34d`.
Node package tests pass 442/442, TypeScript and dependency-boundary checks
pass, and native ABI/profile/port smoke passes 1/1. A separate task-owned
Chrome smoke passed two live-DataTransfer dispatches; it is not an official
suite result, and lifecycle cleanup across scoped probes remains unresolved
after a later registered-selector probe retained native leases. The unchanged
Core v1.58.0 `page-dispatchevent.spec.ts` 17-case Chromium file is pinned by
`tests/official/playwright-core-page-dispatch-event-manifest.json` (SHA-256
`4bb9714f113992367330d0578f2862b0ced17d766c69310faaca655310b06137`).
The pinned Core oracle v1 and candidate v2 each classify the selected 17 as
14 expected / three unexpected, with identical per-spec outcomes; the shared
failures are sensor-event cases on an insecure HTTP fixture. Oracle
report/provenance SHA-256 is
`e5cc8d623a30203ebf336790c98fe3d19ca241de4d4196a6c21959c612fc627f` /
`c1b0486ccefc1a170d252678a1ee9306bb3495f015d4666807aede7253e7e95d`;
candidate v2 is
`c62ed0843bc3e0cbe225bd2277d550eac6bb7d706959605af783023d2414e1b9` /
`88ce1497a6ec6d6d53c743778b0d2baf0e02bafc805c188641ec3dff8afda478`.
After the HTTP v2 dispatch merge, strict 48,477-row ledger SHA-256 was candidate
`c75186cd78e28259a48247e553b566a514944f9470bc103e98d57b8de6411f4b`
(1,579 expected / two skipped / seven unexpected / 46,889 not run) and
oracle `65ec860c18b4b5e3e82e96f87167eb0fd36f170ca7f0c1e1e2f51b6886298f39`
(1,282 expected / one skipped / six unexpected / 47,188 not run).
After the JS request-cache lifecycle patch, candidate v3 again classified
14 expected / three unexpected, identical per spec to the pinned HTTP oracle.
Its report/provenance SHA-256 is
`693db5b080912f67831f382c472bb17e7387d340ab33489307c5e23273b69522` /
`aca608efc75caa1497e07b6635f7b95f6aec968a6f06f82d3bf942d071df3b03`;
strict candidate ledger SHA-256 is
`cc21a501b7a12e575c870ff1f74eb0a3e3db89b9b8b0a0aa438bf129bc2f4560`.
Core and Python fresh official runs now reject existing result/sidecar paths
before launch (apart from Python's explicit isolated-resume mode); the secure
Core validator requires the exact 17 unique spec IDs. Historical sidecars
do not hash runner/helper code, so this protection does not retroactively
prove a fresh earlier execution. A separate browserless `chromium-library`
BrowserType manifest SHA-256 is
`6845bcc03f0e3e6e7753b07edce877338c6c3f56c7de95dee3e5f400e1862594`.
Its `browserType.name` row is expected in both lanes, while invalid-channel
`launchServer` is expected only in the oracle and unexpected in the candidate.
After exactly those two rows, strict 48,477-row ledger SHA-256 is candidate
`556eafa00527dbf41cd7d494dcb2f2a9423272581af614ab6ea1a7f09f93faa7`
(1,580 expected / two skipped / eight unexpected / 46,887 not run) and oracle
`1b08b44e3351da8442652333694aef39a0ac7ab07ff866dea7d3e5b5f10fdb3c`
(1,284 expected / one skipped / six unexpected / 47,186 not run).
The Core HTTP fixture adapter sets `PREFIX` and `CROSS_PROCESS_PREFIX` to
the same MagicDNS origin, so its cross-origin-titled pass does not test
distinct origins. A revised task-owned trusted-Tailscale-TLS dual-port lane
has secure manifest/preload SHA-256
`efc2f9f131c245baf7ddca262cd08ca3b5cee3d9f8d3d7a4248cbe55a5a03de2` /
`f3243c9f73c33a07a8eb62428505bd612f6248384f02ded14ffe2d42d219b8c2`.
Browser-free checks pass, but **no secure official browser run** exists; a
preceding trailing-dot TLS attempt failed preflight before tests. Different
ports separate origins, not necessarily sites or processes. V3 provenance
verified at capture, but the runner changed afterward, so present-harness
re-verification is not claimed. Lifecycle cleanup remains unresolved and
full-corpus or complete semantic parity is not claimed.
The separate Python browserless `BrowserType.name` async/sync pair is pinned
by manifest SHA-256
`e34e466352e5bb94304ba5dfa21655255877f6330b5ba236313c99dc97e0dac8`;
installed ABI105-v3 candidate and independent oracle both pass 2/2 with
verified provenance. The source checker allows only this exact two-case
source/fixture-hash-pinned closure; other fixture-argument browserless cases
remain denied. Tailscale-bound HTTP/HTTPS fixture servers ran, no Chrome/CDP.
The two-key strict Python ledger merge reaches candidate 1,242 passed / eight
skipped / four failed / 5,272 not run (SHA-256
`ed31f1074ab675e75b8828e27a141a858144e8f85e408b9955d36fd1f2c6e4e8`)
and oracle 798 passed / seven skipped / four failed / 5,717 not run (SHA-256
`2c02dffe0d102fe0d2dd660b30318097e455018c074140f73db5707067511852`).

NODE-434 adds thin selector-based `page.check()`, `page.uncheck()` and
`page.setChecked()` delegates over the opt-in Locator/Rust checked-state
action. `ElementHandle.boundingBox()` reads the original retained Handle
through the existing Rust geometry program; it uses `getBoundingClientRect()`
and is not proven semantically identical to Chromium `DOM.getBoxModel` beyond
the selected official case. The complete unchanged 17-case Chromium Page
`page-check.spec.ts` file is pinned by
`tests/official/playwright-core-page-check-manifest.json` (SHA-256
`41787c922fd4690cc96cbf778e918132af57c778321afd7c470acec2ff8432b5`).
Baseline candidate v1 was 0/17 with 17 unexpected failures (16 missing Page methods,
one missing bounding-box method); the independent oracle passes 17/17.
The new `official-conformance-page-check` native artifact SHA-256 is
`a991b225d0a3dece24d0a457b05898e6aacb8f6217eb9d79f5d710c33ea30531`;
the original baseline artifact remains
`f229cbbc36691afaee8b8ba65d6dfffad2b3a8f2576647a1e186d5e558215384`.
Final candidate v2 and oracle each pass all 17 selected cases, with verified
candidate/oracle report SHA-256 values
`58a5de733c8d052f64919a5ad6a738672ca059f72e787ee0a54a14b321d19375`
and
`16b0b069164778d38e91b8979a0b7104aa1f8f2736f11abf32c1f69209689547`.
Exact 48,477-row ledger merges reach candidate 1,565 expected / two skipped /
four unexpected / 46,906 not run (SHA-256
`f3176185ef59fc45cc49228c3bec688dcea6c935ab2451f21e138962156cdad8`)
and oracle 1,268 expected / one skipped / three unexpected / 47,205 not run
(SHA-256 `d4a2878876e885d77297e47fe3f6bdaa91e72a4bc735448305c0cb205f8ea0a2`).
Both classification audits retain SHA-256
`32520af75e4ac14352a9060e6f7156900bb52c4a3d99fe927311ba7490328af0`.
Node package tests pass 424/424; TypeScript, dependency boundaries, selected
release host 617 passed / 14 ignored, and strict addon Clippy pass. Complete
Core and Python corpora remain open.

NODE-432 proves all nine unchanged Chromium Page
`page-wait-for-response.spec.ts` outcomes against Pwright and Playwright
1.58.0: both lanes have eight expected outcomes and the same one unexpected
cached-image result under Chrome 154. `waitForResponse()` shares the
synchronous Page event waiter, while Response publication, URL-glob matching,
terminal completion, body storage and copy limits remain in Rust.

The standalone `response-events` artifact exposes only bounded response
notices; its final binary is `8ac4603...49d1` and omits response completion,
body, routes and request history. The independent `response-finished` artifact
is `c744980...15a` and adds terminal ownership without selecting body reads.
The full official artifact remains `30bb029...853`. Node tests pass 407/407,
the exact official host union passes 850 tests with 29 explicit browser or
diagnostic ignores, and full-corpus conformance remains open.

NODE-431 proves the complete unchanged Chromium Page
`page-wait-for-request.spec.ts` file 8/8 against both Pwright and Playwright
1.58.0. `waitForRequest()` and `waitForEvent('request')` share one synchronous
language waiter with Playwright-shaped `TimeoutError`, quoted event messages,
captured public call stacks, Page-default timeout lookup and 50-code-point
matcher logs. Request observation, bounded lifecycle queues, URL-glob matching
and timeout state remain in selected Rust capabilities; no second observer or
native task is added. Full-corpus conformance remains open.

NODE-430 adds standalone `request-events` and `request-post-data-json` artifact
profiles. The first exposes `page.waitForRequest()` and Page request lifecycle
events over one bounded Rust notice stream plus the Rust URL-glob compiler.
The second adds only Rust JSON/form parsing to the same immutable Request
snapshot. JavaScript installs match listeners synchronously, applies public
RegExp/function predicates, and wraps snapshots through the existing Request
facade; it does not subscribe to CDP, parse post data or own request lifecycle
state. Both minimal artifacts are built and native-loaded independently, and
default artifacts still omit them.

The complete unchanged Chromium `network-post-data.spec.ts` file passes 6/6 in
candidate and Playwright 1.58.0 oracle lanes. The two annotated failures reach
matching real post-data assertions rather than a missing API. Exact six-row
ledger promotion reaches candidate 1,526 expected and oracle 1,229 expected;
full-corpus conformance remains open.

NODE-429 adds a standalone `response-request` artifact profile and composes it
explicitly into the official profile. Navigation `Response.request()` wraps the
actual immutable Request snapshot captured by Rust request history; it does not
reconstruct URL, method, headers or post data from JavaScript settings. The
feature requires frame navigation and bounded request history but does not
select Page routes or `extra-headers`. Default artifacts still omit it.

The complete unchanged Chromium Context/Page extra-header checkpoint records
the same outcomes in candidate and Playwright 1.58.0 oracle lanes: six expected
and one shared unexpected pass caused by Chrome 154 satisfying an assertion
still marked `test.fail`. This repairs the old masked
`response.request is not a function` result. The official artifact is
`4d06472...9f17`; all four provenance sidecars verify. Full-corpus conformance
remains open.

NODE-428 adds a standalone `touchscreen-input` artifact profile and composes
it explicitly into the official profile. Raw `page.touchscreen.tap(x, y)` and
selector-driven Page/Frame/Locator/ElementHandle tap are thin facades over
Rust-owned coordinate dispatch, actionability, hit-target, trial, modifier and
cancellation semantics. The complete unchanged Chromium Library `tap.spec.ts`
file passes 9/9 in both candidate and Playwright 1.58.0 oracle lanes with
verified provenance. Default artifacts still omit touch input.

NODE-427 adds an independent `device-emulation` native profile and composes it
explicitly into the official profile. `Browser.newContext({ hasTouch: true })`
now delegates one boolean to the existing bounded Rust Context device registry;
the Node facade performs only public validation and feature selection. The
Rust-owned Locator dispatch program also converts plain TouchEvent initializer
records into native browser `Touch` objects for `touches`, `targetTouches` and
`changedTouches`. The complete unchanged upstream Chromium touch-dispatch case
passes 1/1 against both candidate and Playwright 1.58.0 oracle. Default
artifacts still omit this capability.

NODE-416 makes Page/Frame `$` a single Rust-owned optional ElementHandle query.
The backend distinguishes exact null from an Element before publication,
reclaims absent-route admission, adopts utility-world Nodes and releases
non-Node results on failure. JavaScript neither counts nor derives `.first()`;
Rust owns the non-strict first-match rule while `Locator.elementHandle()` stays
strict. The remaining five official Chromium selector-registration rows pass
5/5 in both candidate and Playwright oracle lanes. Exactly five rows advance
the Core ledger to 1,379 expected / one skipped / two unexpected / 47,095 not
run, SHA-256
`8be091c0356f65de9a05b89b5a826d07ca6090342884fabb8296128a34ee8ea6`.
The official artifact SHA-256 is
`2018ea0912efd134c8e5b9984b168f722fcdd307900399e2b02a9250398d3d9e`;
331/331 package units plus types, native-load, boundary, release and Clippy
gates pass.

NODE-415 keeps registered-selector ElementHandle creation in Rust, including
main/utility-world selection, Node validation, utility-to-main adoption and
failure compensation. The Node addon also exposes Rust's atomic
`Browser.newPage()` implicit-Context owner; the current public slice accepts no
options, `baseURL`, or explicit `acceptDownloads: false`, and closing the Page
closes its hidden Context. `acceptDownloads: true` remains explicitly
unsupported until the independent download-events capability is selected. The
new official-profile artifact passes 329/329 package units, strict TypeScript,
native-load, boundary, Clippy and release ownership tests. Browserless official
selector-registration regressions pass 9/9 across library and BiDi projects
with verified provenance against that exact rebuilt artifact.
Page/Frame `$` selects `first()` in the Rust-owned Locator recipe before strict
Handle publication, so multiple matches return the first element. Atomic
count-plus-Handle publication during concurrent DOM mutation remains open.
Public invalid/predefined/duplicate registration and unknown-engine diagnostics
also retain their official wording.
The rebuilt official artifact SHA-256 is
`6196f520808b3ceb2059ff2a88cfa5fc85faa90ee34d728f25d4c9336d072dfe`.
The browser-backed selector row is still unverified because its safety preflight
found an unrelated nonblank Page in dedicated Chrome and refused to run; the
Core ledger therefore remains unchanged.

NODE-414 exposes Rust's non-strict first-element evaluation lane to the Node
facade, so Page/Frame `$eval` and `$$eval` execute registered selector engines
without handing selector parsing to JavaScript. The first official
browser-backed Chromium row passes 1/1 in both candidate and Playwright oracle
lanes, advancing the Core ledger to 1,374 expected / one skipped / two
unexpected / 47,100 not run. Package units pass 319/319. Positive custom
`page.$` Handle publication and the remaining selector rows are still open.

NODE-413 exports Playwright-shaped `selectors.register()` and
`selectors.setTestIdAttribute()` through an independent
`selector-registration` native profile. JavaScript only normalizes sources and
Node error casing; the shared bounded Rust environment owns registrations,
test-id settings and selector compilation. The initial browserless candidate
0/9 versus oracle 9/9 discrepancy is repaired to 9/9 in both lanes. Exactly
nine rows advance the Core ledger to 1,373 expected / one skipped / two
unexpected / 47,101 not run. The package passes 317/317 units and strict
TypeScript; browser-backed registered-engine rows remain open.

NODE-411 reuses the unchanged NODE-408 static `errors` and 143-entry `devices`
exports under the official BiDi configuration. Candidate and Playwright 1.58.0
oracle each pass all 12 browserless instances across six Chromium/Chrome and
Firefox channel projects with verified provenance. Exactly 12 rows advance
the Core ledger to 1,364 expected / one skipped / two unexpected / 47,110 not
run. No browser, native addon, package source or resource boundary changes.

NODE-408 exports Playwright's complete pinned 143-entry `devices` registry
through CJS, ESM and TypeScript. It is licensed static compatibility data with
an exact upstream SHA and provenance record; importing the package still does
not load a native artifact or start Chrome. The official browserless launcher
metadata slice moves from candidate 3/6 versus oracle 6/6 to 6/6 in both lanes.
Exactly six rows advance the Core ledger to 1,352 expected / one skipped / two
unexpected / 47,122 not run. Package tests pass 312/312 plus strict TypeScript.
No Rust ABI, native feature or resource boundary changes.

NODE-400 adds Playwright-compatible `to.have.value`, `to.have.class` and
`to.have.class.array` terms to the existing `_expect` adapter. Exact and regex
matchers, including mixed ordered class arrays, are typed at the JS boundary
and delegated to the Rust host; JavaScript owns no matching poll loop or DOM
program. The complete package suite passes 309/309, strict TypeScript passes,
and the native capability boundary verifies both standalone and combined
history profiles. The matching four upstream Python async/sync Chromium cases
pass 4/4 in both candidate and oracle lanes; this is incremental evidence, not
a full Core JS/TS conformance claim.

NODE-399 adds an independent `history-navigation` native profile and composes
the same leaf into `official-conformance-page-bindings`. `Page.goBack()` is a
thin public adapter over the Rust-owned browser-history command; Rust selects
the previous native history entry and owns same-document versus new-document
lifecycle waiting. Default artifacts do not contain the capability. The
standalone release feature compiles, the optional-dependency matrix passes, 40
native-profile contracts pass and strict TypeScript accepts the Playwright
option/result shape. The standalone and 25-feature official artifacts load the
native `pageGoBack` method and have SHA-256 values
`a3b6c3f189097cb4c1cb9da67f8cac8fc06c19fb0dd7005472ee2a81ea656774`
and `44a37f7126ed9ec81980eb815a752d64acfbc64c9c3bfe8a05b5f3f4773a8507`.
The differential history lane is defined but live browser evidence remains
pending.

Future official Core result sidecars now hash this package's `package.json`,
complete `src/` tree and the exact selected native artifact. The strict
provenance verifier rejects a wrong profile, source drift or artifact drift
before a report can be treated as ledger evidence. Historical reports are not
backfilled; the first runtime-produced verified sidecar remains pending a
memory-safe execution lane.

The complete unchanged Page init-script file is revalidated 9/9 against the
explicit current `official-conformance-page-bindings` artifact and now also
passes 9/9 independent oracle. These candidate rows were already expected;
exactly nine oracle rows advance, producing 1,051 expected / one unexpected /
47,425 not-run, SHA-256
`5aa674c8f42d538f54b58caf0b506662f6cb3ced6b736c4052b4eaa0d010b842`.
The Core runner now requires an explicit candidate native profile so a default
artifact cannot be misreported as a composed-profile compatibility failure.

The new independently buildable `bypass-csp` profile validates
`browser.newContext({ bypassCSP })` in this package and delegates Context
policy retention, session installation and teardown to the existing fixed Rust
owner. Proxy/header combinations remain explicit unsupported compositions.
The unchanged official CSP candidate progresses 0/4 to 4/4 across meta/header
CSP, cross-process navigation and iframes. Its final report SHA-256 is
`52762acf1fb0ba47924897e6827c3228df8fcb51700aba6de3d60d1a4ff89a2f`.
After the unrelated shared-Chrome page exited naturally, the guarded matching
oracle passed 4/4 with report SHA-256
`a60aba4f4226eb3d74eef621292a7dc059b81c8450e867c77057b2cb69dc2fbf`.

Default and standalone feature builds now compile independently after
FrameLocator and console callbacks were moved into their feature-gated N-API
impl blocks. The native class boundary covers 14 opaque classes. Package tests
pass 306/306 plus strict TypeScript, official/standalone native-load, boundary,
formatting and whitespace gates. The latest candidate ledger is 1,346 expected
/ one skipped / two unexpected / 47,128 not-run, SHA-256
`67dd35b67d392963e1848192c88ae1bbd5e811dee4856244022f924958eae508`.

The complete unchanged Context extra-header file separately passes 2/2 in
candidate and oracle. Candidate/oracle report hashes are
`c5694f1ef1c40cbb50b0b397bff35a45fdc70d678747cfd230de31f0149fb099`
and `a9cdcafcc44f8c8da97d7420f7f4e6a15a3cde000c8082707ccfc7757e31131f`.
The latest oracle ledger is 1,042 expected / one unexpected / 47,434 not-run,
SHA-256
`626e596ce9e0579fdcb8de5025060c51290cd6d78fb641286f64748d6d9b0a30`.
The separate 48,477-row applicability audit identifies all 417 expanded
invocations that use official private `__testHook...` options. Both current
candidate unexpected rows belong to that class but remain unexpected rather
than being rewritten as skips or passes.

The `official-conformance-page-bindings` profile now also selects the additive
`page-dialogs` and `page-routes` leaves. Dialog action, bounded route matching,
call publication and continue/fallback/abort settlement remain in Rust; this
package owns public object shape, event/callback identity and error
presentation. The backed Page route surface currently accepts string matchers
and exposes `continue`, `fallback` and `abort`; regex/predicate matchers,
override objects, `fulfill` and `fetch` remain explicit unsupported operations.

The complete unchanged official `frame-goto.spec.ts`,
`page-click-react.spec.ts` and `page-wait-for-load-state.spec.ts` selections
pass 4/4, 3/3 and 18/18 in both candidate and oracle lanes. This package passes
301/301 plus strict TypeScript and native-load smoke. The latest Core candidate
ledger is 1,340 expected / one skipped / two unexpected / 47,134 not-run,
SHA-256
`da4fe162d3dfeb7fa87017834c1d412fbbca6a84c026b5284d648156fdcca483`;
the oracle ledger is 1,036 expected / one unexpected / 47,440 not-run,
SHA-256
`cd0ef619ca9c96955e5e283a5089a785e6a19214d2c846b601d28d256f50b0e0`.
The remaining candidate failures use official implementation-private
`__testHook...` options and have not been hidden or treated as public passes.
This is measured NODE-366/367/368 progress, not full-corpus completion.

The two combined request profiles now compose client certificates with either
HTTP CONNECT or SOCKS5 without adding TLS dependencies to either proxy leaf.
Rustls upgrades the already-established tunnel and retains exact-origin SNI,
verification and client-identity policy. The unchanged official proxy+mTLS
rows pass 3/3 per proxy in Library and 6/6 per proxy in Bidi: 18/18 candidate
and 18/18 independent oracle overall, with no browser/CDP. The eleven request
crates pass 113/113 release tests and this package passes 282/282. The
candidate ledger is now 1,309 expected / one skipped / 31 unexpected / 47,136
not-run, SHA-256
`cc0a0c565820c7716a806d78bfea26b8d9c477b65f12d67bbd6d90ee14355399`;
the oracle ledger is 992 expected / 47,485 not-run, SHA-256
`a28e1e3e553e05292860d374ceb368336f7a33da4bbe0ff826e6aa7cb877c6a1`.
This is NODE-362 selected evidence, not a full-corpus completion claim.

The eight already-supported global-request client-certificate rows now pass
the complete six-project Bidi matrix: validation 6/6 plus seven direct-request
behaviors 42/42 in both candidate and independent oracle lanes. The selected
release artifact was rebuilt and load-verified before the candidate run. The
candidate ledger is now 1,291 expected / one skipped / 31 unexpected / 47,154
not-run, SHA-256
`45e50cac8c73c0fb08a84e85200a16599d08e8549e438f1e91ad1883fca454a6`;
the oracle ledger is 974 expected / 47,503 not-run, SHA-256
`cbc14a9f83502e1495fa752f5152f667d607263f11dd0615520051b4e870ac9b`.
NODE-361 changes no runtime, ABI or dependency edge.

The three unchanged global-request proxy rows now also pass the complete Bidi
project matrix: authenticated HTTP CONNECT 6/6, HTTPS proxy-server ALPN 6/6
and SOCKS5 6/6 in both candidate and independent oracle lanes. Together with
the Library matrix this is 27/27 per lane. The candidate ledger is now 1,243
expected / one skipped / 31 unexpected / 47,202 not-run; the oracle ledger is
926 expected / 47,551 not-run. This is the historical NODE-360 checkpoint;
current cumulative hashes are recorded above.

The optional `api-request-socks5-proxy` profile extends only `api-request` with
a bounded Rust SOCKS5 leaf. Node selects this profile independently from HTTP
CONNECT; Rust owns URL validation, the no-auth handshake, reply parsing and
remote hostname resolution. A real Tailscale-bound native smoke passes, and
the unchanged official SOCKS row passes 3/3 candidate and 3/3 oracle across
the Library project matrix. Together, the HTTP credential, HTTPS proxy-server
ALPN and SOCKS rows pass 9/9 in each lane. The eleven request crates pass 110
release tests and the Node package passes 279/279. HTTPS target tunneling,
authenticated SOCKS and bypass remain explicit later work. The current Core
ledgers are candidate 1,225 expected / one skipped / 31 unexpected / 47,220
not-run, and oracle 908 expected / 47,569 not-run. This is the historical
NODE-359 checkpoint; current cumulative hashes are recorded above.

The optional `api-request-http-proxy` profile extends only `api-request` with a
bounded Rust HTTP CONNECT leaf. Node validates the public proxy object and
passes server/credential values once; Rust owns proxy parsing, redacted Basic
authorization, tunnel establishment and prefetched bytes. Both unchanged
official global-request rows pass 6/6 candidate and 6/6 oracle across the
Library browser-name matrix: authenticated HTTP proxying and HTTP targets
through an HTTPS proxy with exact HTTP/1.1 ALPN. This remains the historical
NODE-357/358 checkpoint; current cumulative evidence is recorded above.

The optional `element-handle-dom` profile now also owns exact-target
`textContent`, ordered `$$` result publication and the temporary query-array
lifecycle for `$$eval`. Scoped XPath matches upstream's relative-slash and
Element-only behavior. Node only validates arguments and publishes wrappers;
Page/Frame `textContent` delegates to the existing Rust Locator. Package
273/273, strict TypeScript, native build/load, strict Clippy and boundary gates
pass. The unchanged official 17-row candidate/oracle rerun is still pending
because the shared dedicated Chrome has an unrelated page, so the ledger below
has not moved.

The optional `element-handle-dom` profile composes the retained Handle owner
with Rust-owned descendant selector compilation and `HTMLElement.innerText`.
`ElementHandle.innerText`, `$` and `$eval` first-match lookup are thin Node
facades; `Page.innerText` and `Frame.innerText` delegate to the existing Rust
Locator. The unchanged official focused selection passes 15/15 in candidate
and oracle. `ElementHandle.textContent`, `$$` and `$$eval` remain explicit
native-extraction work. Package 272/272, strict TypeScript, standalone/full
native-load, dependency/native-class/no-process proofs and selected-profile
strict Clippy pass. At the NODE-292 checkpoint the Core ledger was candidate
1,216 expected / one skipped / 31 unexpected / 47,229 not-run and oracle 899
expected / 47,578 not-run; the current cumulative values are recorded above.

The optional `api-request` native profile now exposes all seven global request
verbs through the Rust HTTP/TLS/redirect/retry/auth/cookie owner. Storage-state
cookies stay in Rust, origins are retained in the native context, and the
official resolver hook forwards bounded IP values without rewriting Host/SNI.
The complete unchanged official files pass candidate/oracle 198/198 + 60/60
in Library and 396/396 + 120/120 in Bidi. The optional
`api-request-client-certificates` profile adds lazy, exact-origin PEM/PFX mutual
TLS without loading material for an unmatched origin. The official malformed
input row passes 3/3 and the seven direct-request behaviors pass 21/21 in both
candidate and oracle lanes. A real Tailscale-bound PEM/PFX mutual-TLS smoke
also passes. At the NODE-223 checkpoint the Core candidate ledger was 1,214
expected / one skip
/ 31 unexpected / 47,231 not-run, SHA-256
`b9d7b48117f08def6811425eca735414ba803c506d6fa39cdb28ebf2c2e60c92`;
the oracle ledger is 884 expected / 47,593 not-run, SHA-256
`fcb2883c54d3b6b0df0f6344016947c3d8d4ae00b528e831d118e246b3aa4676`.
This is selected-file evidence, not a full Playwright suite claim. Proxy,
multiple identities for one origin and BrowserContext certificate wiring
and the remaining official invocations are open.

The current source adds Rust-owned official matcher diagnostics for the boolean
expectation slice: the last valid received state is retained, and the first
mismatch may collect one bounded element preview while the original deadline
still has time. Node formats the returned preview/missing fields but performs
no DOM or browser probe. Package 257/257, focused Rust expectation 12/12,
strict TypeScript, selected native-load and selected-profile Clippy pass. No
new official browser result is claimed because the empty-browser guard blocks
the run while the restored unrelated page remains open; the 62/24 result below
predates this change and still needs an isolated rerun.

The latest complete unchanged official boolean-expectation file records an
86/86 upstream oracle and a candidate progression from 37/49 to 62/24.
Checked, editable and enabled matching reuse typed Rust Locator reads;
indeterminate and focused matching use Rust-selected browser programs. Node
only validates public options and formats results. At that boolean checkpoint,
the Core candidate ledger was 416 expected / one skip / 31 unexpected /
48,029 not-run and the oracle ledger was 86 expected / 48,391 not-run. The candidate
run also exposed that fixture cleanup could retire an unrelated nonblank page
already present in dedicated Chrome. That exact page was restored, and the
official runner now refuses runtime execution until the CDP target list has no
nonblank pages. This is measured progress, not a full-suite pass claim.

The latest final-source actionability checkpoint passes the complete unchanged
official `page-click-timeout-2.spec.ts` file 2/2. Rust preserves one original
deadline, completes cancellation/cleanup first, then consumes one fixed
last-actionability enum for Playwright timeout wording. Node performs no DOM
probe, browser call or second timeout. The controlled 12-case Page batch is
9/3; the remaining failures are client-redirect/network-idle lifecycle, dialog
events and the implementation-private `__testHookBeforePointerAction`. The
cumulative Core ledger is 354 expected / one skip / seven unexpected / 48,115
not-run. This remains measured progress, not a full-suite pass claim.

The complete unchanged official Locator highlight file now passes 1/1.
Independently selectable Rust bounding-box and highlight leaves reuse the exact
Locator/Frame evaluation owner; highlight composes geometry, replaces one
bounded overlay/animation-frame state per selected utility world, and relies on
world teardown for navigation cleanup. Node supplies only the public Locator
description, result types and error presentation. The controlled 12-case Page
batch improves from 7/5 to 8/4. The cumulative Core ledger is 351 expected /
one skip / 10 unexpected / 48,115 not-run; this remains measured progress, not
a full-suite pass claim.

The latest unchanged-upstream matcher file now passes 2/2. Rust owns Locator
attached/detached/empty state, deep property and JavaScript-RegExp text
expectations, including navigation-context retries and correct missing-element
polarity under one bounded deadline. Node retains only the Locator description,
bounded expected-value conversion and Playwright error presentation. The same
controlled 12-case Page batch improves from 5/7 to 7/5. The cumulative Core
ledger is 350 expected / one skip / 11 unexpected / 48,115 not-run; this is
measured progress, not a full-suite pass claim.

The prior response-body Page batch records 5/12 expected outcomes. A
text selector matching inside a disabled control now times out without a click
because Rust reuses shared enabled-state retargeting; rich retry-log formatting
remains open. Rust preserves the specific detached-Frame navigation error and
the completed document's request identity; the independently selectable Rust
response-body leaf owns exact Page/Frame routing, `Network.getResponseBody`,
bounded base64 decoding and FFI lifetime. The thin Response facade caches the
copied Buffer for `body`, `text` and `json`. This raises the complete
`frame-goto.spec.ts` file from 2/4 to 3/4. The cumulative Core ledger is 348
expected / one skip / 13 unexpected / 48,115 not-run; this
is measured progress, not a full-suite pass claim.

Page and Frame selector actions now preserve their public `page.*`/`frame.*`
error identity while continuing to delegate to the same Rust Locator action;
Locator calls retain `locator.*`. Standard timeout punctuation also matches
Playwright. The official two-case Page click-timeout file still fails because
bounded Rust actionability progress diagnostics are not yet published, and the
private `__testHookAfterStable` upstream case remains explicitly unexpected.

Upstream conformance update: the unchanged Playwright v1.58.0
`page-navigation.spec.ts` file now passes 3/3. Quoted text shorthand and `>>`
chains remain typed Rust selector recipes, selector-runtime version 8 matches
submit/button input values, and paused popup startup is a fixed five-owner
Rust caller-polled batch. Node retains no popup scheduler, browser registry or
selector engine. This is one fully accounted official file, not a full-suite
conformance claim.

Upstream conformance checkpoint: the unchanged Playwright v1.58.0
`page-expose-function.spec.ts` file now records 24/24 candidate passes. Its
first complete run was 21/24 because `page.addInitScript`,
`page.waitForNavigation`, and `page.click` were missing. Those APIs are now present in the
source-built `official-conformance-page-bindings` profile: init scripts are an
independent Rust registration leaf, navigation waiting reuses the Rust Frame
lifecycle owner, and selector click delegates to the existing Locator action.
The profile also composes the existing optional Rust URL-wait leaf: Page and
Frame `waitForURL` keep lifecycle/timeout ownership in Rust while the thin Node
adapter preserves string-glob, JavaScript RegExp, and URL-predicate matching.
It also composes the independent zero-state Rust `request-gc` leaf:
`Page.requestGC()` resolves the exact Page's current Chromium session in Rust
and sends `HeapProfiler.collectGarbage`; Node does not select protocol commands
or retain GC state.
The navigation ticket is registered synchronously during native-call admission
before the returned Promise can yield.
The next run exposed missing `ElementHandle.innerText` after click dispatch;
that facade now reuses retained Rust Handle evaluation. Package tests are
219/219 and strict TypeScript/native-load/Clippy gates are green for the new
profile. A focused teardown probe still reports two retained native leases, so
the functional 24/24 result is not a cleanup-completion claim. The subsequent
unchanged Page init-script file now passes 9/9. The new Page/Frame script-tag
facade composes the existing Rust Frame evaluation and retained ElementHandle
owner; its complete official file progressed from 9/12 to 10/12 and now passes
12/12. `page.waitForFunction` is an
independent Rust feature with fixed eight-operation admission, native
deadline/cancellation and exact truthy Handle publication. Rust drives one-shot
Frame attempts and disposes falsy Handles, so no browser polling Promise remains
after timeout or cancellation.
The complete 26-case wait-for-function source also requires `page.$`,
`waitForTimeout`, and default timeout inheritance. `$` composes retained Rust
Handle evaluation; the timer is a separately selected fixed-operation Rust
leaf; Context/Page defaults stay in the existing fixed Rust timeout table and
an omitted wait timeout is resolved there. Their focused offline/release/native
gates pass. Complete official runs progressed 22/26, 24/26, 25/26 and 26/26.
The final fixes follow live Frame execution-context replacement across
navigation and map `ElementHandle.contentFrame` directly from the exact retained
element through the existing Rust topology owner, without a temporary Handle or
second store.

The next unchanged official files are also recorded, not sampled away:
Page console is 13/13, Page load progressed 0/2 → 1/2 → 2/2, locator-list
progressed 0/1 → 0/1 → 1/1, JSHandle evaluate is 2/2, asElement is 4/4 and
jsonValue is 3/3. Rust deduplicates Page lifecycle by bounded loader/phase;
FrameLocator lazily composes exact Handle-to-Frame resolution with a native
Locator; text expectations poll in Rust; `Locator.all` is capped at 1024 and
Playwright `>>` chains are Rust `Within` recipes. `jshandle-to-string` remains
0/6 because synchronous Handle previews and Playwright sparse-array console
formatting are not implemented yet.

NODE-004 migration has started below the package: Page/OOPIF session
initialization and synchronous Frame event/metadata projection are independent
exact five-package plugin contracts, while bounded Frame lifecycle/navigation-
wait observation is an exact ten-package plugin. Host adapters consume all
three directly. The aggregate keeps compatibility re-exports, so these
ownership slices reduce aggregate-coupled host files from 55 to 39 but
leave the default host/addon closures at 236/251 packages. The package remains aggregate-backed
and makes no feature-minimal or size-reduction claim yet.

Code-quality hardening (2026-09-21): the package loader now requires native ABI
version 1 and the `NativeEnvironment` export before it checks the exact feature
profile, so an empty, stale or wrong addon can no longer pass as the default
artifact. `pnpm test:native-load` loads the release addon in an isolated child
process and verifies that contract without leaving the addon runtime attached
to the test runner; a stalled macOS load can be killed without hanging CI.

Session-signal failures are now terminal, observed failures: the facade closes
the session, preserves a cleanup failure in an `AggregateError`, forces one
local disconnect if native terminal publication cannot complete, and never
leaves a rejected watch Promise unhandled. Page, Frame and JSHandle evaluation
also use the same runtime `String(pageFunction)` conversion already used by
Locator evaluation. Finally, `connectOverCDP({ timeout: 0 })` disables both the
overall attach deadline and ordinary CDP response timers; it no longer hides a
30-second command timeout.

CI now runs package unit and type contracts on Node 20, 22 and 24; Node 20 also
checks exact dependency boundaries, builds and loads the default native addon,
and exercises terminal retained-payload standalone/combined Rust profiles.
The package remains private and source-built: this checkpoint does not claim
prebuilt npm distribution support.

NODE-191 adds opt-in `retained-serialized-payload-mutation` and combined native
artifacts. The internal coordinator consumes the Rust-owned serialized graph
and retained prevalidated INPUT in one non-retargeting Rust/CDP operation,
performs the pinned `File`/`DataTransfer` plus `input`/`change` mutation, waits
for group cleanup, then terminally removes the same FFI ticket. It does not use
`DOM.setFileInputFiles`, add a generic evaluator or expose public
`ElementHandle.setInputFiles`.

Both rebuilt profiles pass 392 offline retained-input/oracle cases, 167 package
cases, all 42 ordinary real-Chrome raw binding cases and a separately enabled 50 MiB
minus one byte maximum-payload gate, plus focused Rust, Worker, type,
dependency, formatting and strict Clippy gates. The direct Playwright 1.58.2
matrix covers empty/multiple files, events, listener exceptions, late type
change, cancellation and lifecycle races. It records that Playwright resolves
post-instrumentation main-document navigation while Pwright rejects fail-closed;
the channel-stripped `lastModifiedMs` remains a second explicit discrepancy.
See the [canonical
NODE-191 contract](../../docs/knowledge/pwright-node-handles.md#node-191-terminal-rust-owned-serialized-payload-mutation).

NODE-190 adds opt-in `retained-serialized-payload-import` and combined native
artifacts. After NODE-189 performs JavaScript-compatible serialization, the
selected host reserves one bounded 96 MiB Rust owner before the addon copies the
graph. Success releases all eleven direct V8 roots and retains a move-once Rust
value on the same validated-target ticket. This remains internal transport
staging: it sends no CDP command, mutates no INPUT, emits no DOM event and does
not expose public `ElementHandle.setInputFiles`.

Both rebuilt profiles pass the 482-case extended offline matrix, 166 package
cases, 36 focused serializer/dispatch cases, 3 Rust owner cases, Worker, type,
exact boundary, formatting and strict Clippy gates, plus both complete 34/34
real-Chrome raw binding lanes. See the [canonical NODE-190 contract](../../docs/knowledge/pwright-node-handles.md#node-190-bounded-rust-ownership-for-the-serialized-payload-graph).

NODE-189 adds opt-in `retained-call-argument-serialization` and combined native
artifacts. The internal coordinator can now preserve one validated target from
instrumentation through Playwright-compatible value serialization while Rust
owns session/phase/root authority. It stops before CDP mutation, events,
terminal error transport and public `ElementHandle.setInputFiles`; normal users
should not treat this internal profile as upload API completion.

The selected checkpoint passes 398/398 offline file-input cases, 165/165
package cases, both 34/34 focused artifact lanes and both 32/32 real-Chrome
binding lanes, plus Worker, type, boundary, formatting and strict Clippy gates.

NODE-188: The `retained-input-instrumentation` and combined artifacts extend the
internal path through Playwright's awaited `onBeforeInputAction` position. Rust
gates `Prevalidated -> WaitingInputAction -> Instrumented`; Node awaits a frozen
internal port under the existing deadline with the original receiver and
retained `call@N`. The production port is an awaited no-op until optional
tracing/debugger/recorder/screencast capabilities are implemented.

This remains internal and adds no public listener API. Late utility-world
serialization, DOM mutation/events, terminal cleanup/error transport and public
`ElementHandle.setInputFiles` remain absent. Both rebuilt profiles pass Worker
identity checks and 30/30 real-Chrome raw binding cases; offline lifecycle,
package, boundary, type and strict Clippy gates are green. See the
[canonical NODE-188 contract](../../docs/knowledge/pwright-node-handles.md#node-188-awaited-input-action-instrumentation-boundary).

NODE-187: The new `retained-target-prevalidation` and combined artifacts add an
internal `preparePrevalidatedTarget` path. It consumes the exact Rust marked
owner after projection, runs read-only DOM retarget/policy validation, and parks
the validated owner on the same FFI ticket under the same Node deadline. The
ticket remains at ten roots and one 1 MiB permit; there is no release/re-adopt
gap, selector requery or payload-byte serialization.

This remains internal. Input instrumentation, late utility serialization,
mutation/terminal publication, Playwright error mapping and public
`ElementHandle.setInputFiles` are not implemented. Current gates pass 52
composition cases with one ignored, 666 host cases with 24 ignored, 68 addon
cases, 271 offline lifecycle cases, 163 package cases, both Workers/artifacts,
types/exact boundaries, strict standalone/combined Clippy and both 28/28
real-Chrome raw binding lanes. See the
[canonical NODE-187 contract](../../docs/knowledge/pwright-node-handles.md#node-187-same-owner-retained-dom-prevalidation).

NODE-186: Successful selected marking now retains the exact move-only native
target owner in the same parked FFI result. The internal
`prepareProjectedTarget` helper then runs the real carrier-projection kernel,
keeps four raw projected values as roots, stores native multiplicity/directory
flags and advances the same ticket from Marked to Projected. Selected roots grow
from six to ten; earlier profiles remain at six and no JS-byte bound is claimed.

This remains internal. Retained DOM prevalidation has not yet consumed the
owner, and input instrumentation, late serialization, mutation/cleanup, public
error mapping and `ElementHandle.setInputFiles` remain absent. Current gates
pass 68 addon tests, 666 host tests with 24 ignored, 270 offline lifecycle cases,
162 package cases, both Worker/artifact profiles, types/boundaries, strict
standalone/combined Clippy and both 27/27 real-Chrome raw binding lanes.
See the [canonical NODE-186 contract](../../docs/knowledge/pwright-node-handles.md#node-186-retained-marked-owner-and-same-ticket-carrier-projection).

NODE-185: The two `retained-target-marking` artifacts now allocate monotonic
`call@N` identities per native session after server validation, then start one
Node-event-loop deadline after the instrumentation checkpoint and before
materialization. That same progress covers materialization and native target
marking; the marker reads the retained Rust ID rather than accepting JS input.
The internal helper is `prepareMarkedTarget`; ordinary raw preparation can still
stop at Materialized.

This remains internal. Public `ElementHandle.setInputFiles`, carrier projection,
retained DOM prevalidation, input instrumentation, late utility serialization,
mutation and public error wrapping are not implemented. Current gates pass 68
selected addon tests, 269 offline lifecycle cases, 162 package cases, types,
boundaries and strict standalone/combined Clippy. The full internal real-Chrome
binding lane passes 26/26 under both rebuilt artifacts; this is not public API parity.
See the [canonical NODE-185 contract](../../docs/knowledge/pwright-node-handles.md#node-185-session-call-identity-and-one-node-deadline).

NODE-184: The package can build `retained-target-marking` and
`retained-target-marking-combined` artifacts. They expose an internal raw
Session method that takes an actual Materialized preparation through Rust
original-target marking and parks the same FFI ticket at Marked. The method is
feature-identity checked and absent from earlier profiles. It is not called by
the public file-input facade, so public `ElementHandle.setInputFiles` behavior
is unchanged and incomplete.

Both marker artifacts pass Worker loading and the real-Chrome raw binding lane
passes 24/24 under each, including same-permit accounting, synchronous abort,
late original disposal and foreign-session isolation. The retained lifecycle
contracts pass 267/267 under the marker profile. Public connection-wide call
IDs, the pre-materialization deadline, later native phases and full differential
parity remain gated. See the
[canonical NODE-184 contract](../../docs/knowledge/pwright-node-handles.md#node-184-same-admission-node-target-marking-handoff).

NODE-183: Rust host consumers can explicitly select the default-off
`retained-target-marking` feature for original-Handle marking with a bounded
persistent utility-world cache. Same-world calls reuse private state; native
retirement is independent of disposing source Handle wrappers. This is a Rust
host capability only: **no addon feature or Node artifact selection changes**,
and public `ElementHandle.setInputFiles` remains unimplemented. Node preparation
still stops at Materialized; typed handoff, original-target Weak upgrade, the
single operation deadline and later native phases remain integration work.
Selected Rust tests and strict Clippy pass. One assembled-host Chrome case
passes both final host selections with all resources retired; this does not
verify a Node artifact rebuild or public handoff. Default/older retained
regressions and dependency/boundary gates also pass. See the
[canonical NODE-183 contract and verification](../../docs/knowledge/pwright-node-handles.md#node-183-host-owned-utility-world-marker-cache).
Earlier checkpoints below retain their historical scope and execution status.

NODE-182: The default-off `retained-target-marking` backend/composition primitive
owns private persistent utility-world marker state separately from temporary
target groups. The caller still owns world-lifetime routing; this feature is
**not selected by host/addon profiles** and exposes no public retained upload.
Run `pnpm test:handle-target-marking-oracle`: **21 offline cases pass**, comparing
actual extracted Playwright 1.58.2 marking policy with the checked-in scripts
in plain-object VMs (19 paired cases plus two creation contracts), not real DOM.
Coverage includes same/different-call state, event order/flags, dynamic utility
methods and incoming Set-construction failure before old-state reset. Separate
Rust backend/composition suites pass 37/36 standalone and 55/51 with preflight
and large commands. One real-DOM native primitive case passes in both selections
with nine marking calls, eight events and zero remaining
owned cleanup resources. These are primitive-level results, not public Node
call-ID/deadline, weak-target promotion or complete setInputFiles parity.
[Canonical NODE-182 contract and evidence](../../docs/knowledge/pwright-node-handles.md#node-182-persistent-native-target-marking).
Earlier checkpoints below retain their historical scope and execution status.

NODE-181: Selected preparation now carries the original native wrapper's inert
weak identity through every language phase. Capture/clone does not lock,
upgrade, check disposal or pin native resources; copied metadata and an
overridden JavaScript receiver cannot replace this binding. The existing
32-entry capacity and independent call/synchronous-phase storage bounds remain
unchanged. Preparation still stops at Materialized: native Weak upgrade,
marking, typed native handoff, retained prevalidation and public upload are open.

Both rebuilt profiles pass **268 selected offline checks each**; package tests
pass **161**, including ten sampling/diagnostic tests, with types and the
35-case descriptor/language DOM oracle passing. Both selected addon suites pass
65 tests; the focused host phase-storage test, five strict Clippy selections,
boundary/dependency gates and formatting pass. After the final-retirement fix,
standalone and combined actual-ticket live binding each passed **20/20**. A
standalone repeat then passed 19 and failed one active-baseline comparison:
queued watch delivery became armed watch work with unchanged ownership totals.
The harness now also requires no queued FFI and exact running/pending watch
accounting for active baselines. Final frozen-source standalone, combined and
standalone-repeat runs each pass **20/20**, with zero failures/cancellations/skips
and fully retired owned cleanup: **20 distinct cases × three runs = 60 successful
executions**, not 60 distinct cases. Initial live evidence is retained: five
passes preceded a case-six cleanup failure and fourteen blocked cases; the
diagnostic rerun failed its first cleanup on `ffiPending = 1`, blocking nineteen.
The harness now waits for all final resources, including watches, to reach
zero instead of accepting command-idle samples; deadlines were not increased.
These cases do not prove later Weak upgrade, wrapper GC or upload success.
Exact evidence and final live results belong in the
[canonical NODE-181 checkpoint](../../docs/knowledge/pwright-node-handles.md#node-181-original-native-wrapper-binding).
Earlier checkpoint paragraphs below retain their historical execution status.

NODE-180: The selected host adds eight source-bound weak FFI revocation slots
with a separate 4 KiB administrative allowance. Registration belongs after
Materialized, immediately before the first native phase; it must not reduce
the 32 language-preparation slots or introduce early disposal/DOM checks.
Explicit close, peer termination and failure teardown revoke detached hooks
outside locks before native work drains. The shutdown admission barrier and
Owners close also keep cancellation/tracker wakeups outside their state locks.
Source review also moves retirement-failure transport disconnect outside the
session lock; this is not an assembled-session failure-path test.
This is host bookkeeping, not actual Node promotion: preparations still hold
a unit FFI result and stop at Materialized. Exact original-wrapper binding,
typed native progress, marking, retained prevalidation, late serialization and
public upload remain open. Rebuilt standalone and combined selected artifacts
each pass **268 cases across seven offline lanes**. Package contracts pass 151,
types pass and the descriptor/language DOM oracle passes 35; these are offline regressions, not
proof of actual Node ticket promotion or DOM/browser execution. The 17
actual-ticket live cases remain unexecuted; host/component results and remaining
gates
belong in the [canonical NODE-180 checkpoint](../../docs/knowledge/pwright-node-handles.md#node-180-session-bound-revocation-before-native-drains).

NODE-179: Rust now provides raw carrier projection plus same-admission FFI
park/resume prerequisites. Parking retains the original result/permit without
a running task token across foreign waits; neither prerequisite is wired into
the actual preparation, which still stops at Materialized. There is no marking
receipt or public upload API. Run
`PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:handle-file-input-projection`
and repeat with `retained-file-input-combined`. Both final artifacts, rebuilt
after the shutdown-lock/waker fix, pass **47 projection cases and 268 cases
across all seven selected offline lanes each**. Package contracts pass 151,
type checks pass, and the separate DOM oracle passes 35 descriptor/language
cases. These diagnostics do not prove actual ticket ownership, DOM/browser
behavior or upload success. The 17 actual-ticket cases remain unexecuted.
Final artifact and native evidence
belong in the [canonical NODE-179 checkpoint](../../docs/knowledge/pwright-node-handles.md#node-179-same-admission-ffi-handoff-and-raw-carrier-projection).

NODE-178: The default-off native `retained-preflight` leaf owns the retargeted
input after a payload-free DOM precheck; it is not selected by host/addon
profiles and does not expose public ElementHandle.setInputFiles. Run
`pnpm test:handle-file-input-dom-oracle` from this package: **35 offline cases
pass**, comprising 24 pinned upstream cases and 11 comparisons of the actual
candidate script against pinned callbacks on plain descriptors, not DOM/browser
execution. Current package contracts pass 151 cases and type checks pass. No
addon was rebuilt for these regressions; selected artifacts remain the NODE-177
builds. The 17-case actual-ticket lane remains unexecuted. Native test evidence
and remaining integration gates are maintained in the
[canonical NODE-178 checkpoint](../../docs/knowledge/pwright-node-handles.md#node-178-owned-native-file-input-prevalidation).

NODE-177: Selected retained preparation adds Rust registration checks before
server validation and after a thin empty-instrumentation await, with setImmediate
delivery from the client continuation. Checks use exact owner-ticket and creation
Page membership, not world/Handle/Frame runtime liveness. The internal driver
resolves undefined at Materialized; the caller retains its preparation. It adds
no roots, dependencies, classes, budgets or native pins and does not expose public
ElementHandle.setInputFiles. Twenty new offline dispatch cases pass per artifact;
canonical verification distinguishes those adapters from actual tickets/browser
work. Full instrumentation, error transport and correctly ordered late utility
serialization/native submission remain open. No fresh browser pass is claimed.
[Canonical contract and verification](../../docs/knowledge/pwright-node-handles.md#node-177-deferred-registration-gates-before-materialization).

After rebuilding, run `PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:handle-file-input-dispatch`
from this package and repeat with `retained-file-input-combined`. Both versions
pass 221 cases across dispatch, materialization, channel, arguments, preflight
and Worker lanes. The 17-case live binding lane remains unexecuted while shared
Chrome CDP is unavailable.

NODE-176: Selected retained-upload builds add Rust server-direction validation
and raw payload materialization, preserving custom serializers and outer async
assimilation. Public ElementHandle.setInputFiles and native submission remain
unimplemented. Run `PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:handle-file-input-materialization`
after building, then repeat for
`retained-file-input-combined`. This is offline kernel evidence only.
Both rebuilt profiles pass 76 new cases plus regressions; the 12-case live
ticket gate remains unexecuted while shared Chrome CDP is unresponsive.
[Canonical contract and current evidence](../../docs/knowledge/pwright-node-handles.md#node-176-server-direction-validation-and-raw-payload-materialization).

NODE-175 (2026-09-17): Selected retained-upload preparation adds Rust-owned
raw-buffer channel validation and an internal ChannelReady stop point. This
preserves observable map/valueOf/property assignment and Buffer identity without
copying bytes. It does not yet expose public ElementHandle.setInputFiles, valid
WritableStream authority, API error wrapping or native submission. No feature,
profile, dependency, class or preparation-root limit is added.
[Contract, verification and remaining gates](../../docs/knowledge/pwright-node-handles.md#node-175-rust-owned-raw-buffer-channel-validation).

After rebuilding the selected artifact, run
`PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:handle-file-input-channel`
from this package; repeat with `retained-file-input-combined`. This offline
diagnostic lane compares actual native schema behavior with pinned Playwright,
not actual preparation tickets or browser upload conformance.

The final standalone and combined artifacts each pass **61 channel cases**,
plus the existing argument/preflight/Worker checks. The live binding lane now
has eight authored cases, including ChannelReady and channel-failure cleanup;
these browser-dependent cases remain unexecuted.

NODE-174 (2026-09-17): Retained-upload profiles now bind Handle creation to a
Rust-owned canonical Page/Context timeout policy. JS forwards the creation Page
ID; Rust carries the immutable binding through descendant Handles and preparation,
without using current DOM ownerFrame or caller-selected late settings. Session
initialization consumes import-cached raw debug inputs; Rust chooses the policy.
Metadata admission now covers cells, repeated scoped wrappers and session indexes
under a 784 KiB administrative ceiling, not a process-RAM limit. `pnpm test`
includes offline facade forwarding/lifecycle contracts. Public default setters
and ElementHandle.setInputFiles remain unimplemented; actual native/browser
binding verification remains separate. [Contract, evidence and remaining work](../../docs/knowledge/pwright-node-handles.md#node-174-canonical-creation-page-timeout-binding).

The new live gate is **written and syntax-checked, not yet run**. Once the
dedicated Chrome CDP endpoint is recovered, run from this package:

```sh
PWRIGHT_NATIVE_PROFILE=retained-file-input PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:handle-file-input-binding
```

Repeat with `retained-file-input-combined`. The script sets `PWDEBUG=0` before
import. These six cases require actual session/Handle/preparation objects and
verify ArgumentsReady plus abort/reclamation; they do not prove public uploads.

NODE-173 (2026-09-17): Selected retained-upload staging now merges arguments
and resolves live numeric Context/Page timeout settings in Rust immediately
after conversion. The internal metadata factory adds a bounded 528 KiB allowance
and one opaque class, not browser or task ownership. Public creation-Frame
binding, default-timeout setters and `ElementHandle.setInputFiles` remain
unimplemented; no public timeout policy is replaced by this internal component.
After building, run `PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:handle-file-input-arguments`
from this package; repeat with `retained-file-input-combined`. This is offline
kernel/metadata verification, not actual-ticket or browser conformance.
[Exact contract, evidence and limits](../../docs/knowledge/pwright-node-handles.md#node-173-rust-owned-argument-assembly-and-live-timeout-settings).

NODE-172 (2026-09-17): The retained-file-input selections add a Rust preflight
kernel and reentrant phase guard. A narrow language driver preserves public
owner lookup, synchronous Context resolution and async conversion of the exact
ordinary payload carrier. Root counts now reflect 3/4/5/6 retained values by
stage. Initial construction and replacement separately reserve bounded
synchronous scratch (32 phases x 16 KiB), surviving callback revocation without
holding an FFI task across awaits. These are administrative allowances, not
arbitrary V8 graph byte bounds. This is still internal staging:
public `ElementHandle.setInputFiles`, options/channel validation, path work,
materialization and native submission are absent. Selected diagnostic exports
exercise the same kernel offline but create no real ticket/session/browser.
[Canonical contract, verification and limitations](../../docs/knowledge/pwright-node-handles.md#node-172-rust-owned-preflight-and-reentrant-language-phases)
distinguish kernel comparisons from public API compatibility.

After building either selected profile, run
`PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:handle-file-input-preflight`
from this package (substitute `retained-file-input-combined` for the combined
artifact). This runs the shared Rust kernel against pinned Playwright 1.58.2,
not the public upload method or actual rooted tickets.

NODE-171 (2026-09-17): Two new opt-in artifacts, `retained-file-input` and
`retained-file-input-combined`, expose an internal session-bound preparation
registry. Older profiles are unchanged. The new raw native ticket admits and
roots arguments, supports explicit abort, and is revoked during session close
and environment cleanup. Public `ElementHandle.setInputFiles` is still absent;
no owner lookup, conversion, serialized payload or native submission is wired
to this ticket yet. Its fixed entry/root limits do not bound arbitrary V8 input
graphs or user-retained closed-wrapper/tombstone allocations.

Build and run the offline selected-artifact check from this package:

```sh
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm build
PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:preparation-boundary
```

Use `retained-file-input-combined` for both commands to select the combined
profile. The check validates real class loading/type tags and environment
Worker exit, with zero actual tickets or browser connections. It does not
establish rooted-ticket cleanup or upload compatibility. See the
[canonical NODE-171 contract and evidence](../../docs/knowledge/pwright-node-handles.md#node-171-session-bound-node-preparation-registry).

NODE-170 (2026-09-17): The Rust host adds revocable `FfiPreparation<T>` under
its existing `retained-file-input` selection. Preparation shares the existing
FFI lane/byte budget without holding a running-task token until atomic start.
Exact-budget provenance binds the environment, not the session: the Node
registry, session revocation, type-tagged preparation ticket and full retained
upload ingress remain missing. This checkpoint adds no addon feature, public
Node API, artifact, dependency or feature.

`pnpm test:handle-file-input-oracle` characterizes upstream-only offline behavior.
The additions cover owner `page()`/`context()` evaluation, error precedence,
empty-input microtasks, actual client metadata disposal during getter reentry,
and custom Buffer serialization with per-item snapshot order.
They do not exercise native DOM disposal, a browser, or candidate upload parity.
[Canonical NODE-170 contract and current verification](../../docs/knowledge/pwright-node-handles.md#node-170-revocable-ffi-preparation-before-execution)
records Rust gates still in progress and remaining integration. Earlier entries
below retain their historical scope.

NODE-169 (2026-09-17): Run `pnpm test:handle-file-input-oracle` from this package to characterize Playwright 1.58.2 retained-upload ingress offline. It imports the pinned dependency from tests/node_modules and never connects to Chrome. The new retained-file-input selection is Rust-host-only; no Node artifact or public ElementHandle.setInputFiles support is added.
[Canonical contract and evidence](../../docs/knowledge/pwright-node-handles.md#node-169-safe-host-retained-upload-and-pinned-ingress-oracle).

NODE-167 implementation contract (2026-09-17): The explicit `handles` artifact
now exposes `ElementHandle.ownerFrame(): Promise<Frame | null>`. Rust queries the
retained node's current DOM owner, not its creation Frame, using a temporary
document-element Handle, direct raw `DOM.describeNode` and settled native
cleanup. After topology publication, JavaScript returns the canonical Frame
wrapper; `evaluateHandle`, `getProperty` and `getProperties` descendants preserve
the context-scoped resolver. Removed nodes may retain an owner; orphan documents
return null. Retained ElementHandle upload is still not implemented.

OwnerFrame acquisition validates/copies one protected receiver ID only after
evaluation admission. It rejects source-alias success and skips that borrowed
ID during malformed/exception compensation, still attempting every distinct
new owner. Other acquisition paths are unchanged; this is not generic
borrowed-ID protection or a release pin against concurrent source disposal.

To distinguish navigation from close, scalar and batch publication retain an
immutable Page/numeric-context/`Arc<str>` unique-context fingerprint inside the
existing 16 KiB descriptor allowance, with no client, receipt, permit or native
lease. Only an observed replacement main world on the same Ready Page permits
the canonical navigation error before lookup or refinement of ordinary
TargetClosed; missing worlds, crashes, closes and Cleanup are not reclassified.
The focused lane also covers full persistent capacity, same-document navigation
and navigation followed by close; canonical verification, not this case list,
records what actually passed on each artifact.

The host pre-owns the started-call continuation and shares eight call slots,
with two transient `RemoteHandle` units per query, sixteen aggregate at pending
scope `extension(154, 1)`. No slot from the 64 persistent-result owners is used.
Raw callers still owe source/future retention through cleanup settlement;
unaddressable raw IDs/pre-owned groups and conservative Cleanup-versus-detach
outcomes remain open. Run `pnpm test:handle-owner-frame` after dedicated
Chrome/Pwright preflight with `PWRIGHT_NATIVE_PROFILE=handles` and
`PWRIGHT_TEST_CDP=http://127.0.0.1:9222`; the optional OOPIF case also needs the
existing served `PWRIGHT_DRAG_FIXTURE`. [Exact verification and remaining gates](../../docs/knowledge/pwright-node-handles.md)
are canonical; this checkpoint makes no live-pass claim. NODE-166 and earlier
entries below are historical checkpoints, not current aggregate pass counts.

NODE-166 (2026-09-17): The handles artifact implements JSHandle.getProperties(),
returning an ordered Map with independently disposable JSHandle/ElementHandle
values. Its Rust core admits a whole batch before browser work and compensates
failed publication without waiting for GC. Selected session capacity remains
64 retained results total (including the source); concurrent calls share eight
slots. Run the focused comparison with `pnpm test:handle-properties` after the
required dedicated Chrome/Pwright preflight and with PWRIGHT_NATIVE_PROFILE=handles
and PWRIGHT_TEST_CDP=http://127.0.0.1:9222. [Exact contract and verification](../../docs/knowledge/pwright-node-handles.md)
remain distinct from the broader unfinished Playwright surface.

NODE-165 (2026-09-17): Rust Handle acquisition now preserves raw CDP result
and exception identities through settlement and compensates addressable IDs
before reporting failure. Existing admission stays held during that cleanup;
unaddressable IDs or refused releases remain visible Cleanup failures.
This adds no public Node API, package, feature or portable schema.
[The canonical checkpoint](../../docs/knowledge/pwright-node-handles.md)
records the selected artifact and exact verification; historical passes do
not verify a newly built artifact. Late property-read settlement, full property-
pipeline admission, pre-owned groups for unaddressable IDs, Node getProperties
batch ownership and broader compatibility remain unfinished.

NODE-164 (2026-09-17): All exported raw native classes now receive module-local TypeTag identities through the actual N-API conversion checks, addressing the earlier priority cross-build implementation gap. The napi_runtime dependency key still refers to the same napi package/version/features. Portable property extraction also preserves snapshot identity and pre-admitted cleanup ownership. [The canonical checkpoint](../../docs/knowledge/pwright-node-handles.md) records exact old-peer/same-build module isolation verification and remaining regression gates. Node getProperties and the broader compatibility surface remain unfinished.

NODE-163: The handles profile adds JSHandle.evaluateHandle (function/string and owned arguments) and getProperty. Browser policy and ownership remain Rust-side; failed native/JS publication compensates the owned result explicitly. getProperties remains unimplemented, and its five paired tests are retained as failures. Run pnpm test:handle-producers with the selected handles artifact; PWRIGHT_HANDLE_PHASE_TRACE=1 enables bounded oracle/candidate diagnostics. Exact evidence and property Promise-identity semantics are in ../../docs/knowledge/pwright-node-handles.md.

NODE-162: The handles profile now supports string-form JSHandle.evaluate and string Frame/Page evaluation with owned Handle arguments. Arguments are still converted and validated even though expressions ignore their values. Rebuild this profile for the updated native Handle call signature. Handle-targeted producers, previews, ownerFrame/adoption and retained ElementHandle actions/uploads remain unsupported.

NODE-161: The handles profile now accepts real Handle arguments in function-form Page/Frame.evaluate, Page/Frame.evaluateHandle and JSHandle.evaluate, including nested arrays/objects, aliases and cycles. TypeScript unwraps Handle argument types. String expressions with Handle arguments, cross-Frame adoption, additional Handle producers, toString and ElementHandle actions/uploads remain unsupported. Run PWRIGHT_NATIVE_PROFILE=handles pnpm build and the matching pnpm test:handles lane after dedicated Chrome preflight.

NODE-160: An independent handles native profile now supports Page/Frame.evaluateHandle, JSHandle.jsonValue, function-form JSHandle.evaluate, dispose and ElementHandle.asElement identity. Build with PWRIGHT_NATIVE_PROFILE=handles pnpm build; select that same environment value when running. Reference arguments, string Handle evaluation, toString, additional Handle producers and ElementHandle actions/uploads are not implemented. See ../../docs/knowledge/pwright-node-handles.md for exact verification and limitations.

NODE-159: The Rust safe host now integrates staged Frame Handle creation, driver publication and owned orphan-result cleanup. This Node package still does not select Handles or expose JSHandle/ElementHandle. No addon artifact was rebuilt or new public parity score established; see the canonical Node Handle integration record.

NODE-158: A separately opt-in Rust leaf now supports staged native Handle creation. This package still has no public JSHandle/ElementHandle API and does not select native-creation. No addon artifact was rebuilt; the last paired Node close evidence remains NODE-157.

NODE-157: Rust host Handle cleanup now uses bounded native-release receipts and
driver-owned local commits. Public JSHandle/ElementHandle APIs remain absent;
the addon still does not select handles. The existing combined profile
`file-input-concurrent-close-drag-frame-extra-headers` was rebuilt because
in-process concurrent close no longer allocates an unused control outbox.
Its SHA-256 is `93c0527d02bbcb66d8082e4ddda6f673b1b66528d784e82445e5ec122de2cd6a`.
Paired native-close-confirmation passes 6/6, paired lifecycle-events 7/7,
Node contracts 100/100 and strict TypeScript validation passes. Further validation and
remaining gaps are recorded in the [Node Handle checkpoint](../../docs/knowledge/pwright-node-handles.md#node-157-host-release-coordinator-and-parent-retirement-evidence);
this is not a rerun or improvement claim for the complete 57-case shared corpus.

NODE-154: The Rust host is adding owned pre-publication Handle results and session-driven cleanup. This Node package still has no Handle producer or selected handles artifact. Do not infer JSHandle/ElementHandle compatibility from the internal Rust owner tests.

NODE-152: The Rust host has a handles-only native installation prerequisite. This Node package still does not select it or expose JSHandle/ElementHandle. No native package artifact is rebuilt; use the Node Handle integration record for pending publication/cleanup work.

NODE-151: test:handle-file-input is a new explicit experimental conformance lane. PWRIGHT_HANDLE_PROVIDER=oracle characterizes 18 passing upstream cases; default paired mode fails on missing Node Handle APIs. This does not add ElementHandle support or change the prior shared score. Rust has a new borrowed-node payload primitive; no native package artifact was rebuilt.

NODE-150: Rebuilt file-input profiles support Page.setInputFiles and Frame.setInputFiles as well as Locator. Selector APIs choose the first match by default; strict: true opts into strictness. Upload verification passes 43/43 x3 standalone and 43/43 combined; full shared corpus remains 53/57 matching. ElementHandle and native directory uploads remain unsupported.
[Page/Frame upload contract and verification](../../docs/knowledge/pwright-file-input.md#node-150-page-and-frame-selector-taking-upload-apis).

NODE-149 verified checkpoint: upload suites pass 32/32 on standalone and combined profiles. The complete shared API corpus is 53/57 matching; overall Playwright compatibility is not complete. Default and previous native artifacts remain unchanged.

NODE-149: Locator.setInputFiles is now available only with PWRIGHT_NATIVE_PROFILE=file-input (build/select it explicitly). Supports memory payloads and regular paths; SharedArrayBuffer, native directories and Page/Frame/handle upload methods remain unsupported. Earlier NODE-148 and prior entries below are historical checkpoints.
[Selected Node upload boundary and verification](../../docs/knowledge/pwright-file-input.md#node-149-selected-node-buffer-ingress-and-public-locator-uploads).

NODE-148: Selected safe Rust host APIs now support buffered/path upload composition with existing Frame/Locator ownership. This JS package and all existing native artifacts remain unchanged: public setInputFiles is not yet implemented. Node ingress, selected addon build and differential verification are next.
[Selected safe-host upload contract](../../docs/knowledge/pwright-file-input.md#node-148-selected-safe-host-upload-integration).

NODE-147: The optional Rust transport can now admit large buffered uploads independently and fragment their writes. This Node package still does not expose setInputFiles or select the upload leaves; no native artifact was rebuilt. Public lifecycle integration and differential verification remain pending.
[Large outbound admission and remaining gates](../../docs/knowledge/pwright-file-input.md#node-147-independently-admitted-large-outbound-commands).

NODE-146: Rust has an optional owned payload-upload continuation with admitted cleanup and explicit protocol settlement. This package still does not expose setInputFiles or select the upload leaves. No native addon was rebuilt; full public cancellation, native path/directory behavior and differential verification remain pending.
[Upload continuation and remaining gates](../../docs/knowledge/pwright-file-input.md#node-146-single-use-upload-continuations).

NODE-145: Optional Rust buffered-file mutation now has actual Chrome fixture evidence, including an end-to-end Rust file-reading path. This Node package still does not select the backend or expose setInputFiles. Native path/directory completion, operation cancellation and public FFI integration remain pending; no addon was rebuilt.
[Low-level payload backend checkpoint](../../docs/knowledge/pwright-file-input.md#node-145-chromium-payload-mutation).

NODE-144: Optional Rust file-input execution now has bounded task admission and cancellation-safe filesystem workers. This Node package still does not select it: public setInputFiles remains unsupported until native action, lifecycle integration, message budgets and wrapper wiring are implemented. Native artifacts remain unchanged.
[Executor checkpoint](../../docs/knowledge/pwright-file-input.md#node-144-admitted-blocking-executor).

NODE-143: Optional Rust filesystem preparation and a pinned MIME provider now cover path sources, without selecting them in this Node package. setInputFiles is still unsupported; the executor, native backend and FFI wrapper remain to be implemented. Existing native artifacts remain unchanged.
[Filesystem/MIME checkpoint](../../docs/knowledge/pwright-file-input.md#node-143-filesystem-preparation-and-pinned-mime).

NODE-142: A separate Rust file-input content leaf now has ledger-admitted bounded copying/reading and ownership tests. It is not selected by this package; setInputFiles remains unsupported pending path preparation, native actions and Node integration. No native artifact was rebuilt.
[Content ownership checkpoint](../../docs/knowledge/pwright-file-input.md#node-142-ledger-admitted-content-ownership).

NODE-141: A separate Rust file-input preparation model and `pnpm run probe:file-input-oracle` now document pinned upload behavior. This does not add Locator.setInputFiles or an upload native artifact; the public API remains unsupported until resource-admitted IO, native actions and FFI wiring are complete.
[File-input policy, evidence and remaining work](../../docs/knowledge/pwright-file-input.md).

NODE-140: Broader verification is not green: Frame navigation 27/29 and shared API equality 51/57. The existing experimental artifact is unchanged. New bounded diagnostics are available as `pnpm run probe:page-close-body` and `pnpm run probe:shared-click`; their observed failures are retained, not counted as passing compatibility gates.
[Expanded verification and retained failures](../../docs/knowledge/testing-strategy.md#node-140-broadened-node-ffi-verification-remains-non-green).

NODE-139: The selected Frame navigation core retains Page lifetime through pending publication and final result conversion. Run the new paired committed-document close controls with `pnpm run test:frame-navigation-close`, selecting a frame-navigation native profile and setting PWRIGHT_TEST_CDP, PWRIGHT_TEST_BIND and the full PWRIGHT_TEST_HOST. This is a focused regression lane, not a claim of complete Playwright parity.
[Page lifetime publication contract and verification](../../docs/knowledge/pwright-frame-navigation.md#node-139-page-lifetime-through-navigation-publication).

NODE-138: Experimental `concurrent-close` and
`concurrent-close-drag-frame-extra-headers` profiles now wire bounded owned
Page/Context native close scheduling and local retirement in Rust. They install
into separate native directories and require a matching binary marker; the
default artifact is unchanged. Build explicitly with
`PWRIGHT_NATIVE_PROFILE=concurrent-close-drag-frame-extra-headers pnpm run build`,
then use the same environment selection when testing or loading the package.
The final combined artifact passes controlled/transparent close gates and 27
focused lifecycle regressions. This is not full compatibility: interrupted
Frame.goto error classification still differs, and default/external Context
paths retain the old scheduler.
[Integration scope, retained failure and browser evidence](../../docs/knowledge/pwright-close-cascade.md#node-138-opt-in-node-close-driver-integration).

NODE-133: Rust world receipts retain owning-Page identity and process real Page destruction even after local route retirement. Default and drag/frame/extra-headers addons were rebuilt and pass 28 focused real-Chrome regression cases; hashes and exact scope are recorded below. The serial Node close scheduler and controlled ancestor-dispatch gap remain unchanged. Earlier NODE-123 broad matrices cover older artifacts, not these builds.
[Native Page world-retirement contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-133-retained-worlds-follow-exact-native-page-destruction).

NODE-130: Rust adds an optional deletion-only local route commit for upcoming concurrent close integration. Node still uses its existing serial lifecycle path; no installed addon rebuild, public behavior change or new browser differential result is claimed.
[Deletion-only close route contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-129: Rust gains optional concurrent native close scheduling with bounded receipts and guarded route validation. The Node driver/addon still use serial close execution: installed artifacts and browser evidence remain NODE-123. Retirement/publication and host integration remain required.
[Bounded native scheduler contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-129-bounded-native-close-scheduler).

NODE-128: Rust gains optional exact native Context ancestor receipts and sealed Page bindings, verified at the composition layer. The Node driver/addon do not select native-close-authority yet; default/combined installed hashes and browser evidence are unchanged. Concurrent native coordination remains next.
[Native ancestor authority contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-128-exact-native-context-ancestor-authority).

NODE-127: The Rust leaf now supports separately owned native close acknowledgement and destruction waiting. This additive API is not selected by the package's serial close scheduler yet. Installed default/combined addon hashes and browser evidence are unchanged; exact ancestor receipts and coordinator integration remain next.
[Native close phase contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-127-native-command-and-destruction-wait-phases).

NODE-126: Rust close observers now support deferred publication admission and current-view preparation. This prerequisite is not wired into the Node close scheduler; installed native artifacts and browser compatibility evidence are unchanged.
[Deferred publication contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-126-deferred-close-publication-capacity).

NODE-125: Optional Rust runtime disposal/outbox reservations now complement topology reservations. Neither is selected by this package yet; Node binaries and browser evidence are unchanged. The next work is bounded native close scheduling, exact ancestor receipts, publication rebasing and complete peak ownership budgets. These storage primitives alone do not fix ancestor close dispatch.
[Runtime reservation contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-125-opt-in-runtime-disposal-and-outbox-reservations).

NODE-124: Rust now has an optional topology transition-reservation building block for concurrent close work. It is not selected by this Node package yet and does not fix ancestor close dispatch alone. Runtime disposal reservations, native scheduling and publication rebasing remain next; the NODE-123 addon artifacts and verified browser matrix are unchanged.
[Topology reservation contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-124-opt-in-topology-transition-reservations).

NODE-123: In-flight Context.close interrupted by Browser/peer termination reports TargetClosed after terminal staging; retired Contexts remain idempotent. Driver producer/queue loss now seals admission before close settlement, preserving original emitters. Both final profiles pass 50 close/event/connection cases plus two extra six-case confirmation repeats. `pnpm probe:close-cascade` records transparent or explicitly controlled diagnostics; the candidate ancestor-dispatch gate remains red, outside that matrix. All scheduling policy remains in Rust.
[Design, bounded probe configuration and evidence](../../docs/knowledge/pwright-close-cascade.md).

NODE-122: The Node Events composition selects Rust native destruction confirmation for Page.close, including ownership-safe interruption and terminal event settlement. Run `PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:native-close-confirmation` against the dedicated Chrome; the four cases include paired held-receipt behavior and Worker teardown. Both final artifacts pass 48 close/lifecycle/connection tests. Frame navigation remains 27/29 at the unresolved arrival-boundary close scenario; use the workspace `tests/fixtures` directory for that separate lane. No Node driver, browser launcher or full Playwright-parity claim is added.
[Contract and evidence](../../docs/knowledge/pwright-page-close.md#node-122-selected-native-destruction-confirmation).

NODE-121: Adds transparent original-listener close diagnostics and eleven ordinary unit tests. Both existing native profiles pass their 16 diagnostic samples and full 18-case listener corpora; natural failures remain unproven as fixed. Pinned source confirms the ordinary-close confirmation policy still needs a native composable implementation.
[Transparent trace and next confirmation contract](../../docs/knowledge/pwright-browser-close.md#node-121-transparent-original-scenario-tracing).

NODE-120: Native backend errors retained by Browser.close now include driver/retirement phase context. The new test:driver-close-attribution lane verifies an injected Page.enable response timeout on both rebuilt profiles. Full listener corpora pass 18/18, but naturally occurring driver timeouts and close-confirmation exhaustion remain unresolved; this is not a stability or complete compatibility claim.
[Close attribution contract and evidence](../../docs/knowledge/pwright-browser-close.md#node-120-attribute-retained-native-driver-failures).

NODE-119: Native Context.page and Page.popup notifications are connected to the Rust publication source and precommit wrapper converter, alongside explicit/external/terminal close events. Both final profiles pass full lifecycle and listener corpora 7/7 and 18/18. One combined listener repetition timed out during browser.close cleanup and remains unresolved. This is still an experimental API subset, not full Playwright compatibility or a stability guarantee.
[Native ready-source contract and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-119-native-ready-and-popup-source).

NODE-118: Internal ready/popup wrapper conversion is prepared, with metadata admission before native publication and wrapper cache installation afterwards. Native creation/linkage source wiring is still pending: newPage/popup event parity remains incomplete. Existing close-event support is unchanged.
[Creation-conversion contract and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-118-precommit-creation-wrapper-conversion).

NODE-117: External Page-close notifications now match Playwright in the Worker-confirmed differential case, alongside explicit and terminal close sources. Both profiles pass 5/7 lifecycle-event cases and 14/18 listener cases. newPage/popup notifications remain incomplete; this package is not yet fully Playwright-compatible.
[External-close source contract and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-117-pre-admitted-external-close-source).

NODE-116: Explicit owned Page/Context close events now reach Node using Rust native transaction receipts, including synchronous listener failure and close reentry. Terminal events remain supported. Page creation, popup and externally initiated Page-close event parity are still incomplete.
[Explicit-close source contract and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-116-explicit-close-source-integration).

NODE-115: The package now selects Rust terminal lifecycle delivery: Session termination dispatches close on existing Page/Context wrappers before Browser disconnected. Ordinary Page/Context explicit-close events and creation/popup notifications remain incomplete; do not treat the package as fully Playwright-compatible.
[Terminal dispatcher contract and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-115-environment-thread-terminal-dispatch).

# Pwright Node FFI (experimental subset)

NODE-114: The Rust host now offers an experimental terminal-event connection path, verified through actual native shutdown. This package does not select it yet; Page/Context event coverage still awaits native ordinary-source joining and FFI delivery.
[Terminal source contract and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-114-pre-admitted-native-terminal-source).

NODE-113: Rust now provides a bounded lifecycle publication group/cursor primitive; it is not yet wired into this package's Page/Context events. The listener gate adds Context/Browser close exception-isolation cases. Browser disconnected support is unchanged.
[Group contract and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-113-bounded-dependent-publication-groups).

NODE-112: The Rust composition offers opt-in explicit close receipts. The Node host still
uses its existing services without that observer; this does not add public
Page/Context events. Native selected-path and fresh addon regression evidence
are tracked separately.
[NODE-112 close receipt contract and verification](../../docs/knowledge/pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: The native composition gains a borrowed driver-effect input. No Node API or
new lifecycle callback support is exposed by this checkpoint; the existing
metadata fallback remains in use. Rebuilt artifact regression evidence is
recorded with the source coverage limits.
[NODE-111 source contract and verification](../../docs/knowledge/pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-110 fixes the synchronous Browser disconnected listener error channel in
rebuilt addons. test:lifecycle-listeners adds eleven isolated reference/paired
cases; Page/Context event cases remain incomplete. Older addons keep the old
exception behavior until rebuilt. No runtime queue or browser logic is added to
JS by this scoped exception-forwarding boundary.
[NODE-110 contracts, fix and verification](../../docs/knowledge/pwright-node-lifecycle-events.md#node-110-listener-failure-boundaries-and-scoped-exception-reporting).

NODE-109 adds an exclusive Rust metadata drain foundation, not a new Node API or
lifecycle callback implementation. The package still publishes whole fences.
Page/Context/popup lifecycle notifications remain incomplete; internal queue
steps must not be treated as supported Playwright events.
[NODE-109 contract and verification](../../docs/knowledge/pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-108's attachment-command diagnostic requires a freshly rebuilt Rust probe
with bounded native lifecycle identity receipts. Old probe binaries lack this
field and fail evidence validation. This TRACE data does not implement public
Page/Context lifecycle callbacks. Controlled retirement comparisons pass on the
unchanged NODE-107 Node addons; earlier intermittent initialization failures are
not claimed fixed.
[NODE-108 receipts and verification](../../docs/knowledge/pwright-chromium-topology-events.md#node-108-bounded-native-lifecycle-identity-receipts).

NODE-107's rebuilt combined-profile live gate is not fully verified: attachment
timeouts interrupt Page enumeration/baseURL and some other cases. The Rust-only
combined probe reproduces initialization stalls outside the Node bridge; no fix
or deadline change is included. Default regressions do not prove combined stability.
[Known failures and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#combined-profile-reproduction-and-incomplete-live-gate).

NODE-107 adds a Rust-only primitive for rebasing visible collection membership
across admitted revisions. Existing Node collection getters and full-fence
publication remain unchanged. This does not implement lifecycle callbacks.
[NODE-107 contracts and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-107-validated-cross-revision-visibility-rebasing).

NODE-106 rebuilds both addon profiles with Rust immutable collection visibility
views. Existing getters still expose full published membership; the new view
foundation does not add Context page, Page/Context close or popup event support.
No JavaScript API, dependency or feature-selection change is introduced.
[Implementation and measured evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-106-immutable-single-revision-visibility-foundation).

NODE-105 adds the intentionally RED test:lifecycle-events lane. Dedicated-Chrome
preflight plus PWRIGHT_TEST_CDP is required; no HTTP fixture is used.
PWRIGHT_LIFECYCLE_REFERENCE=1 runs the pinned reference only (7/7). Default and
drag-frame-extra-headers each fail all seven cases: Context page, Page/Context
close and popup callbacks are missing, although final collections are correct.
Do not infer these events are supported merely because on() accepts a listener.
[Evidence and Rust implementation design](../../docs/knowledge/pwright-node-lifecycle-events.md).

NODE-104 adds test:collection-creation (7/7 twice on both NODE-103 artifacts).
It covers blocked-loop single/concurrent creation, retirement/replacement and
independently confirmed borrowed Page discovery; no HTTP fixture is needed.
Use the dedicated Chrome/Pwright preflight and PWRIGHT_TEST_CDP explicitly.
PWRIGHT_COLLECTION_REFERENCE=1 runs only the pinned reference, not candidate parity.
Context page, Page/Context close and popup events remain unimplemented.
[Exact scope and evidence](../../docs/knowledge/pwright-frame-publication.md#node-104-creation-and-discovery-publication-characterization).

NODE-103 connects collection lifecycle publication and reads in Rust. Rebuilt
default and drag-frame-extra-headers pass test:collection-publication (7/7),
test:collection-lifecycle (1/1), Browser/Page scalar and enumeration regressions.
test:context-input-boundary checks Pwright's own bounded-input TypeError contract
and selected-header getter identity; test:context-base compares 16 upstream cases.
No browser is downloaded or launched by the package; use the dedicated Chrome
Pwright preflight and explicitly select/rebuild the desired native profile.
[Implementation, artifact hashes and remaining limits](../../docs/knowledge/pwright-frame-publication.md#node-103-rust-collection-producers-and-published-node-reads).
The earlier red/unwired checkpoints below are historical, not current test status.

NODE-102 implements the Rust collection storage/fence layer, but does not yet
wire native lifecycle producers or replace Node collection getters. Installed
artifacts are unchanged and test:collection-publication remains RED as recorded
in NODE-101. Do not treat the new Rust unit results as a public compatibility fix.
[Storage implementation and pending cutover](../../docs/knowledge/pwright-frame-publication.md#node-102-bounded-rust-collection-revision-storage).

NODE-101 adds the intentionally RED collection publication gate:
```sh
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:collection-publication
```
Use the dedicated-Chrome/Pwright preflight; no HTTP fixture is required.
PWRIGHT_COLLECTION_REFERENCE=1 explicitly runs reference-only characterization
(7/7 twice), not candidate conformance. Default and drag-frame-extra-headers
currently pass only 1/7: collections remove members too early while Node is
blocked. No native fix/rebuild is included in this checkpoint; NODE-100's
scalar getter fix remains valid within its tested scope.
[Evidence and Rust implementation gate](../../docs/knowledge/pwright-frame-publication.md#node-101-context-and-page-collection-publication-discrepancy).

NODE-100 fixes Browser.isConnected publication timing in Rust and removes the
JS disconnect flag from rebuilt artifacts. After dedicated-Chrome/Pwright
preflight, run:
```sh
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:browser-connection
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 PWRIGHT_NATIVE_PROFILE=drag-frame-extra-headers pnpm test:browser-connection
```
No HTTP fixture is needed. Both rebuilt profiles pass 5/5 and Page.isClosed
8/8; the combined Page enumeration lane passes 7/7. Rebuild each desired native
profile: older binaries keep the premature getter behavior through a legacy
fallback. Broad lifecycle/event/API compatibility is not yet complete.
[Contract and current artifact hashes](../../docs/knowledge/pwright-browser-close.md#node-100-rust-owned-browser-disconnect-publication).

NODE-099 adds Rust-backed `page.isClosed(): boolean`. Rebuild the desired native
profile; old artifacts fail this method explicitly. The existing
`PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:page-is-closed` lane now passes
8/8 on final rebuilt default/combined profiles, with Page enumeration 7/7 and
Worker teardown 3/3 each. Final artifact hashes and resource checks are in the
checkpoint below; other profiles have not been rebuilt in this turn.
No browser launch, JS lifecycle copy or new dependency is introduced. Page
close/crash events, before-unload true and close reasons remain unsupported.
[Contract and verification](../../docs/knowledge/pwright-page-close.md#node-099-rust-owned-page-close-publication).

The NODE-098 red results below are the preserved pre-implementation baseline.

NODE-098 adds `PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:page-is-closed`
(after the dedicated-Chrome/Pwright preflight; no HTTP fixture). This lane is
currently RED: Page.isClosed is not implemented. Eight upstream cases pass;
default and combined candidate runs fail 0/8 at the missing method. Explicit
`PWRIGHT_IS_CLOSED_REFERENCE=1` runs labelled reference-only characterization,
not candidate conformance. No native artifact or supported API changed.
[Contract and next implementation gate](../../docs/knowledge/pwright-page-close.md#node-098-pageisclosed-reference-contract-and-failing-candidate-baseline).

NODE-097 expands `pnpm test:navigation-peer-disconnect` to 11 cases: four existing
shutdown controls plus seven paired mid-navigation peer/recovery observations.
Combined and default pass 11/11. Three modes deliberately reproduce native
navigation failure in BOTH implementations; passing means matching the original
outcome and healthy explicit recovery, not pretending every navigation succeeds.
The same dedicated-CDP and Tailscale fixture variables below apply. No production
API or native binary changed. [Evidence](../../docs/knowledge/pwright-chromium-topology-events.md#node-097-page-auto-attachment-from-a-second-client).

NODE-096 adds `pnpm test:navigation-peer-disconnect`: four owner/peer combinations
hold the borrowing client's native child resume command, disconnect that client,
and verify the owner Page/child survive. Use the selected native profile plus
PWRIGHT_TEST_CDP=http://127.0.0.1:9222, explicit PWRIGHT_TEST_BIND (Tailscale IP)
and PWRIGHT_TEST_BASE (full MagicDNS HTTP origin), as with the header fixture.
The combined control passes 4/4; continuous connection churn remains failing.
This test-only checkpoint does not change production Rust or increase deadlines.
[Evidence and limits](../../docs/knowledge/pwright-chromium-topology-events.md#node-096-navigation-timing-and-borrowing-peer-control).

NODE-095 extends test:attachment-retirement to require connection settlement
from native detach BEFORE a held initialization response. The Node host selects
Rust pending-session retirement with a 256-byte native ID bound; no JS driver is
added. Current combined/default attachment cases pass 10/10 and 6/6;
test:detach-retirement now tests three native receipt/event/live-Worker orderings
and passes 3/3 on both artifacts. Child-header overlap remains a failed gate.
[Contract and evidence](../../docs/knowledge/pwright-chromium-topology-events.md#node-095-native-detach-before-an-initialization-response).

NODE-094 adds `pnpm test:queued-context-retirement`, using the dedicated Chrome
CDP endpoint and no HTTP fixture. It tests late disposed/live Context attachment
while a known Page's native initialization is held. Build the selected native
profile before running; this lane is not complete Playwright conformance.
[Checkpoint](../../docs/knowledge/pwright-chromium-topology-events.md#node-094-queued-attachments-from-disposed-contexts).

NODE-093 expands test:attachment-retirement to cover header initialization:
8 combined cases and 6 headers-only cases pass on rebuilt artifacts. The gate
selects the header-specific Network.enable occurrence, while the pinned oracle
uses its shared network manager. Broader concurrency/disconnect parity is still
unfinished. [Checkpoint](../../docs/knowledge/pwright-extra-http-headers.md#node-093-typed-header-initialization-retirement).

NODE-092 adds test:detach-retirement: a real native Worker-close race between
resume and detach, plus a live-worker preservation control. The fix is entirely
Rust; no extra feature is required. Full Worker/Node compatibility remains open.
[Build, command and evidence](../../docs/knowledge/pwright-chromium-topology-events.md#node-092-child-disappears-between-resume-and-detach).

NODE-092 targeted default/combined comparisons pass 2/2 each (combined repeated).
Serial child-header/attachment regressions pass 8/8 and 7/7. The overlapping gate
still fails at 4/8 child-header cases alongside 35/35 headers. Native tests pass
79 CDP, 373 composition and 274 host; this is not complete compatibility.

NODE-091 adds test:header-child-sessions, comparing native OOPIF and main-session
closure during public header setters with Playwright 1.58.2. Session policy and
bounded watches live in Rust; the relay is test-only. This remains an experimental
subset, and Page.isClosed is not implemented by this checkpoint.
[Build, command, evidence and exclusions](../../docs/knowledge/pwright-extra-http-headers.md#node-091-native-session-closure-during-header-fanout).

NODE-091 final gate: 8/8 in both `drag-frame-extra-headers` and `extra-headers`,
plus one combined repeat. Existing headers pass 35/35 and attachment 7/7; Rust
host tests pass 274. Concurrent attachment/cleanup failures remain open, so
these artifact-specific passes do not establish full compatibility.

NODE-090 adds `test:attachment-retirement`, a real-Chrome public differential lane
for a Page disappearing during native connection initialization. Select and build
the desired native artifact first; the combined profile also covers lifecycle
Network/Page enabling. The rebuilt combined artifact passes 7/7 plus three 7/7
repeats, and headers pass 35/35. Rust CDP/composition/host tests pass 78/373/269
respectively. This remains an experimental subset, not complete compatibility.
[Command and evidence](../../docs/knowledge/pwright-chromium-topology-events.md#node-090-missing-session-retirement-before-page-publication).

NODE-089 final gate passes 35/35, with three further 4/4 receipt-gate repetitions.
An earlier attachment failure remains unresolved under target churn. Host units
pass 269; isolated native close reports zero owners. No test runtime/relay is
left running, and the existing foundation commit remains local and unchanged.

NODE-089 corrects Context Page-by-Page header dispatch and native protocol-error
messages in Rust. Four public success/failure comparisons hold real Chrome ACKs
while a newer setter proceeds, including deferred later-Page values. These run
inside the existing `test:extra-headers` lane; its diagnostic relay is test-only
and loopback-only. Full Node/cross-browser compatibility remains unfinished.
[Current evidence](../../docs/knowledge/pwright-extra-http-headers.md#node-089-native-receipt-gates-and-context-page-sequencing).

NODE-088 selected header builds use bounded Rust native fanout/jobs. A setter's
first failure settles before its retained compensation drains; the existing driver
continues polling jobs and lifecycle work. Combined public tests pass 31/31.
Controlled real-CDP timing, Worker/OOPIF handoff and shutdown retirement remain
unfinished; this is still an experimental subset.
[Current evidence](../../docs/knowledge/pwright-extra-http-headers.md#node-088-caller-polled-native-jobs-and-early-primary-rejection).

NODE-087 selected header profiles now prepare setters and Page/Frame navigation
synchronously in Rust before scheduling native completion. Frozen native plans
preserve each invocation's values; combined release tests pass 30/30 and five
focused repetitions pass 6/6 each. Unpolled Rust future ordering and reclamation
also pass live. This fixes
the demonstrated logical ordering path, not native concurrent fanout or early
failure settlement. The package remains an experimental subset.
[Current behavior and evidence](../../docs/knowledge/pwright-extra-http-headers.md#node-087-synchronous-rust-preparation-and-frozen-dispatch).

NODE-086 identifies an unresolved ordering bug: navigation immediately following
an unawaited header setter can see stale Page headers. The expanded combined lane
has 26 tests and retains this regression. Rust's optional overlapping model matches
128 internal upstream traces, but is not yet integrated into the host scheduler.
The rebuilt full lane passed 26/26 once; three focused repeats were pass/fail/pass.
The Frame ordering discrepancy remains unresolved in the current artifact.
[Current discrepancy and next work](../../docs/knowledge/pwright-extra-http-headers.md#node-086-pending-setter-race-and-bounded-overlapping-model).

NODE-085 selected header builds reconcile `goto({ referer })` against the first
original Page Referer in Rust; Context headers do not cause option conflicts.
Combined header/drag/frame tests pass 24/24 and header-only passes 23/23.
The implicit referrer copy has a
16-KiB selected bound and eight admitted concurrent navigation owners.
[Current behavior and limitations](../../docs/knowledge/pwright-extra-http-headers.md#node-085-navigation-referrer-reconciliation).

NODE-084 adds `browser.newContext({ extraHTTPHeaders: { 'X-Example': 'value' } })`
to selected header profiles, composable with `baseURL`. Rust owns the complete
creation/settings transaction. Header-only and combined drag/frame/header releases
both pass 21/21 live tests, now
including creation language semantics, deferred syntax, recovery and admission.
Earlier notes below retain their historical scope; full compatibility is not claimed.
[Current evidence and remaining work](../../docs/knowledge/pwright-extra-http-headers.md#node-084-context-creation-headers).

NODE-083 corrects deferred HTTP-syntax validation and Page-initialization failure
isolation in selected Rust header builds. `pnpm test:extra-headers` now has 15
tests, including Context masking/clearing, recoverable failed Page creation and
18 failed-Page retirements. Fresh header-only and drag/frame/header artifacts
both pass 15/15; the latter checks Frame.goto after failure recovery. Creation's
`extraHTTPHeaders` option remains unsupported; this fixes prerequisites rather
than claiming that option or full failure/cancellation compatibility.
[Current evidence](../../docs/knowledge/pwright-extra-http-headers.md#node-083-deferred-syntax-and-page-local-initialization-failure).

NODE-082 adds `Page.setExtraHTTPHeaders` and `BrowserContext.setExtraHTTPHeaders`
in independently selected native header profiles. Build and run with
`PWRIGHT_NATIVE_PROFILE=extra-headers`; `pnpm test:extra-headers` runs nine paired
public groups and one input-budget test (10/10 passing). The lane requires the
dedicated Chrome preflight and explicit CDP/Tailscale bind/base variables; it
owns a dynamic fixture. Rust handles merge, ownership, routing and native work.
Context creation's header option, navigation referer policy, native failure
fanout/timing, OOPIF/workers and full compatibility remain unfinished.
[Scope, budgets and commands](../../docs/knowledge/pwright-extra-http-headers.md#node-082-public-selected-setters-and-language-phase-differential).

Header combinations are `extra-headers`, `drag-extra-headers`,
`frame-extra-headers`, and `drag-frame-extra-headers`, in addition to the four
existing profiles. Each has a separate native path and all three compiled flags
must match. Only header-only was rebuilt/live-tested in this checkpoint; combined
Clippy and dependency gates do not establish combined live parity. Earlier NODE
entries below are historical checkpoints, not current profile coverage.

NODE-081 adds opt-in native Rust host header application, not public Node
`extraHTTPHeaders` setters or a packaged addon feature. Existing addon binaries
are unchanged. The dependency check now also verifies header-only/combined Rust
host selections and keeps headers absent from existing addon selections.
[Native tests and remaining public work](../../docs/knowledge/pwright-extra-http-headers.md#node-081-selected-rust-host-and-native-session-reconciliation).

NODE-080 adds `scripts/check-header-settings-oracle.mjs`, a development-only
54-case comparison of Rust metadata transactions with pinned upstream setters.
It is not a public Node extraHTTPHeaders implementation; package API and retained
addon binaries are unchanged.
[Build/run instructions](../../docs/knowledge/pwright-extra-http-headers.md#node-080-bounded-desired-settings-transactions).

NODE-078 adds development-only header characterization and Rust policy
comparison scripts. Public `setExtraHTTPHeaders` and Context creation's
`extraHTTPHeaders` option are still unsupported; no addon was rebuilt.
[Commands, evidence and remaining integration](../../docs/knowledge/pwright-extra-http-headers.md).

NODE-077 expands `pnpm test:frame-navigation` with post-execution close and
timeout continuation: 27 paired cases match in both profiles, but the original
arrival-close case remains red (Rust also fails it intermittently). New
`pnpm test:frame-navigation-workers` passes 3/3 in each profile for owned pending,
borrowed pending and queued-result environment teardown. It uses the same
profile/CDP/base/bind variables and a diagnostic observer, not an upstream driver
in production. Run browser lanes serially; retained addon files are unchanged.
[Current evidence and limits](../../docs/knowledge/pwright-frame-navigation.md#node-077-causal-close-timeout-continuation-and-worker-teardown).

NODE-076 exposes `frame.goto(url, { waitUntil?, timeout?, referer? })` in the
explicit `frame-navigation` native profile. It uses Rust navigation/lifecycle
logic and returns a Response whose frame() is the exact navigated wrapper.
Context baseURL is supported; extraHTTPHeaders/referer conflicts, exact error
text and the full compatibility goal remain unfinished. Both new profiles pass
23 paired Frame cases and independent Rust close, but fail an upstream
held-response Page-close diagnostic deadline; the full lane is not green.
The combined profile also passes the existing drag/Keyboard/Locator 10/10 lane.
[Current scope, resource bounds and verification](../../docs/knowledge/pwright-frame-navigation.md#node-076-public-node-frame-navigation).
Earlier NODE entries below describe their historical artifacts, not current
selected-profile coverage.

NODE-065 adds development-only `pnpm test:frame-lifecycle`. Build the Rust
`frame_lifecycle_trace` example with `frame-lifecycle`, then supply absolute
`PWRIGHT_FRAME_LIFECYCLE_BINARY` and optional unused absolute
`PWRIGHT_FRAME_LIFECYCLE_REPORT`. The 93/93 normalized-state comparison does not
expose public Frame.goto or rebuild the addon.
[Build command and verification scope](../../docs/knowledge/pwright-frame-navigation.md#node-065-optional-normalized-lifecycle-reducer).

NODE-064 adds the development-only `scripts/probe-frame-navigation.mjs` oracle
and Rust native-initiation comparison. Six local/nested/OOPIF observations pass;
this does not add public Frame.goto or rebuild either Node addon. Full navigation
still requires Rust lifecycle/cancellation and Frame-specific response identity.
[Setup and design](../../docs/knowledge/pwright-frame-navigation.md).

NODE-063 adds `page.frame(name | { name?, url? })`, including globs, caller RegExp
and URL predicates, backed by Rust selection. Rebuild the selected addon first;
the historical default binary does not implement it. `pnpm test:frame-lookup`
passes 41/41 observations and three native groups. Configure loopback
`PWRIGHT_TEST_CDP`, verified serve-lib `PWRIGHT_FRAME_LOOKUP_FIXTURE` directory
and unused absolute `PWRIGHT_FRAME_LOOKUP_REPORT`. Glob text is capped at 16 KiB;
native bindings/read bytes remain explicitly bounded. This is not full Playwright
parity; Frame.goto, Frame events and connection reliability remain open.
Same-addon sequential typing passes 38/38 using sequential-typing.html.
[API scope and verification](../../docs/knowledge/pwright-frame-lookup.md#node-063-public-lookup-over-the-rust-kernel).

NODE-062 browser.newContext({ baseURL }) and Page.goto relative URL resolution
now pass 16/16 Playwright 1.58.2 observations plus four native boundary groups.
Context enumeration is cache-aware; pwrightContextMetadataMetrics() exposes its
separate ledger. Final-addon Page 7/7 and native glob 1647/1647 plus seven checks
also pass. Other Context options, Frame.goto and public page.frame remain
unsupported; connection reliability is still under investigation.
[API scope, quotas and verification](../../docs/knowledge/pwright-context-creation.md#verification-status-and-remaining-gates).

NODE-061: pnpm probe:attachment-commands runs a prebuilt attach_probe test binary
asynchronously across observer layouts. Supply its absolute
PWRIGHT_ATTACHMENT_TRACE_BINARY, loopback PWRIGHT_TEST_CDP and an unused absolute
PWRIGHT_ATTACHMENT_COMMAND_REPORT. The unchanged addon now passes original-layout
glob 1647/1647 and Page 7/7; this is not a fix for intermittent connection failures.
[Build command, limits and evidence](../../docs/knowledge/pwright-chromium-topology-events.md#node-061-reuse-the-bounded-attachment-collector).

NODE-060 adds internal native glob conversion and pnpm test:glob-ffi. It requires
PWRIGHT_CDP_ENDPOINT, an unused absolute PWRIGHT_GLOB_FFI_REPORT, and the rebuilt
selected addon. A labelled PWRIGHT_GLOB_OBSERVER_SESSION=after run passes 1647
patterns and seven boundary checks. Default before-layout and Page regressions
still fail during attachment; page.frame and Context baseURL remain unfinished.
[Boundary, setup and evidence](../../docs/knowledge/pwright-frame-lookup.md#node-060-actual-n-api-url-value-conversion-port).

NODE-059: `pnpm test:glob-base` compares 1647 Rust URL-glob patterns against
Playwright 1.58.2. Optional `PWRIGHT_GLOB_BASE_REPORT` must be an unused absolute
path. This offline helper verifies preprocessing with a diagnostic Rust parser,
not public `page.frame` or production parser allocation admission. Native
binaries remain unchanged.
[Contract and remaining gates](../../docs/knowledge/pwright-frame-lookup.md#node-059-bounded-glob-preprocessing-and-borrowed-url-resolution).

NODE-058: `pnpm test:glob-plan` builds/runs a bounded Rust-only helper and
compares 2810 resolved-glob patterns against Playwright 1.58.2. This is not public
`page.frame` support; N-API lookup, baseURL and identity integration remain open.
Future native builds include host MIT/Apache license texts and upstream notices
alongside the selected binary. Existing binary bytes were not changed here.
[Implementation and attribution](../../docs/knowledge/pwright-frame-lookup.md#node-058-rust-traversal-and-bounded-resolved-glob-compiler).

NODE-057: `pnpm test:lookup-oracle` runs 14 pinned-client characterization tests;
`page.frame(...)` is still unimplemented. [Design and remaining gates](../../docs/knowledge/pwright-frame-lookup.md).
Mouse now passes 44/44 on the unchanged addon in both original and experimental
`PWRIGHT_MOUSE_SETUP=background` modes. Original remains the default; the
earlier intermittent extra events are not explained by these successful reruns.
[Diagnostic scope and evidence](../../docs/knowledge/pwright-raw-mouse.md#node-057-setup-experiment-not-a-causal-fix).

NODE-056 adds `frame.name()` as a Rust-published navigation snapshot. Rebuild
and select `PWRIGHT_NATIVE_PROFILE=drag-runtime`; historical addons return an
explicit unsupported/rebuild error. `pnpm test:frames` now passes 18/18 shared
local/OOPIF cases, including names retained after detach/close. `page.frame`
lookup and Frame events are not yet supported.
Current Mouse regression has unexplained extra movement events and is not green;
the successful Frame gate must not be read as whole-package parity.
[Setup, bounds and evidence](../../docs/knowledge/pwright-frame-publication.md#node-056-bounded-navigation-names-and-public-framename).

NODE-055 adds admitted, cache-aware Page enumeration and explicit Rust-backed
transient activation for Page/Frame `evaluate`. The shared `pnpm test:pages`
gate passes 7/7, including popup discovery and retained closed Pages; repeated
enumeration does not accumulate native wrappers. The creation-pressure gate
now includes four states. Default historical addon bytes remain unchanged.
[Setup, bounds and evidence](../../docs/knowledge/pwright-frame-publication.md#node-055-admitted-page-descriptors-and-cache-aware-enumeration).

NODE-054 pre-admits new-Page root bindings in Rust, fixing a reproduced native
target leak under retained-Frame pressure. `pnpm test:page-binding` checks that
resource rejection creates no target and restores metadata charges. The rebuilt
selected addon passes Frame 10/10, Mouse 44/44 and drag/Locator 10/10.
`pnpm probe:attachment` is a bounded diagnostic matrix, not a passing parity
gate; earlier intermittent connection timeouts remain under investigation.
[Setup, artifacts and evidence](../../docs/knowledge/pwright-frame-publication.md#node-054-creation-time-root-binding-admission).

NODE-053 adds `page.frames()`, `frame.parentFrame()`, `frame.childFrames()` and
`frame.isDetached()`. Rebuild with `PWRIGHT_NATIVE_PROFILE=drag-runtime pnpm build`
and select that same profile when running. The old default binary remains
untouched and explicitly rejects these new methods until rebuilt. The shared
`pnpm test:frames` gate passes 10/10 local/OOPIF lifecycle observations; names,
lookup and Frame events are still unsupported. `browser.pwrightFrameTreeMetrics()`
reports retained native states, revisions/read guards and logical bytes.
[Current API and verification](../../docs/knowledge/pwright-frame-publication.md#node-053-public-node-frame-collections-and-retained-wrappers).

NODE-052 fixes owned-Page OOPIF attachment in Rust and rebuilds only the selected
`drag-runtime` artifact. Public owned-OOPIF navigation now succeeds; Mouse 44/44,
Keyboard 24/24 and drag/Locator 10/10 pass on that exact binary. The host now
maintains bounded Frame trees, but `page.frames()` and public child-Frame wrappers
remain unsupported. The default addon is unchanged.
[Artifact and native evidence](../../docs/knowledge/pwright-frame-publication.md#node-052-live-host-projection-and-paused-oopif-attachment).

NODE-050 adds the opt-in `PWRIGHT_NATIVE_PROFILE=drag-runtime` local build and
loader. Build with that environment variable; the default binary is preserved.
A Rust capability export verifies the selected artifact on first native use.
Actual Node results: Mouse 44/44, Keyboard 24/24, drag/Locator 10/10. The profile
remains experimental, and public child-Frame APIs are still unsupported.
[Selection, evidence and remaining work](../../docs/knowledge/pwright-chromium-drag.md#node-050-native-lifetime-gates-and-selected-node-addon).

NODE-049 fixes Rust-host Frame input lifetime and verifies 12/12 native cases.
The package addon remains unchanged and does not select `drag-runtime`; these
are not new public Node child-Frame or drag-compatibility passes.
`pnpm test:boundary` now checks default, drag-source and drag-runtime closures
and their resolved runtime feature selection, as well as the thin JS boundary.
[Contract, evidence and remaining gates](../../docs/knowledge/pwright-chromium-drag.md#node-049-optional-frame-to-page-lifetime-handoff).

NODE-048 adds internal Rust-host native child/OOPIF tests, not public child-Frame
APIs. Eight of ten cases match; post-focus iframe removal still cancels Rust
input too early. The addon remains unchanged and does not select drag.
[Open gate](../../docs/knowledge/pwright-chromium-drag.md#node-048-native-frame-gate-exposes-a-lifetime-discrepancy).

NODE-047 verifies ten selected Rust-host drag/Keyboard/Locator scenarios.
This package's addon is unchanged and still does NOT enable `drag-runtime`.
The NODE-045 public Mouse result remains 39/44, not ten new public Node passes.
[Selection gates](../../docs/knowledge/pwright-chromium-drag.md#node-047-locator-keyboard-selects-the-owning-page).

NODE-046 adds selected Rust-host Keyboard Escape cancellation, verified against
five native oracle cases. This package's addon still does NOT select drag and
was not rebuilt; NODE-045 public Mouse evidence remains 39/44.
[Remaining selection gates](../../docs/knowledge/pwright-chromium-drag.md#still-required-before-public-drag-selection).

NODE-045 rebuilds this package's default addon with 552 pending CDP commands,
derived from admitted Mouse batches plus the existing non-Mouse allowance.
Public Mouse is 39/44; all 25 raw cases match. `drag-runtime` is still unselected.
[Exact artifact and remaining gates](../../docs/knowledge/pwright-chromium-drag.md#node-045-verification-and-actual-node-artifact).

NODE-044 implements an opt-in `pwright-node-host/drag-runtime` integration, but
this package's addon still uses the default host. Do not infer Node drag support
from host-native tests; public Mouse evidence remains **36/41**.
[Host selection gates](../../docs/knowledge/pwright-chromium-drag.md#remaining-compatibility-gates).

NODE-043 adds an opt-in host `drag-source`: fresh Page utility snapshots now
remain owned through movement results, retirement and quarantine. Exact world
destruction drains started listener requests without issuing new cleanup there.
Default/selected host closures are **234/235**; only the drag leaf is added.
HostSession Mouse/close/Escape selection and public differential checks remain
pending; the addon and public **36/41** baseline are unchanged.
[Production source and evidence](../../docs/knowledge/pwright-chromium-drag.md#node-043-retained-production-page-drag-source).

NODE-042 adds immediate, budgeted snapshots of a Page's existing utility worlds,
including child Frames/OOPIFs without waiting for a main world. Utility-only
native destruction receipts retain snapshot admission and remain distinct from
route invalidation and owned-session retirement. Production drag-source/host
integration is still pending; no addon rebuild or public **36/41** change.
[Snapshot ownership and verification](../../docs/knowledge/pwright-chromium-drag.md#node-042-existing-page-utility-world-snapshots).

NODE-041 implements the explicit `mouse-dispatch` feature and Page-owned cleanup
driver: **82** unit tests and **4** selected-Mouse native cases pass, alongside
the prior **10** native cases. Default/feature closures are **3/5** Pwright
packages. Host Frame-source/lifecycle and Keyboard Escape integration remain
pending; no addon rebuild or public **36/41** change.
[Selected dispatcher and evidence](../../docs/knowledge/pwright-chromium-drag.md#node-041-selected-mouse-dispatcher-and-page-cleanup-driver).

NODE-040 adds Page-owned drag state and retained native enter/over/drop/cancel.
**70** unit tests and **10** native scenarios pass, including actual drop and
cancel with dragend checks. The optional leaf remains **3** Pwright packages;
Mouse dispatch/forClick/down suppression, Keyboard Escape and Node host selection
are still pending. The addon/public **36/41** evidence is unchanged.
[Page state and verification](../../docs/knowledge/pwright-chromium-drag.md#node-040-page-drag-state-and-native-event-owners).

NODE-039 adds composed intercepted movement to the optional Rust leaf (**57**
unit tests; **9** native cases). It is not selected by this Node package. Page
drag state, Mouse adaptation and host teardown remain pending; the addon and
public **36/41** result are unchanged.
[Source checkpoint](../../docs/knowledge/pwright-chromium-drag.md#node-039-composed-intercepted-movement).

NODE-038 adds retained subscription/interception ownership in the optional Rust
drag leaf, with **44** unit tests and **6** native listener/capture cases passing.
It is not selected by this package; the addon and public **36/41** evidence are
unchanged. [Source checkpoint](../../docs/knowledge/pwright-chromium-drag.md#node-038-retained-capture-lifecycle).

NODE-037 adds bounded typed drag-data storage to the optional Rust leaf, not this
package's active host. Unit and native data-flow checks pass; the addon is not
rebuilt and public Mouse remains **36/41**. Capture lifecycle and Page-scoped
integration are still pending.
[Source checkpoint](../../docs/knowledge/pwright-chromium-drag.md#node-037-admitted-intercepted-data).

NODE-036 adds an optional Rust-owned drag-listener helper, verified on real
Chrome. It is not installed in this package; the addon is unchanged and HTML
drag/public concurrency compatibility remains incomplete.
[Native helper and remaining integration](../../docs/knowledge/pwright-chromium-drag.md).

The native factory and Page route now support a selected Mouse dispatcher
(NODE-035), but the host still uses its direct binding. A real Page-scoped drag
provider and host teardown integration remain pending. No addon is rebuilt or
public discrepancy resolved by this checkpoint.
[Current source integration](../../docs/knowledge/pwright-raw-mouse.md#page-scoped-dispatcher-selection-node-035).

The Rust input scheduler now exposes an optional cancellation-aware Mouse
dispatch seam (NODE-034). It is not selected by the public host and no addon
is rebuilt for that source checkpoint. It does not resolve the drag differences
below. [Ownership and remaining integration](../../docs/knowledge/pwright-raw-mouse.md#optional-mouse-dispatch-ownership-node-034).

The expanded same-turn Mouse corpus is **not green**: **33/41** on the prior
addon, **36/41** after Rust first-poll ordering. Five left-held-move cases still
differ because the optional Chromium drag path is missing. The original **26/26**
below covers only the earlier corpus. Do not serialize the whole click or add JS
delays to make these cases pass. [Current concurrency checkpoint](../../docs/knowledge/pwright-raw-mouse.md#same-turn-input-ordering-node-033).

Final ordering addon `e65e6d85...17b246`: Mouse **36/41**, Mouse Worker/profile
**3/3**, Keyboard **24/24**, typing **30/38**, actions **24/27**, screenshot
**12/12**, lifecycle **45/45**. The typing/action failures are attachment timeouts
on both providers; do not carry over the intermediate artifact's green results.
Reload/shared were not rerun on this final artifact. [Exact gates](../../docs/knowledge/pwright-raw-mouse.md#final-artifact-gate).

Earlier release addon `e9d9e543...109535` introduced stable `page.mouse` with
move/down/up/click/dblclick/wheel; all input state and scheduling remain in Rust.
Public comparison: **26/26**. Profile rejection and owned/borrowed Worker teardown:
**3/3**, including no synthetic release on the surviving borrowed Page.
Run `pnpm test:mouse` with explicit dedicated `PWRIGHT_TEST_CDP`, verified serve-lib
`PWRIGHT_MOUSE_FIXTURE` and a new absolute `PWRIGHT_MOUSE_REPORT`; use
`pnpm test:mouse-native` for the profile/Worker lane.
[Limits, artifact and reproduction](../../docs/knowledge/pwright-raw-mouse.md#public-node-mouse-checkpoint).
Host dependency closure remains **234**. Nested allocation audit, HTML drag,
exact error fidelity and full artifact-specific regression remain open. This
does not imply real Touchscreen support.

On this Mouse addon: Keyboard **24/24**, typing **38/38**, actions **27/27**,
screenshot **12/12**, lifecycle **45/45**, reload **16/18**. Shared 57 cases:
**53 match / 2 unsupported / 1 oracle-error / 1 setup-error**. The renewed 5 s
topology-attach timeout and both original reload-close failures remain open;
overall compatibility is red. [Exact evidence](../../docs/knowledge/pwright-raw-mouse.md#same-artifact-regression).

Full rerun on the Keyboard addon: **38/38 typing**, **27/27 actions**,
**12/12 screenshot**, **45/45 lifecycle**, **16/18 reload**. Held-modifier typing
entries are no longer filtered out. Two original reload-close failures remain;
intermittent attachment/window issues were not reproduced or proven fixed.
[Exact evidence](../../docs/knowledge/pwright-raw-keyboard.md#same-artifact-post-keyboard-regression).

Previous Keyboard addon `1319fa02...ded25dc1` exposes `page.keyboard.down/up/press/
insertText/type` with Rust-owned Page state, routing, delay and cancellation.
Public comparison is **24/24** and shared coverage **52 match / 3 unsupported /
2 oracle-error**, no other errors. Run `pnpm test:keyboard` with dedicated
`PWRIGHT_TEST_CDP`, verified serve-lib `PWRIGHT_INPUT_FIXTURE`, and an absolute
unused `PWRIGHT_KEYBOARD_REPORT` file. Key/text limits are 128 bytes/1 MiB,
physical key capacity 16, and unpaired UTF-16 rejects explicitly.
[Artifact and budgets](../../docs/knowledge/pwright-raw-keyboard.md).
Earlier artifact-specific reload/lifecycle/screenshot results below remain
historical; the full gates were not rerun by this Keyboard checkpoint.

Playwright-shaped ESM/CommonJS API backed by the Pwright Rust v2 host through
Node-API. No Playwright runtime dependency, Node driver subprocess, browser
download, or browser launch. This package is private and not published.

`page.reload({ waitUntil, timeout })` now invokes Rust navigation 1.1, with a
separate native Reload command and the existing FFI admission/publication path.
Both original shared reload scenes match: corpus **48 match / 7 unsupported /
2 oracle-error**. The new public reload lane is **16/18** (response-arrival close
fails on both sides; the separate post-commit close passes), and lifecycle is
**44/45** (window geometry).
The same artifact's screenshot gate is **0/12**, failing at attachment before
image comparison; earlier green screenshot runs do not supersede this result.
Ordinary/POST/timeout success is not complete lifecycle compatibility.
Run `pnpm test:reload` with CDP/base and `PWRIGHT_TEST_BIND` as for `test:live`.
[Contract, exact artifact and red evidence](../../docs/knowledge/pwright-page-reload.md).
The separate `scripts/probe-reload-boundary.mjs` launches a test-only Rust host
diagnostic with explicit CDP/Tailscale settings. Its four native controls track
bounded request/loader events and target presence; a successful diagnostic run
does not mean reload-close compatibility passes.
[Commands and causal findings](../../docs/knowledge/pwright-page-reload.md#causal-native-close-diagnostic).
Pass `raw` to run the standalone CDP reproducer with no host or addon, additionally
setting the verified loopback `PWRIGHT_CDP_WS_URL`. It retains the original fixture
and separately samples native target presence; it is not a workaround or a parity
pass. [Raw isolation](../../docs/knowledge/pwright-page-reload.md#minimal-raw-cdp-isolation).

`Locator.pressSequentially(text, { delay, timeout })` and deprecated
`Locator.type()` call the Rust typing capability. Node explicitly selects
interleaved Page input state; the new artifact passes **30/30 typing + 27/27
action** tests, including overlap and the prior delayed-Press Shift mismatch.
Physical state survives command timeout instead of automatically releasing all
keys. It remains fixed at 16 physical codes per Page; unpaired UTF-16 rejects.
These APIs do not claim full keyboard, native cancellation or string compatibility.
[Typed contract, limits and evidence](../../docs/knowledge/pwright-sequential-typing.md).

Current screenshot checkpoint: `page.screenshot()` returns a Rust-captured,
Node-owned Buffer. PNG/JPEG, quality, fullPage, clip, scale and timeout are exposed;
the recursive-attachment artifact now passes all 12 decoded-pixel/resource/Worker
tests and all 45 lifecycle tests. The host has an eight-call finite
queue and a 2 MiB image ceiling. Path, rendering-state options, combined fullPage/
clip and default font/caret fidelity remain incomplete. After the dedicated
Chrome/Pwright preflight and fixture setup, run `pnpm --dir packages/pwright
test:screenshot` with the same `PWRIGHT_TEST_CDP` and `PWRIGHT_TEST_BASE` variables
as the other live lanes. [Screenshot evidence and limits](../../docs/node-ffi-compatibility-design.md#node-page-screenshot-binding-and-ownership-checkpoint-2026-09-07).

Prior checkpoint: owned-session retirement is wired into host shutdown.
The full 23-case Locator evaluation lane and native diagnostic supervisor pass,
including preservation of a borrowed Page and its running Promise when a Worker
terminates (NODE-027). Compatibility is still incomplete: NODE-013 attach timing,
NODE-014 window dimensions, NODE-022 keyboard behavior and unsupported APIs remain.
The latest lifecycle attempt failed a window comparison and reached its unchanged
total deadline. [Current evidence and resource contract](../../docs/node-ffi-compatibility-design.md#node-027-host-integration-and-worker-verification-2026-09-07).

## Previous checkpoints (historical)

Complete compatibility is **not green**. Previous host-owned rebuild `8cc20b9e...6aad0` passes
Locator 32/32, evaluation 38/38 and lifecycle 45/45; actions are 25/26 (NODE-022).
All 57 shared cases finish with 41 matches / 14 unsupported / 2 oracle errors,
and no setup/value/candidate/cleanup errors. The safe host now owns Locator
selection/adoption and delayed conversion inside one controlled command, but
its initial native Context-close cleanup failure (NODE-026) is now fixed with
optional native-destruction receipts. Two earlier safe-host Chrome runs pass 5/5;
the final epoch-guarded run is 4/5 due to a 5 s attach setup failure before queue
assertions. Context cleanup still passes; an isolated queue diagnostic passes
without replacing that failed gate. Connection timing remains unresolved.
Public Node `Locator.evaluate` is now implemented over that Rust host, including
delayed source/getter conversion and binary staging. Its dedicated lane passes
22 assertions but still fails the native diagnostic gate when terminating a
Worker during a never-resolving browser evaluation (NODE-027). It is experimental,
not a fully verified compatibility claim. No silent retry or Chrome restart was used.
[Current FFI evidence](../../docs/node-ffi-compatibility-design.md#public-locatorevaluate-ffi-checkpoint-2026-09-07).

On this new addon, existing evaluation/Locator/lifecycle regressions pass
38/38, 32/32 and 45/45. All 57 shared cases now finish with 43 matches /
12 unsupported / 2 oracle errors; both label observations newly match. That
shared run has no native cleanup failure, but it does not exercise or clear
NODE-027. No complete-compatibility or memory-improvement claim is made.
[Evidence and next gates](../../docs/node-ffi-compatibility-design.md#native-destruction-receipt-checkpoint-2026-09-07).

## Build and test

Final native-retirement rebuild `930469ad...a7a4a3` passes 442 Rust release
tests, but its public lanes remain non-green: Locator 29/32, evaluation 17/38,
lifecycle watchdog after 11 passes / 5 failures, actions 25/26 (NODE-022).
The full shared corpus is 41 matches / 14 unsupported / 2 oracle errors, with
zero value/candidate/setup/cleanup errors. Attachment failures affect both
implementations and remain unclassified; the shared subset does not replace
the failed public lanes. That historical rebuild predates public Locator.evaluate.

From this directory, with Rust and Node installed:

```sh
pnpm install --frozen-lockfile
pnpm build
pnpm test
pnpm test:types
pnpm test:boundary
pnpm test:native-load
```

The default build compiles a release addon and atomically replaces
`native/pwright.node` (gitignored). To build the experimental selected profile
without replacing that file:

```sh
PWRIGHT_NATIVE_PROFILE=drag-runtime pnpm build
PWRIGHT_NATIVE_PROFILE=drag-runtime node your-script.mjs
```

For Frame navigation without drag, use `PWRIGHT_NATIVE_PROFILE=frame-navigation`
for both build and execution. For both capabilities, use
`PWRIGHT_NATIVE_PROFILE=drag-frame-navigation`. Each profile has a separate
`native/<profile>/pwright.node`, and both compiled flags must match the selected
profile. Default and drag-only files are not overwritten by these builds.

`pnpm test:frame-navigation` uses repository `../../tests/fixtures` (not this
package's fixture directory), registered via serve-lib. Set the selected native
profile, loopback `PWRIGHT_TEST_CDP`, verified full-MagicDNS `PWRIGHT_TEST_BASE`
and explicit Tailscale `PWRIGHT_TEST_BIND`. The pinned oracle is development-only.

This writes `native/drag-runtime/pwright.node`; the same environment selection is
required when running it. Selection is fixed when the package is first imported.
Unknown profiles, missing selected binaries, ABI/core-export mismatches and
mismatched compiled capabilities fail; there is no download or fallback to
another profile. With the documented
dedicated Chrome preflight and verified serve-lib fixture/report variables set,
`PWRIGHT_NATIVE_PROFILE=drag-runtime pnpm test:drag-keyboard` runs the new shared
native drag/Keyboard/Locator Node comparison. It needs `PWRIGHT_TEST_CDP`,
`PWRIGHT_DRAG_FIXTURE` and a new absolute `PWRIGHT_DRAG_KEYBOARD_REPORT` path.

Local validation used Node 26.7.0 on macOS arm64. The intended
Node >=20 range and other platforms are not yet verified.

For the real-browser comparison, install the repository's pinned oracle first:

```sh
pnpm --dir ../../tests install --frozen-lockfile --ignore-scripts
chrome-cdp start
chrome-cdp status
```

Register `test/fixtures` with the existing `serve-lib` daemon using an explicit
Tailscale bind IP, route, port and lifetime. Verify the exact full-MagicDNS URL
with `serve-lib list` and `curl -I`. Static HTML must not use another server.
For example, this Mac's verified fixture route is
`http://macbookpro-yuacx.tail8f3b66.ts.net:9384/pwright-node-actions` while its registration
is active. `PWRIGHT_TEST_BIND` is the verified Tailscale IP; it is used only for
ephemeral dynamic stalled-response test endpoints, which are closed afterward.

In a task-specific working directory, run the mandatory initial Pwright open
against that fixture URL:

```sh
pwright --cdp http://127.0.0.1:9222 open http://macbookpro-yuacx.tail8f3b66.ts.net:9384/pwright-node-actions/index.html
```

Then return to this package and run:

```sh
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 \
PWRIGHT_TEST_BASE=http://macbookpro-yuacx.tail8f3b66.ts.net:9384/pwright-node-actions \
PWRIGHT_TEST_BIND="$TAILSCALE_IP" pnpm test:live
```

The same CDP/base configuration also runs `pnpm test:evaluation`,
`pnpm test:locator`, `pnpm test:locator-evaluation` and `pnpm test:actions`
(no dynamic bind variable needed). Locator evaluation covers delayed conversion,
timeout scope, error identity, binary copying and four Worker teardown phases.
Its supervisor rejects native cleanup diagnostics even if every TAP assertion passes.
The Locator supervisor enables explicit GC only in its test child. All package
lanes use TAP so failure details survive a later supervisor deadline kill.
The lifecycle lane has a 600-second aggregate watchdog and an independent
60-second bound for each case (setup/action/cleanup), startup and process exit.
Other lanes retain 180 seconds total. Original API deadlines stay unchanged.
Bounded start/end IPC drives the case watchdog; fixture-phase TAP comments cannot
renew it. At most 128 cases, 4096 phase records and 4 MiB stdout are accepted;
stderr diagnostics and native panic/cleanup failures remain supervised.
The unchanged interleaved-input addon now completes all **45/45 lifecycle cases
in 192.715 seconds** with this policy; the prior traced 180-second run was killed
after 44 passes. Earlier attachment/window failures remain open.
[Evidence](../../docs/node-ffi-compatibility-design.md#node-013-lifecycle-supervision-evidence-2026-09-07).
Run all package lanes and the shared corpus serially against one CDP endpoint;
separate working directories isolate Pwright state, not browser-wide target
discovery. Overlapping supervisors produced missing-session/timeout failures
while attaching, recorded in the paired-world access checkpoint. Serialization
is the test protocol, not a fix or a compatibility claim for concurrent clients.
The preceding concurrency addon passed Locator 22/22, lifecycle 44/44 and
evaluation 38/38. The action-stage addon then reproduced setup timeouts and a
ten-read close/fence overflow (NODE-023); the latter now has a scoped Rust fix
and completed Locator 22/22, evaluation 38/38 and lifecycle 44/44 reruns on
`e5076407...65d48`. Actions remain 22/24 with NODE-021/022. These are supported subsets, not full
Playwright gates. Artifact-specific evidence and retained failures are in the
[Node design record](../../docs/node-ffi-compatibility-design.md).
No failures are skipped or silently retried.

The preceding topology rebuild `72644cf9...b9ddcd` completes a serial run of Locator **32/32**,
evaluation **38/38**, lifecycle **44/45** (NODE-014) and actions **25/26** (NODE-022).
The unchanged shared corpus completes **41 matches / 14 unsupported / 2 oracle
errors**, with zero value/candidate/setup/cleanup errors and no supervisor
timeout or native failure. Lifecycle, action and shared gates remain nonzero.
An earlier overlapping package/shared run on this same binary failed with
additional attachment errors: Locator 27/32, evaluation 34/38, lifecycle 43/45
and shared 11 setup errors. Both runs are retained; a serial pass does not
resolve concurrent attachment or the older navigation discrepancies.
See the [preceding artifact evidence](../../docs/node-ffi-compatibility-design.md#production-paired-world-access-checkpoint-2026-09-07).
The independent Rust strict-wait and main-world adoption/evaluation leaves are
now installed by the safe host, whose normal closure is 224 packages. Node
`Locator.evaluate` was unavailable at that checkpoint; it is now exposed with
correctly timed FFI conversion and an open running-cancellation cleanup gate.

At the earlier Rust-only checkpoint, paired preparation passed release and real Chrome
gates, including retained route ownership through cancelled native cleanup.
It then remained outside this package's 221-package normal closure. That
checkpoint did not rebuild the addon or rerun the five full browser lanes; it
is historical evidence, not the current installation. See
[the preparation record](../../docs/node-ffi-compatibility-design.md#paired-element-preparation-checkpoint-2026-09-07).

This explicitly authorized loopback CDP endpoint controls the dedicated Chrome
automation profile, never a normal user profile. Tests do not close preexisting
pages. Test cases create and close their own fixture pages. Close the preflight
page using `pwright --cdp http://127.0.0.1:9222 close` in its unchanged task
working directory after verifying its active target is the owned preflight. Verify
that the target is absent from CDP discovery afterward. Do not reset
the shared automation browser. The oracle is pinned to Playwright 1.58.2 by the
existing `tests/pnpm-lock.yaml`, and a different version fails the lane.

## Supported today

### Attachment diagnostics

After the same dedicated Chrome/Pwright preflight, run from the repository root:

```sh
CARGO_INCREMENTAL=0 cargo test --release -p pwright-node-host --test attach_probe --no-run
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm --dir packages/pwright exec node scripts/probe-attach.mjs
```

For the separate retained-world investigation:

```sh
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm --dir packages/pwright exec node scripts/probe-retained-worlds.mjs
```

This probe creates, reloads and closes one owned blank Page, then disconnects its
own connections. It observes bounded context metadata on that Page across six
fixed candidate/oracle attachments; browser-wide attachment can still initialize
other existing targets. It stops on failure without retry and does not change
the five-second connection timeout. A completion record proves that reload
created a different main world. Context counts do not measure heap/RSS or prove
init-script retention. The current result shows one reused Pwright world and
three added oracle worlds across three reconnects each, with no NODE-013 root
cause claim. See [the evidence and limits](../../docs/node-ffi-compatibility-design.md#node-013-retained-world-diagnostic-2026-09-07).

The original attachment diagnostic compares Node main-thread, Worker and independent Rust
host attachment while one pinned Playwright connection remains open. It does
not create or close Pages, retry a failed attach, raise its five-second budget,
or change the conformance suite. Cleanup and auxiliary CDP observations have
explicit diagnostic deadlines. A failure keeps the command nonzero.

The Rust probe records at most 32 unanswered commands and the latest 128 replies,
with method/session/target names capped at 128 bytes, plus counts for at most 64
event methods and 64 deduplicated session-to-target bindings. A separate counter records pre-admission omissions
without mixing them into delivered event counts. It retains no console, script
or page-content payloads. Its subscriber is test-only; ten offline tests verify
recording bounds, conflicting bindings, method counts and omission separation.

A separate prerequisite-only ABBA diagnostic compares widths 1/2/2/1 with no
production scheduling change. After prebuilding `--test attach_pair_probe
--no-run`, supply `PWRIGHT_TEST_CDP` and `PWRIGHT_TEST_WS` (the exact dedicated
browser WebSocket reported by `chrome-cdp status`) to
`pnpm --dir packages/pwright exec node scripts/probe-attach-pairs.mjs` from the
repository root. It only enables session-local Page/Runtime domains and reads
frame metadata on two to eight existing Pages; it detaches its own sessions and
never creates worlds, navigates or closes Pages. It is not a full-host/Node
benchmark. The first fixed experiment showed no improvement with one slow Page:
serial 555–567 ms, paired 587–591 ms. [Evidence and limits](../../docs/node-ffi-compatibility-design.md#node-013-target-correlation-and-bounded-pair-experiment-2026-09-07).

NODE-013 reproduced outside Node-API too. The first standalone host probe took
64 ms; with the oracle connected, both Node lanes and the independent Rust host
hit five seconds. A controlled TCP_NODELAY experiment showed no consistent
improvement and was reverted. The latest paired-prerequisite diagnostic reports
roughly 1.7–1.9-second overlapping attachments with three Pages. Command traces
prove the two prerequisites overlap; changing live activity prevents a controlled
performance attribution. The full regressions below were independently rerun;
neither a passing probe nor those tests proves full compatibility or root cause.
See the [diagnostic record](../../docs/node-ffi-compatibility-design.md#node-013-attachment-diagnostic-checkpoint-2026-09-06).

### Compatibility checkpoint

The recursive-attachment addon (`31333abc…45cdd9`) passes **12/12 screenshots**,
including both Worker settlement/Buffer cases, and **45/45 lifecycle tests**.
The full shared corpus completes with **45 match / 10 unsupported / 2 oracle-error**
and no value/candidate/setup/cleanup errors; its supervisor remains non-passing.
Earlier 3/12, 8/10 and 10/10 results belong to different artifacts and remain in
the record. [Current verification](../../docs/node-ffi-compatibility-design.md#node-013-recursive-attachment-prerequisite-join-2026-09-07).

This host now explicitly joins recursive auto-attach with Frame Tree, Runtime
enable and empty utility-world registration. Context settings/Page enable remain
earlier; all replies precede world creation, resume and Page publication. Other
Rust profiles keep their previous order. The extra declared response expansion
is 16 MiB, not a measured RSS increase or an allocated buffer. Three-Page live
traces prove command overlap but changing browser activity prevents a controlled
speedup claim or closure of intermittent NODE-013.

An earlier Node profile opted into a bounded Frame Tree/Runtime initialization
pair. Default Rust consumers remain serial; utility worlds, Context defaults and
PageReady retain their fences. This adds one declared response expansion, not a
Node driver or task. Test fixtures now pass their owner's URL to Workers and
probe titles only at that URL, retaining the unique/readable marker assertion.
See the latest Node design record for verification; prior full-gate failures
remain evidence until explicitly superseded by a completed rerun.

The package-local lifecycle/resource lane above is separate from the broader
[shared API corpus](../../tests/README.md). From `tests/`, `pnpm test:node-ffi`
runs all 19 existing API suites / 57 cases on fresh owned Contexts, using this
package's public API without a Playwright or JavaScript browser-logic fallback.
The preceding action-stage addon (`eae32ce9...41af9e`) completed the shared run with
36 matches, 19 unsupported cases and 2 official oracle history-navigation
errors (NODE-015), with no value mismatch or candidate/setup/cleanup errors.
Locator reads added ten matches over the earlier evaluation 12/43/2 checkpoint;
public actions add another fourteen over the read/concurrency 22/33/2 result.
The final terminal-completion addon (`e5076407...65d48`) completes all 57 cases
with 31 matches, 19 unsupported, two oracle errors and five attachment setup
errors. No current setup failure inherits an earlier artifact's passing result.
It intentionally exits nonzero; unsupported and oracle-error
cases are not passes. Artifact hashes and prior failures remain in the design record.
Earlier package-local passes do not mean all of Playwright is supported.

## Public subset

With the explicit handles native profile, Page/Frame.evaluateHandle and
JSHandle.evaluate/evaluateHandle/jsonValue/getProperty/dispose preserve native
object identity; ElementHandle.asElement distinguishes DOM results. getProperty
preserves the property's type, including a Promise, unlike evaluateHandle's
awaited evaluation result. The new producer lane is intentionally non-green
until getProperties has an atomically owned batch path. Preview/toString,
ownerFrame/adoption and retained ElementHandle actions/uploads remain absent.

```js
import { chromium } from '@shuozeli/pwright';
// CommonJS: const { chromium } = require('@shuozeli/pwright');

const browser = await chromium.connectOverCDP('http://127.0.0.1:9222');
try {
  console.log(browser.version());
  const context = browser.contexts()[0];
  const page = await context.newPage();
  try { console.log(page.mainFrame().page() === page); } // true
  finally { await page.close(); } // close exactly the Page this code created
} finally {
  await browser.close(); // disconnects; does not terminate external Chrome
}
console.log(browser.pwrightMetrics()); // Pwright extension: teardown counters
```

The endpoint-object overload (`{ endpointURL, timeout }`) is also supported.
Browser, Context, Page and main Frame identity are stable while live. Repeated
or concurrent browser close is idempotent and emits `disconnected` once.

No-options `BrowserContext.newPage()` and ordinary `Page.close()` are supported,
including `runBeforeUnload: false`. Creation, metadata admission, native target
initialization and close all execute through existing Rust services. Explicit
lifecycle calls share an eight-entry serial queue with topology fences. A new
Page reserves its main Frame's metadata entry and URL bytes before creating a
native target. Retained closed wrappers still consume that metadata admission.
Close suppresses only target-closed errors, not capacity/backend failures.

`Browser.newContext()` / `newContext({})` now create an isolated owned Context;
`context.close()` disposes its native Context and descendants. Repeated and
concurrent close are idempotent. Closing the borrowed default Context instead
disconnects this attachment, matching Playwright CDP without closing external
Chrome. Browser disconnect and forced Worker teardown remove the exact owned
native Context/Page, verified through Chrome's inventory.

Owned Contexts select a Rust pre-resume initializer for the default 1280x720
viewport/screen, desktop scale, native window size, focus and media preferences.
The initial document records these values before an observer could alter them.
Non-empty Context options, accepted-download/artifact initialization and
headless-font defaults are not implemented; see NODE-014. This is not full
default/options parity merely because `newContext()` returns successfully.

Like Playwright's CDP attachment, this facade attributes Pages from Contexts
unknown to the connection to its default Context. This does not acquire or own
those foreign Contexts. Do not use that view as evidence of a private Context
boundary. The reusable Rust topology driver retains strict exclusion by default;
only this compatibility host explicitly enables the fallback behavior.

`Page.goto` now supports `commit`, `domcontentloaded`, `load`, and `networkidle`;
the returned response exposes URL/status/status text/headers/ok/frame metadata,
not bodies, request objects or completion of body streams. `Page.title/content`
and main Frame `title/content` run in Rust. Timeouts and browser-close cancellation
use the controlled v2 dispatcher and bounded cleanup executor.
Worker teardown joins Rust cleanup before Node destroys its runtime. The live
suite verifies that terminating a Worker during navigation cancels its pending
HTTP request while leaving the external browser alive. Native cleanup failures
fail the runner even when Node assertions pass. A prior rebuilt 44-case lane
passed all cases, including native URL metadata, prototype interference and
concurrent DOM reads, blocked-JS native-result pressure (over 800,000 UTF-8
bytes), independent close admission, undelivered-result Worker teardown and
URL visibility while the JS event loop is blocked, plus queued-URL reclamation
when a Worker is forcibly terminated. Lifecycle cases additionally compare
initialized new Pages, concurrent creation, idempotent close after external
closure/disconnection, Page-close navigation cancellation and rejection of
unsupported options before mutation. It also verifies Context isolation,
descendant navigation cancellation and native cleanup on disconnect/Worker exit.
An earlier lifecycle rerun reported 17 passes and one Worker failure before the
180-second supervisor killed the incomplete lane; it is not a 44-case pass.
Earlier 30-second attachment failures and subsequent 5-second topology-phase
timeouts remain NODE-013. Later passing runs do not establish their cause or
resolution. The diagnostic record preserves this incomplete run and its exact
test-owned Page cleanup without restarting shared Chrome.

Synchronous `Page.url()` and main `Frame.url()` read a Rust-owned published view,
including external navigation, history/hash changes and the last URL after
close. The separate `browser.pwrightMetadataMetrics()` counts retained metadata
even when live-session counters are zero. Each host admits 128 metadata records,
16 KiB per URL and 2 MiB total URL bytes, including native/published/queued
overlap, with at most 256 unpublished changes. `pendingPublications` reports
that queue independently from FFI task counters. Fixed watermarks publish only
on Node callbacks, so native receipt while JS is blocked does not change a
synchronous URL getter within the same turn. This behavior now matches the
paired Playwright fixture; ordering with future public events remains open.
`frame.pwrightMetadataMetrics().unpublishedUrl` identifies a native URL revision not
yet published for that exact Frame; these metrics are Pwright extensions.
Title uses the Rust
`FrameTitle` fixed read in the utility world, sharing bounded DOM-read routes
and result ownership with content. The NODE-010 discrepancy is resolved and
remains a mandatory regression test. Title does not depend on public evaluation;
the host now installs that capability explicitly for `evaluate`. Reads have no action timeout; native CDP command budgets
still apply and remain an explicitly documented compatibility boundary.

`Page.evaluate` and main `Frame.evaluate` support string expressions or functions
with one argument, Promise results, special scalars, cyclic/shared objects and
arrays, Date/URL/RegExp/Error values and eleven standard typed-array kinds. Browser
execution uses the existing Rust capability in the main world; the facade does
not execute user functions in Node or delegate them to Playwright. Argument and
result trees have explicit admission, including 32 KiB source, depth 64, 4,096
nodes/entries and separate 256 KiB aggregate string/binary limits. Each evaluation
reserves 8 MiB of FFI payload allowance through JS conversion, within the same
64 MiB environment cap. A two-phase Node-thread conversion snapshots ordinary
object structure immediately and copies typed-array bytes at the next injected
`setImmediate`, before any Rust browser execution. Captured fixed views have a
1 MiB aggregate backing-capacity ceiling per evaluation, including the full
buffer `maxByteLength` behind small subarrays. Shared backing aliases are charged
conservatively per view. No JS reference or borrowed pointer crosses a Rust await.

`browser.pwrightConversionMetrics()` reports environment-wide `pending`,
`binaryReferences` and `backingBytes` for this prepared phase. They are distinct
from active browser work or queued Rust results; FFI `running` already includes
admitted prepared calls. Getter failures release captured
views; Worker cleanup clears unstarted conversions before draining Rust tasks.
Blocked-JS and forced-Worker tests cover prepared input and running/queued output.

This is not full evaluation parity. The NODE-017 comparisons now match immediate,
microtask and later-immediate mutation, later-getter mutation, detachment,
resizable buffer shrink/growth and fixed subarray behavior. Unpaired
UTF-16 strings, SharedArrayBuffer-backed arguments and invalid Date arguments
explicitly reject; complete serialization/error-stack edge cases remain NODE-018
and NODE-003 work. Handles and callbacks are not accepted as evaluation arguments.

### Locator queries and reads

`Page.locator` and main `Frame.locator` return lazy Rust-owned queries.
Supported composition includes `first`, `last`, `nth`, `and`, `or`, nested
`locator`, and string/has/hasNot/visible filters. `locator.page()` preserves
Page identity. Ordinary CSS pierces open shadow roots; every operation resolves
the current DOM again. This is not the full Playwright selector language:
engine prefixes, XPath, extended selector syntax and RegExp filters are not
supported yet.

Page, main Frame and Locator also expose `getByText`, `getByLabel`,
`getByRole`, `getByPlaceholder`, `getByAltText`, `getByTitle` and
`getByTestId`, with string or RegExp matchers. Exact text options and pinned
Playwright 1.58.2 role-name/state/level/hidden options are forwarded to existing
Rust selector values. Test-id uses data-testid; mutable selector registration
is not exposed. Unpaired UTF-16 explicitly rejects. Recipe limits below include
regex flags and the implicit test-id attribute, before native copying.
The expanded 32-case Locator lane passes on `edd8ba96...b0c9b7`, including
label associations, relative/Shadow DOM lookup, role-name/title priority and
global role-name regex. This is not complete selector or accessible-name parity.

Reads include `count`, `allTextContents`, `allInnerTexts`, `textContent`,
`innerText`, `innerHTML`, `inputValue`, `getAttribute`, `isVisible`, `isHidden`,
`isEnabled`, `isDisabled`, `isEditable` and `isChecked`. Scalar reads are strict;
count/collections are not. Missing attributes return null. For example:

```js
const enabledSave = page.locator('button').filter({ hasText: 'Save', visible: true });
console.log(await enabledSave.isEnabled());
console.log(await page.locator('li').allTextContents());
```

Query values and command-owned copies have their own environment-wide limit:
1,024 entries / 8 MiB, with depth 32, 256 nodes and 64 KiB aggregate strings per
query. `browser.pwrightLocatorMetrics()` exposes `{recipes, bytes}`. Retained
pure values may keep this counter nonzero after browser close without keeping
Rust browser I/O alive. Each read also reserves 6 MiB in the existing FFI
settlement budget. These declarations are not V8 heap or RSS measurements.
Page close/Worker termination cancel reads; count/byte pressure and finalization
are tested independently. The Node profile opts into eight target permits and
eight DOM reads per Frame, within the unchanged eight connection-active /
sixteen pending-call and shared byte limits. Default Rust consumers stay
serial. Independent title/Locator/evaluation work can progress while a Locator
waits (NODE-020); arbitrary navigation/input ordering remains unverified.

### Locator actions

`click`, `dblclick`, `fill`, `clear`, `press`, `pressSequentially`, `type`, `check`, `uncheck`,
`setChecked`, `selectOption`, `hover`, `focus`, `blur` and
`scrollIntoViewIfNeeded` invoke existing Rust services. Options include timeout,
force/trial where applicable, pointer position/modifiers, click button/count
and delay, and Press delay/navigation waiting. Sequential typing supports delay
and timeout; its accepted `noWaitAfter` boolean has no effect. Double-click, hover, checked
state and selection do not wait for navigation, following the pinned reference.
SelectOption accepts one string/descriptor, a homogeneous array of either, or
null for deselection; ElementHandle values are not supported.

Each action retains a 6 MiB FFI allowance through settlement. Fill inputs are
bounded at 1 MiB UTF-8, as are sequential text inputs (owned capacity is checked
separately), key expressions at 128 bytes, and selection at 1,024
descriptors / 64 KiB per string / 1 MiB aggregate. Unpaired UTF-16 input
explicitly rejects. Backend script/wire limits are independent additional
bounds. Fill/clear now share the Rust Page input coordinator with pointer and
key actions, including Delete key-up repair and cancellation cleanup.

After the dedicated Chrome/Pwright preflight, set `PWRIGHT_INPUT_FIXTURE` to the
verified serve-lib URL for `tests/fixtures/sequential-typing.html` and run
`pnpm --dir packages/pwright test:typing` with `PWRIGHT_TEST_CDP`. This supervised
lane includes native cancellation/Worker checks; it is distinct from the older
actions lane and the low-level Rust input oracle.

The action gate is not green: the latest semantic Locator addon passes **25/26**,
with the timed-out Shift-state discrepancy (NODE-022) still failing. Pinned
Playwright can retain Shift after this timeout; Rust releases it under its
existing cleanup contract. Original post-Enter reads and added consecutive
document-replacement comparisons now pass (NODE-021's entry precondition).
Do not treat ordinary action matches as full action/error/keyboard parity.

The Node profile explicitly selects bounded Rust execution-world readiness for
DOM reads, evaluation and ordinary Locator operations. It waits under existing
command admission/deadlines/cancellation; no JS sleeps, input replay, extra
driver or browser launch is added. Eight waiters and 64 Frame ownership pairs
declare an additional 20,736-byte backend-state ceiling and at most nine leases,
not a measured RSS bound. Default Rust profiles remain non-waiting. This does
not make later document replacement during backend dispatch atomic.

On the previous world-readiness addon `1c8ee9ea...d9df65`, Locator passes 22/22 and evaluation 38/38.
Lifecycle completes **42/44**, retaining a native-window initialization
discrepancy and a post-disconnect observer navigation interruption. Shared
results are 36 matches / 19 unsupported / 2 oracle errors. The ordinary failing
gates remain visible in the [artifact record](../../docs/node-ffi-compatibility-design.md#world-readiness-artifact-evidence-2026-09-07).

Context options beyond the implemented `Browser.newPage({ baseURL })` slice,
`Page.isClosed()`, before-unload close, child frames, advanced screenshot options, browser launch,
and general events (including Page/Context close events) are not exposed
yet. Declared unsupported async methods reject with
`errors.UnsupportedOperationError`; unsupported sync methods throw it. Unknown
options are rejected instead of ignored. This is not a complete replacement
for Playwright or Playwright Test.

See [the design and discrepancy register](../../docs/node-ffi-compatibility-design.md)
for bounded-resource settings, verification scope, and pending gates. In
particular, the initial host still uses an aggregate Rust composition; smaller
feature-specific native packages and RSS comparison are future work.

The current async exports also have an environment-wide FFI budget: 16 connect,
32 command, 16 close and 16 session-signal settlements; 64 MiB of reserved
native payload/future bytes. Slots remain charged after Rust finishes until JS
conversion, drop, or environment shutdown reclaims the outcome, including
errors. Close has independent admission. Oversized UTF-8 inputs are rejected
before native copying or browser work. Exhaustion rejects with
`PWRIGHT_RESOURCE_EXHAUSTED`; it never silently drops a result.

`browser.pwrightFfiMetrics()` reports `pending`, `queued`, `queuedCommands`, `running`,
`reservedBytes`, `connects`, `commands`, `closes` and `watches`. `queuedCommands`
excludes metadata/disconnect notifications; `queued` counts all lanes. These counters
cover the shared native environment, not just that Browser. They reach zero
after all sessions close and their callbacks settle; other active sessions can
keep them nonzero. The blocked-JS and forced-Worker cases test undelivered
result reclamation. This does not measure user-owned V8 objects, all historic
metadata, allocator overhead or process RSS. New events and binary APIs still
need explicit budgets before being exposed.
