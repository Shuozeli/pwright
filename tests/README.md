<!-- agent-updated: 2026-09-09T08:50:00Z -->

# Pwright comparison tests

NODE-075 extends `pwright-node-host --features frame-navigation`'s ignored
`live_frame_lifecycle` gate with typed Rust host response projection and a
result retained past shutdown. HTTP/hash/404/OOPIF and cancellation checks pass
with complete final lease release. It uses dedicated Chrome and serve-lib;
no Node addon binary or public differential corpus is changed by this step.
[Evidence](../docs/knowledge/pwright-frame-navigation.md#node-075-typed-rust-host-result-ownership).

NODE-074's navigation 1.2 typed contract is verified in Rust tests under
`pwright-codec-navigation/tests/{frame_schema,remote_frame_goto}.rs`, with
generator/client/server checks and explicit CI feature selection. The stream
test uses a deterministic navigation service, not Chrome. This adds no public
Node comparison case; host-to-contract/FFI integration is still required.
[Contract gate](../docs/knowledge/pwright-frame-navigation.md#node-074-opt-in-typed-navigation-12-contract).

NODE-073's selected `live_frame_lifecycle` Rust host test now executes native
Frame navigation itself, including an OOPIF child and HTTP 404. Its dynamic
chunked-response fixture bounds cancellation/timeout/shutdown checks and aborts
the listener task on drop. Static fixtures continue through serve-lib. This is
not a public Node Frame.goto differential gate.
[Contract and evidence](../docs/knowledge/pwright-frame-navigation.md#node-073-rust-host-dispatch-scheduling-and-cancellation).

NODE-072 extends the actual Rust host `live_frame_lifecycle` integration test
when `pwright-node-host/frame-navigation` is selected. It registers before native
dispatch, obtains HTTP/hash results solely through the production host cursor,
and verifies pending-registration shutdown. The test supplies native dispatch
and deadline scheduling; this is not a public Node comparison.
[Gate details](../docs/knowledge/pwright-frame-navigation.md#node-072-ordered-operation-registration-and-actual-rust-host-observation).

NODE-071 adds `src/frame-navigation-wait-oracle.mjs` and the selected Rust
`frame_navigation_trace` example. It compares 29 traces / 131 observations
against installed Playwright 1.58.2's real `Frame.gotoImpl` and `Request`
response/redirect promises using controlled native/event ports. This offline
internal gate does not import Playwright into the runtime package or establish
public Node/browser compatibility.
[Commands and bounds](../docs/knowledge/pwright-frame-navigation.md#node-071-bounded-navigation-matching-and-response-retention).

NODE-065 compares normalized lifecycle traces against the installed pinned
FrameManager, with fake Page/time ports and a real prebuilt Rust reducer bridge.
Eleven traces and 93 snapshots pass. This offline internal gate does not use a
browser or establish public navigation parity; native sequencing remains open.
[Reproduction and boundaries](../docs/knowledge/pwright-frame-navigation.md#node-065-optional-normalized-lifecycle-reducer).

NODE-064 `frame-navigation.html?mode=local|sandbox` supports the version-pinned
Frame initiation gate. Sandbox must be verified as an owned OOPIF, not assumed
from HTML attributes. Six observations compare document/hash URL, referrer,
identity and root/parent/sibling isolation; native completion is followed by a
separate diagnostic load barrier. This is not yet a public Node FFI test.
[Commands, bounds and evidence](../docs/knowledge/pwright-frame-navigation.md#verification-and-reproduction).

NODE-063 `pnpm test:frame-lookup` compares the same public workload against
Playwright 1.58.2 and the selected addon: 41/41 plus three native boundary groups.
The two frame-lookup fixtures cover imported/new nested topology and duplicate
names; callback/RegExp/glob/close behavior is independently observed. The child
has a 180-second deadline and bounded output/report sizes. Use the verified
serve-lib fixture directory and unused absolute report path, not ad-hoc servers.
Typing regressions pass 38/38 on the same addon; use sequential-typing.html,
not input.html (the latter lacks the workload's resetTyping helper).
[Configuration, artifact and scope](../docs/knowledge/pwright-frame-lookup.md#node-063-evidence-and-remaining-work).

NODE-062 supervised test:context-base now passes 16 public observations and four
native boundary groups, including nonvacuous concurrent wrapper identity,
invalid-option rejection, retained-state pressure, foreign URL callbacks and GC.
The exact selected addon also passes Page 7/7 and glob 1647/1647 plus seven checks.
Earlier connection failures remain recorded; no production timeout was increased.
[Reproduction, artifact and scope](../docs/knowledge/pwright-context-creation.md#verification-status-and-remaining-gates).

NODE-061 adds an asynchronous wrapper for the existing bounded Rust attach_probe
binary. It records target-correlated command timing under three layouts/two rounds;
protocol-error diagnostics no longer inflate response counts, and cleanup is
excluded from the connect snapshot. Original-layout glob 1647/1647 and Page 7/7
pass on the unchanged addon, while historical attachment failures remain open.
[Setup and diagnostic boundaries](../docs/knowledge/pwright-chromium-topology-events.md#node-061-reuse-the-bounded-attachment-collector).

NODE-060 shares glob-base-cases.mjs between Rust-parser and actual N-API checks.
Run pnpm test:glob-ffi from the Node package with its endpoint/report settings.
The late-observer experiment passes 1647 patterns/seven boundary checks, not the
still-failing original attachment/Page layout. Expanded probe:attachment covers
observer browser-session creation; reports distinguish each layout explicitly.
[Setup, resource scope and evidence](../docs/knowledge/pwright-frame-lookup.md#artifact-specific-evidence-and-open-connection-regression).

NODE-059 adds `pnpm test:glob-base` from the Node package: 1647 actual Rust
patterns versus the pinned URL-glob helper, with optional unused absolute
`PWRIGHT_GLOB_BASE_REPORT`. This is bounded offline policy verification using
a diagnostic URL parser, not public lookup, browser, or parser resource parity.
[Commands, limits and evidence](../docs/knowledge/pwright-frame-lookup.md#node-059-evidence).

NODE-058 adds `pnpm test:glob-plan` from the Node package, with optional unused
absolute `PWRIGHT_GLOB_PLAN_REPORT`. It compares 2810 actual Rust pattern outputs
to the pinned compiler with bounded input, output and subprocess time. The
offline result is not candidate Node lookup, regex execution or baseURL parity.
[Kernel gates, storage proof and evidence](../docs/knowledge/pwright-frame-lookup.md#node-058-rust-traversal-and-bounded-resolved-glob-compiler).

NODE-057 adds `pnpm test:lookup-oracle` (14 pure pinned-client characterization
tests, not candidate parity). The [lookup design](../docs/knowledge/pwright-frame-lookup.md)
records version differences and full implementation gates.
`pnpm test:mouse` optionally accepts `PWRIGHT_MOUSE_SETUP=background` to keep an
owned blank Page in front while running unchanged fixtures; `original` remains
the default. Both modes currently pass 44/44 on the unchanged addon, without
explaining the prior failures. `node scripts/probe-mouse-focus.mjs` separately
compares foreground/background idle and down/up traces with a 30-second bound,
partial failure reports and exact owned cleanup.
[Diagnostic environment and evidence](../docs/knowledge/pwright-raw-mouse.md#node-057-setup-experiment-not-a-causal-fix).

NODE-056 extends `pnpm test:frames` to 18 shared local/OOPIF cases. The same
fixture and bounded supervisor now cover cached names, window/attribute mutation,
same/full navigation and retained detach/close metadata. Repeated reads preserve
ledger usage; finalization additionally asserts zero `nameBytes`.
`node scripts/probe-idle-mouse.mjs` is a separate 30-second diagnostic from the
Node package directory. It observes two two-second no-input windows per provider
using the raw Mouse fixture and `PWRIGHT_IDLE_MOUSE_REPORT`. It is not a parity
pass: full Mouse comparisons currently retain unexplained extra movement events.
[Commands, artifact and evidence](../docs/knowledge/pwright-frame-publication.md#node-056-bounded-navigation-names-and-public-framename).

NODE-055 adds `pnpm test:pages` from `packages/pwright`: 7 shared public cases,
4000 cached enumeration loops, uncached popup discovery, 20 externally closed
retained Pages and native finalization. Supply the selected profile, dedicated
loopback endpoint and absolute `PWRIGHT_PAGE_ENUMERATION_REPORT`; no static
fixture is needed. The bounded supervisor enables GC. Original blocked-popup
failures are preserved before the explicit Rust activation-policy fix.
[Commands and evidence](../docs/knowledge/pwright-frame-publication.md#node-055-admitted-page-descriptors-and-cache-aware-enumeration).

NODE-054 adds `pnpm test:page-binding`: retain real detached public Frames,
reject new-Page creation at the state limit, then compare exact owned targets
and ledger usage. It reproduces the prior extra-target leak and passes after
Rust pre-admission. `pnpm probe:attachment` records two rounds of five bounded
subject/observer layouts with phase timing; its failures are diagnostic data,
not assertions of conformance. Both commands run from `packages/pwright`.
[Environment, results and remaining timeout investigation](../docs/knowledge/pwright-frame-publication.md#node-054-creation-time-root-binding-admission).

NODE-053 adds the public `pnpm test:frames` comparison from `packages/pwright`.
Set `PWRIGHT_NATIVE_PROFILE=drag-runtime`, `PWRIGHT_TEST_CDP` to the dedicated
loopback endpoint, `PWRIGHT_DRAG_FIXTURE` to the serve-lib-registered full
MagicDNS `raw-drag.html` URL and `PWRIGHT_FRAME_TREE_REPORT` to an unused absolute
JSON path. The supervisor enables GC and fails on native panic or timeout.
Ten local/OOPIF identity/lifetime observations match the pinned oracle; repeated
enumeration charges, exact owned native cleanup and final wrapper release are
also checked. Settled metadata polling does not assert Frame event/timing parity.
[Evidence and limitations](../docs/knowledge/pwright-frame-publication.md#node-053-public-node-frame-collections-and-retained-wrappers).

NODE-052 adds a real Rust host Frame-projection gate and the public diagnostic
`packages/pwright/scripts/probe-owned-oopif.mjs`. The unchanged selected addon
fails owned-Page sandbox navigation; the rebuilt artifact succeeds and passes
44 Mouse, 24 Keyboard and 10 drag/Locator comparisons. Rust Frame identity
observations are still distinct from a public JS Frame-wrapper conformance gate.
[Evidence and commands](../docs/knowledge/pwright-frame-publication.md#node-052-live-host-projection-and-paused-oopif-attachment).

NODE-051 adds `packages/pwright/scripts/probe-frame-identity.mjs`: a pinned
upstream-only local/sandbox Frame identity probe. It registers `framedetached`
before removing the iframe and separates immediate versus published snapshots.
The Rust metadata kernel has 27 new tests; public candidate Frame parity is
still pending. [Evidence and scope](../docs/knowledge/pwright-frame-publication.md).

NODE-050 expands the shared Frame catalog to 18 with pre-focus detach and
Page/Context closure. The selected Node addon independently passes 44 Mouse,
24 Keyboard and 10 drag/Locator comparisons. Reports record the chosen profile
and its actual binary hash, not the unchanged historical default artifact.
[Selection, evidence and remaining work](../docs/knowledge/pwright-chromium-drag.md#node-050-native-lifetime-gates-and-selected-node-addon).

NODE-049 expands the shared Frame catalog to twelve cases. Real local/OOPIF
post-focus removal matches both timeout and successful finite-delay completion.
The Rust harness records phase labels and propagates action failure immediately;
all owned cleanup and aggregate discrepancy checks remain mandatory.
[Contract, evidence and remaining gates](../docs/knowledge/pwright-chromium-drag.md#node-049-optional-frame-to-page-lifetime-handoff).

NODE-048 adds `raw-drag-frame-cases.json`, `raw-drag-frame.html` and
`probe-drag-frames.mjs`. The Rust host's explicit native test reads that same
catalog and proves actual local/OOPIF boundaries. All ten cases run; local and
OOPIF post-focus detach fail, so the gate exits nonzero at 8/10. Earlier eight-case
success must not be reported as the expanded gate passing.
[Results and commands](../docs/knowledge/pwright-chromium-drag.md#node-048-native-frame-gate-exposes-a-lifetime-discrepancy).

NODE-047 expands `raw-drag-keyboard-cases.json` to ten scenarios, adding Locator
Escape/chord, sequential text, and a 60-second press interrupted by a 2-second
timeout followed by repeated down/up. `probe-drag-keyboard.mjs` and the actual
Rust host read the same catalog and compare before close. These are host tests,
not public Node drag conformance. [Evidence](../docs/knowledge/pwright-chromium-drag.md#node-047-resources-and-evidence).

NODE-046 adds `raw-drag-keyboard-cases.json` and `probe-drag-keyboard.mjs` for
five selected-host comparisons: idle Escape, active-drag press/repeat/chord,
and keyup-only behavior. These are native host tests, not a public Node drag
conformance claim. [Evidence](../docs/knowledge/pwright-chromium-drag.md#node-046-verification).

NODE-045 extends raw Mouse to 25 cases (count 15/16/32 boundaries). Actual
HostSession passes 25/25; the rebuilt default Node comparison is 39/44 with the
same five same-turn discrepancies retained. The original 22-case results below
are historical. [Current evidence](../docs/knowledge/pwright-chromium-drag.md#node-045-verification-and-actual-node-artifact).

The separate raw Mouse catalog now matches **22/22** pinned DOM event/state
cases, including click/dblclick and delayed/count variants. Rust click retirement
is tested separately from event snapshots. This is not Node Mouse coverage or a shared corpus
reclassification. The existing `touchscreen` suite calls `mouseClick` and its
fixture accepts click; as the report limitations already state, it does not
prove touch events or the hasTouch contract. See
[Mouse verification and integration gates](../docs/knowledge/pwright-raw-mouse.md).

Full package regression on addon `1319fa02...ded25dc1` now covers every typing
catalog entry, including held modifiers: **38/38 typing**, **27/27 actions**,
**12/12 screenshot**, **45/45 lifecycle**, **16/18 reload**. This is separate
from the shared 57-case corpus. Evidence: `.work/node031-regression.iGmeff/`.

Latest public Keyboard addon `1319fa02...ded25dc1` passes **24/24** paired API
cases via `packages/pwright/scripts/compare-keyboard.mjs`. The shared adapter
now delegates Keyboard publicly: all **57** original cases produce **52 match /
3 unsupported / 2 oracle-error**, zero other errors. Four Keyboard cases move
from unsupported to match. The supervisor correctly exits 1 for remaining gaps.
Evidence: `.work/node031-keyboard-routing.oTrp98/shared/report.json`.
Earlier primitive-only statements below describe prior checkpoints.

This directory contains two different kinds of evidence: historical v1 gRPC/CLI
golden snapshots and the experimental Rust v2 Node-FFI paired API runner.
The old “38 tests / 100% golden match” statement was a historical gRPC result,
not a current Node compatibility result.

`data/raw-keyboard-cases.json` drives the separate pinned raw Keyboard oracle
and Rust `live_keyboard` primitive comparison. The separate typed native backend
target (`-p pwright-backend-cdp-keyboard --test live_keyboard`) also passes
**14/14** down/up/press event/state cases, checking owned native cleanup and zero
operation leases per case. Its insert-text/type services have unit coverage here,
not new live parity evidence. Neither target exposes Node Keyboard or changes
shared classifications. The optional `pwright-composition-chromium-keyboard`
live target now also matches **14/14** through production encoding, generated
server dispatch, Page route and reply decoding. It checks route/operation/frame
lease release, but does not install aggregate host lifecycle routing or Node FFI.
[Commands and exact scope](../docs/knowledge/pwright-raw-keyboard.md).

The separate sequential-input primitive diagnostic uses
`fixtures/sequential-typing.html` and `data/sequential-typing-cases.json`. It matches
13 pinned-oracle event/value cases in a Rust live test, not through the Node
addon. The existing shared suites/adapters and their classifications are
unchanged in that primitive checkpoint. The subsequent Node binding updates only
the shared `type` adapter to invoke public Locator.type; expected values and
classification rules remain unchanged. [Reproduction and scope](../docs/knowledge/pwright-sequential-typing.md).

## Shared Node-FFI gate

Current reload artifact `8d973a18...e53930b` completes all **57 cases** with
**48 match / 7 unsupported / 2 oracle-error**, no other errors, in 57.781 seconds.
The adapter now calls public `page.reload({ timeout: 5000 })`; both original
reload assertions are unchanged. Supervisor exits 1 for incomplete coverage.
Its independent reload gate is **16/18**: arrival-boundary Page close fails,
while a separate post-commit close passes. Complete lifecycle is **44/45** on
NODE-014; screenshots are **0/12** at attachment. Shared success does not close
those gaps. [Evidence](../docs/knowledge/pwright-page-reload.md).

Previous interleaved-input artifact `5a746ac8...3ba900` completes all 57 shared
cases with **46 match / 9 unsupported / 2 oracle-error**, no other errors, in
61.122 seconds. Its separate public gates pass **30/30 typing + 27/27 actions**,
including prior overlap and NODE-022 failures. The shared supervisor still exits
1 for incomplete coverage. These gates do not establish all native input
cancellation/Frame semantics. Older artifact results below remain historical.

The same artifact's latest lifecycle rerun passes **45/45 in 192.715 seconds**
after separating its aggregate test budget from an independent per-case bound;
original API deadlines and assertions remain unchanged. The serial screenshot
rerun passes **12/12 in 19.324 seconds**. Both supervisors exit 0. Preserve the
earlier incomplete lifecycle run (18 pass / 3 fail) and screenshot attachment
failure/recheck pair: one complete green rerun does not resolve their intermittent
causes or change the shared 46/9/2 classification.
[Test supervision evidence](../docs/node-ffi-compatibility-design.md#node-013-lifecycle-supervision-evidence-2026-09-07).

Initial typing artifact `4a098d75...710dcd` completes all 57 cases with **46 match /
9 unsupported / 2 oracle-error**, zero value/candidate/setup/cleanup errors.
The supervisor exits 1 because overall coverage remains incomplete. Preserve
the earlier JSON fixture MIME preflight failure: metadata moved out of the
HTML-only served fixture directory, with no relaxed verifier. The initial
public typing lane passes 27/27; expanded final-artifact concurrent-input
verification fails 27/28, with exact Page cleanup and no cancelled tests.

Preceding recursive-attachment artifact `31333abc...45cdd9` completes all 57 cases
with **45 match / 10 unsupported / 2 oracle-error**, zero value/candidate/setup/
cleanup errors, and both screenshot cases matching (59.191 s). The supervisor
correctly remains non-passing because unsupported/oracle-error cases are not
success; no signal, timeout, overflow or native failure occurs. Its separate
screenshot and lifecycle gates pass 12/12 and 45/45. No shared fixture, case,
expected value or classification changed. [Current artifact evidence](../docs/node-ffi-compatibility-design.md#node-013-recursive-attachment-prerequisite-join-2026-09-07).

The screenshot adapter now invokes public Rust-backed `page.screenshot()`.
No original PNG fixture/header observation changed. The handoff-stage artifact
`e0dcbd2b...e747b92` completes all 57 cases with **45 matches / 10 unsupported /
2 oracle errors**, including both screenshot matches. Final scoped artifact
`acabe149...b698ef5` completes all 57 with **25 matches / 9 unsupported /
2 oracle errors / 21 setup errors**; both screenshot cases match after initial
oracle/candidate attachment failures. Neither report has value/candidate/cleanup
errors or a supervisor timeout/signal/native diagnostic. Both remain non-passing;
the smaller unsupported count in the final run is caused by earlier setup
failures, not extra API coverage. The separate screenshot lane is 10/10 on the
handoff artifact and 8/10 on final-artifact attachment failures. See
[current evidence](../docs/node-ffi-compatibility-design.md#node-page-screenshot-binding-and-ownership-checkpoint-2026-09-07).

Historical preceding checkpoint:

The candidate now implements the existing getByLabel observation using public
Locator.inputValue followed by Locator.evaluate, matching the oracle's original
tag-name assertion and catch-to-null behavior. No fixture/golden is rewritten.
Cleanup also asserts the new Node Locator conversion registry is empty. A
separate public Locator evaluation lane now passes 23 assertions and its native
supervisor after host session-retirement integration (NODE-027), including
borrowed Page/Promise survival. The final shared run still reports 43 matches,
12 unsupported and 2 oracle errors across all 57 cases, with no candidate/value/
setup/cleanup errors. Overall compatibility remains non-green. See
[the current checkpoint](../docs/node-ffi-compatibility-design.md#node-027-host-integration-and-worker-verification-2026-09-07).

Historical complete run (`.work/node-locator-ffi.DTyrg0/shared-registered/`, addon
`2107bdcd...4b0c66`) has **43 matches / 12 unsupported / 2 oracle errors**, all
57 cases completed with zero value/candidate/setup/cleanup errors. The two label
cases now match. Supervisor status 1 has no signal, timeout, overflow or native
failure. Its preceding `shared/` attempt stopped at fixture preflight because
the serve-lib route was absent; both records are retained. This corpus does not
cover or override the separate running-Worker cleanup failure.

Previous complete run (`.work/node-native-retirement.bA2K7k/shared-epoch/`, addon
`930469ad...a7a4a3`) has **41 matches / 14 unsupported / 2 oracle errors**
in 106.414 s (`07:27:36.194Z`–`07:29:22.608Z`).
All 57 cases ran without skips or retries, with zero value/candidate/setup/
cleanup errors. Supervisor exits 1 without timeout, signal, overflow or native
failure. Earlier event-admission and controlled-dispatch failures remain preserved;
this later run does not prove attachment issues fixed. On this same addon,
package Locator is 29/32, evaluation 17/38, lifecycle is watchdog-interrupted
after 11 passes / 5 failures, and actions are 25/26 (NODE-022). The safe
host continuation was not exposed as public Node Locator.evaluate: its separate
native Context-close failure NODE-026 is now fixed in a later receipt-enabled
rebuild, while FFI conversion was then pending. The shared label cases were then
unsupported. [Historical checkpoint](../docs/node-ffi-compatibility-design.md#native-destruction-receipt-checkpoint-2026-09-07).

`pnpm test:node-ffi` runs all 19 existing API suites / 57 cases, using exactly the
same `TestCase.fn` on Playwright 1.58.2 and `@shuozeli/pwright`. Each side owns a
fresh Context and Page. The candidate adapter calls only the public Node package;
it contains no CDP transport, Playwright fallback, browser launcher or JS
implementation of missing automation capabilities.

The catalog is `src/shared-suites.ts`, also used by the legacy API golden runners.
The gate rejects missing/duplicate suites or cases. There is no filter, golden
rewrite, skipped-case shortcut, or automatic retry.

Do not overlap this supervisor with the package's real-browser lanes on the
same CDP endpoint. Fresh owned Contexts do not isolate browser-wide attachment
discovery from another runner closing its targets. The paired-world checkpoint
retains a failed overlapping experiment separately from its serial rerun;
serialization does not resolve the candidate's attachment-churn behavior.

### Setup on this Mac

From the repository root:

```sh
pnpm --dir packages/pwright install --frozen-lockfile
pnpm --dir packages/pwright build
pnpm --dir tests install --frozen-lockfile --ignore-scripts
chrome-cdp start
chrome-cdp status
```

Register the exact `tests/fixtures` directory with the Shuozeli `serve-lib`
daemon, with an explicit Tailscale bind IP, port, route and lifetime. Do not use
the legacy ad-hoc `createTestServer()` for this lane. Before using the route,
verify `serve-lib list` and `curl -I` against its full MagicDNS URL. The runner
also compares every served fixture's status, MIME type and SHA-256 with disk.

Use a new task working directory for the mandatory initial Pwright open.
For this Mac, while the verified `/pwright-shared` registration is active:

```sh
pwright --cdp http://127.0.0.1:9222 open http://macbookpro-yuacx.tail8f3b66.ts.net:9384/pwright-shared/empty.html
```

Then, from `tests/`, set the fixture URL and an **absolute new output directory**
whose parent already exists:

```sh
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 \
PWRIGHT_SHARED_FIXTURES=http://macbookpro-yuacx.tail8f3b66.ts.net:9384/pwright-shared \
PWRIGHT_SHARED_OUTPUT=/absolute/task-directory/run-1 \
pnpm test:node-ffi
```

Output-directory creation is exclusive: an existing report is never overwritten.
The dedicated Chrome process/profile and unrelated pages are preserved. Close
only the preflight Page via `pwright close` in its original task working directory,
and verify its exact target is absent afterward. Never restart the shared Chrome
process to clean up tests.

### Reading results

| Status | Meaning |
| --- | --- |
| `match` | Both setups/actions/cleanups succeeded, Chrome versions agree, and original observation values are deeply equal |
| `value-mismatch` | Successful values or Chrome versions differ |
| `unsupported` | Candidate encountered a declared missing adapter operation, even if the fixture caught that rejection |
| `oracle-error` | Official Playwright could not produce the observation; not a candidate parity failure or pass |
| `candidate-error` | Candidate failed for a reason other than an explicit unsupported operation |
| `setup-error` | A session could not be prepared, or required evidence is missing |
| `cleanup-error` | A prepared session could not fully clean up; equal data cannot override this |

Classification precedence is cleanup, setup, oracle error, unsupported, candidate
error, then value comparison. Both sides' diagnostics and encountered operations
remain in the report even when only one status is counted. A failed setup whose
cleanup also fails retains both errors in its nested setup diagnostic. Unsupported
counts describe encountered calls, not every downstream API the aborted case
would eventually need.

`metadata.json` identifies versions, git state, native binary hash, fixture/source
hashes and the profile's explicit limits. `cases.jsonl` retains completed pairs if
a later case crashes; `report.json` exists only after the whole corpus finishes.
Values are compared before formatting; `dataInspect` preserves reviewable
undefined/NaN/-0 representations without JSON coercion or golden normalization.
Each side is limited to 256 observed operations and 64 KiB of formatted result
evidence. The corpus is capped at 256 cases. These test-runner bounds are not an
RSS guarantee.

`supervisor.json` separately records native panic/cleanup diagnostics, exit
status, timeout and log overflow. Read it with `report.json`: successful
assertions cannot excuse a native panic. Exit 0 requires the complete matrix to
match; ordinary discrepancies exit 1; supervisor/fatal infrastructure failures
exit 2. The supervisor has a ten-minute deadline and a 2 MiB combined log budget.
Connect and candidate goto use 5-second public deadlines; the oracle's default
action deadline is 5 seconds. Candidate Locator scalar/state reads now use
their public 5-second timeout; title/content retain their native CDP budgets.

### Current checkpoint and corpus limitations

The safe host now installs paired selection/adoption and delayed conversion in
one controlled command. Its separate Chrome tests are 4/5 because native Context
closure cancels the call but fails group cleanup at shutdown. Public Node
Locator.evaluate is not exposed; both label observations remain unsupported.
The preceding `.work/node-element-preparation.YxRNYz/` Rust-only evidence did
not rebuild the addon or rerun the corpus and must not replace current evidence.

Latest topology rebuild
`72644cf9eb2285380af2546833f18796dae7b5ee407864b83ee5cb4471b9ddcd`
has two distinct full reports in `.work/node-frame-worlds.qDq1XE/`:

- `shared/`: overlaps the package lanes, with 34 matches / 10 unsupported /
  2 oracle errors / 11 setup errors. One setup error includes a nested cleanup
  failure; setup failure can prevent reaching an unsupported operation.
- `shared-serial/`: same binary, fixture hashes and runner source hashes,
  **41 matches / 14 unsupported / 2 oracle errors**, completed
  `2026-09-07T04:47:28.166Z` in 52.083 seconds. Value/candidate/setup/cleanup
  counts are zero; supervisor exits 1 without signal, timeout, overflow or
  native failure. This does not replace the first failure or prove its cause.

Serial package gates are Locator 32/32, evaluation 38/38, lifecycle 44/45
(NODE-014) and actions 25/26 (NODE-022). The optional Rust paired-world access
port is not yet connected to Node selection/adoption; public Locator.evaluate
and the two label observations remain unavailable/unsupported. See
[the full artifact record](../docs/node-ffi-compatibility-design.md#production-paired-world-access-checkpoint-2026-09-07).

Previous CDP/Handle rebuild
`42a26fd0b5325c393b930a89f18dc628959c11bfa884e293756fb19c9d734e94`
retains **41 matches / 14 unsupported / 2 oracle errors**, with zero
value/candidate/setup/cleanup errors. `.work/node-element-evaluation.D9og5u/shared/`
completed `2026-09-07T03:58:39.257Z` in 51.743 seconds. Supervisor exit 1 has no
signal, timeout, overflow or native failure. Node Locator.evaluate is still
missing: the Rust strict-wait and adoption/evaluation leaves are prerequisites, not
an implemented public method. The unchanged two label observations therefore
remain unsupported. The same binary passes Locator 32/32 and evaluation 38/38;
lifecycle is 44/45 (NODE-014 outer-window dimensions), actions 25/26 (NODE-022).
See the [full artifact record](../docs/node-ffi-compatibility-design.md#locator-main-world-evaluation-checkpoint-2026-09-07).

Previous semantic Locator addon
`edd8ba969f29904184da9cc025397638cad99bd00860090c6ece267bf2b0c9b7`
completes all 57 unchanged cases: **41 matches / 14 unsupported / 2 oracle
errors**, with zero value/candidate/setup/cleanup errors. Report:
`.work/node-navigation-order.KIOgvf/shared-final/`, completed
`2026-09-07T02:32:42.941Z` in 62.60 seconds on Chrome 152.0.7977.76 / Node
26.7.0 / macOS arm64. Supervisor exit 1 has no signal, timeout, overflow or native
failure. Public getByText/getByRole add five matches. Public getByLabel exists,
but the two legacy label observations still need unsupported Locator.evaluate;
their assertions are not replaced with inputValue or JS fallback.

The same artifact passes Locator 32/32, evaluation 38/38 and lifecycle 45/45.
Actions remain 25/26 (NODE-022). Window-size and cross-connection ordering
failures did not reproduce; that is not a fix. See the
[artifact-specific evidence](../docs/node-ffi-compatibility-design.md#semantic-locator-artifact-evidence-2026-09-07).
The full shared gate still fails; unsupported operations and oracle errors are
not matches. Historical artifact-specific checkpoints follow.

Previous world-readiness addon
`1c8ee9ea2544c1c6e8f03ea5a74b4b29d390eb8cac4ca30b718a260224d9df65`
completes the unchanged 57-case corpus on Chrome 152.0.7977.76 / Node 26.7.0 /
macOS arm64: **36 matches / 19 unsupported / 2 oracle errors**, with zero
value/candidate/setup/cleanup errors. The report is
`.work/node-world-readiness.YcU7iD/shared-final/`, completed
`2026-09-07T01:48:26.443Z` in 52.71 seconds. Its supervisor exits 1 with no
signal, timeout, overflow or native failure. Unsupported and oracle failures
remain non-passing; this is not a full compatibility certificate.

The same artifact's package lanes complete actions 25/26, Locator 22/22,
evaluation 38/38 and lifecycle 42/44. World-readiness cases now pass, but
NODE-022 Shift cleanup, NODE-014 native-window initialization and NODE-024's
post-disconnect observer navigation race remain failing observations.
The first readiness artifact's 44/44 lifecycle pass and the isolated diagnostic
are not replacements for this final failing lifecycle gate. See the
[complete evidence](../docs/node-ffi-compatibility-design.md#world-readiness-artifact-evidence-2026-09-07).

Historically, after the action/terminal-completion fix, the completed full run on Chrome
152.0.7977.76 / Node 26.7.0 / macOS arm64 reports 31 matches, 19 unsupported
cases, 2 oracle errors and 5 attachment setup errors; no value mismatch,
candidate-operation error or cleanup error. Four setup cases time out on both
sides and one only in Rust topology initialization. The release addon SHA-256 is
`e50764079d4aafcb3ba898257c4726b385db2a73194e735028896c3684265d48`;
evidence is `.work/node-actions.5JbLoA/shared-completion-live/` at repo root. The supervisor exits
1 with no signal, timeout, overflow or native failure. This remains a failing
full-compatibility gate. See the [Node discrepancy register](../docs/node-ffi-compatibility-design.md).

The preceding action artifact `eae32ce9...41af9e`, retained in
`.work/node-actions.5JbLoA/shared/`, completed 36/19/2 with no setup errors.
Do not transfer its matches to those five setup failures. A first final
shared invocation stopped before cases because the fixture registration had
expired (HTTP 404); `shared-completion/` preserves that exit-2 report. The route
was renewed and verified before the independently recorded live run.

That earlier action result adds fourteen matches over the read/concurrency 22/33/2
checkpoint (`.work/node-concurrency.TLMFjf/`, addon `9b2415d7...e77452`).
Reads previously added ten matches over the earlier evaluation 12/43/2 baseline
(`.work/node-paired-init.nHmvUx/shared/`, addon `384f05939a4fce614c649e3cf2212daebf294a8ac0de3c4d5d104ea2d3c25a4c`).
The candidate adapter uses public Locator reads and actions; `.first()` preserves
the original oracle adapter's legacy Page-selector first-match semantics. The original
shared filter case still uses evaluation, and shared and/or cases use CSS
selectors, so package-local tests remain necessary evidence for actual public
query composition. No shared spec, fixture or golden was changed.

The preceding concurrency addon passed lifecycle 44/44 (95.17 seconds), evaluation
38/38 (48.80 seconds), and Locator 22/22 (33.92 seconds). The original NODE-020
case now passes through explicitly selected Rust target and DOM-read permits,
within unchanged shared limits. The preceding Locator-stage addon
(`748c976f...e07838`) and its 18/19 failure remain under
`.work/node-locator.51x1V1/`. Earlier setup failures (NODE-013), the first Locator-stage window-size
failure (NODE-014), and pre-fix close-error evidence remain recorded. Passing
subsets do not prove complete APIs, serialization or RSS parity. No skip,
normalization or retry policy changed.

The action-stage package adds `test:actions` as a fifth live lane. Its historical
24-case run passes 22, retaining the post-Enter utility-world readiness failure
(NODE-021) and delayed-press timeout modifier-state discrepancy (NODE-022).
Neither is present in this shared corpus, so 36 shared matches cannot establish
full action parity. Final `e5076407...65d48` passes Locator 22/22, evaluation
38/38 and lifecycle 44/44 after the scoped NODE-023 fence fix; actions remain
22/24. Earlier regression failures and artifact-specific outcomes remain in the
[Node evidence record](../docs/node-ffi-compatibility-design.md).

The subsequent bounded attachment diagnostic reproduces NODE-013 in an
independent Rust process too while a Playwright connection remains open. A
TCP_NODELAY experiment did not demonstrate improvement and was reverted.
Later diagnostic passes are not replacements for these full regression results.
See [the diagnostic guide](../packages/pwright/README.md#attachment-diagnostics);
its scripts never run inside this shared corpus or rewrite its observations.

Both oracle errors occur in history navigation. A separate diagnostic observes
`BackForwardCacheRestore`, the expected restored title and `readyState=complete`,
while Playwright 1.58.2 times out waiting for `load`. This dedicated Chrome
profile does not use Playwright's usual `--disable-back-forward-cache` launch
switch. The gate does not alter that profile, silently change `waitUntil`,
increase timeouts or treat the failed oracle as a match (NODE-015).

Reproduce that diagnostic with the same CDP/fixture variables plus an absolute,
new `PWRIGHT_SHARED_HISTORY_OUTPUT` file, then run `pnpm probe:shared-history`.
It records at most 128 events on a separate owned oracle Page and closes its
observer/Context. It never runs inside the paired gate and is not a conformance
pass.

The old corpus is not an exhaustive API contract. Its download adapter returns
a historical placeholder after the event, not an artifact-content result.
Upload's `/tmp/test-file.txt` alias maps to one new owned fixture file and never
reads an existing machine file. The touchscreen case currently calls
`mouseClick`; some mouse/filter/composition cases also approximate their names
with simpler operations. Label cases observe tag names and screenshots check
headers/size, not pixel parity. Future richer fixtures must be added explicitly,
not retroactively substituted to claim this corpus tested more.

## Commands

| Command | Purpose |
| --- | --- |
| `pnpm test:node-ffi` | Full shared API paired gate; unsupported and oracle failures are non-passing |
| `pnpm test:shared:unit` | Runner classification, resource cleanup, budgets, catalog and adapter-boundary tests |
| `pnpm test:types` | Strict TypeScript check, including the new runner tests |
| `pnpm probe:shared-history` | Separate bounded oracle history/BFCache diagnostic |
| `pnpm test:unit` | Legacy CLI parser/normalizer unit tests |
| `pnpm golden:record [suite]` | Legacy API oracle snapshot writer; not needed by the Node gate |
| `pnpm test [suite]` | Legacy v1 gRPC vs saved golden comparison |
| `pnpm golden:record:cli [suite]` / `pnpm test:cli [suite]` | Legacy CLI snapshot lanes |

The legacy golden lanes still contain their old static HTTP server setup and
are not the prescribed local browser workflow on this Mac. They were not used
for this Node checkpoint. Existing `specs/`, `fixtures/` and `golden/` files
remain unchanged. Add new API suites to `src/shared-suites.ts`; the inventory
check requires every non-`cli-` `*.spec.ts` file to appear exactly once.
