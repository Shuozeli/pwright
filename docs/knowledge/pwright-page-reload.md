<!-- agent-updated: 2026-10-10T20:21:33Z -->

# Page reload: typed Rust command and Node FFI

## Python binding-publication checkpoint (2026-10-10)

The frozen ABI108 reload/click v2 wheel now passes the unchanged pinned
Playwright Python Context/Page init-script case **1/1** in both candidate and
independent oracle lanes. Both schema-5 sidecars verify source-bound; exact
report and wheel hashes are in [the Python conformance log](../python-official-conformance.md#frozen-abi108-reloadclick-refresh-2026-10-10).
This proves the bounded Python callback ordering case, not all Page reload
semantics or the older Node raw-close cases below.

The separate Playwright Python `BrowserContext.expose_function` plus Context
and Page init-script case fails in the installed ABI108 wheel: after
`page.reload()`, its callback list is `['context']` instead of
`['context', 'page']`. A non-official, uncontested CDP probe confirmed both
script markers are present in the new document and a subsequent Page
`evaluate()` observes the delayed `page` callback. The Page script is not
missing; reload returned before current-document binding publication completed.

`crates/pwright-node-host/src/reload.rs` now has an opt-in `page-bindings`
fence after navigation and topology publication. It selects the current main
Frame only when the Page or its Context has a registered binding, awaits that
document's controller `drainPublications()` Promise, and stays within the
original reload deadline and shutdown cancellation. It does **not** use the
global Rust binding watermark, wait on another Page, or turn an absent
controller into a reload failure. Three focused release tests cover
acknowledgement, deadline and cancellation; one checks Page/Context
registration selection. The rebuilt wheel passes the unchanged official case
with verified provenance. It does not resolve the older Node 16/18 raw-close
failures documented below.

NODE-077's public Frame.goto investigation reuses the raw-close evidence below
rather than adding a new stopLoading workaround. Fresh Frame tests reproduce
arrival-close failures but pass after the new document executes. Their timeout
continuation/Worker results do not constitute a new reload run or resolve this
page's historical 16/18 failures.
[Frame-specific evidence](pwright-frame-navigation.md#node-077-causal-close-timeout-continuation-and-worker-teardown).

## Status

The new public Mouse addon `e9d9e543...109535` reruns this unchanged lane at
**16/18** (`.work/node032-mouse-ffi.dx1AXD/reload.log`). The upstream held-response
Page-close observation exceeds 10 s; the separate candidate check again reports
that the exact native target remains open after Target.closeTarget. The post-
commit case still passes. Mouse integration does not fix or waive these failures.

The minimal raw CDP control now reproduces an acknowledged-but-still-present
target **without loading host, navigation service, runtime or Node addon**. It
has no reload timeout/cancellation policy. The same response boundary fails in
two recorded runs. This narrows the investigation to the native close/navigation
boundary, but is not a fix or a complete Chromium root-cause proof. See
[raw isolation](#minimal-raw-cdp-isolation) and the earlier
[host diagnostic](#causal-native-close-diagnostic).

Latest same-artifact rerun on Keyboard addon `1319fa02...ded25dc1` remains
**16/18**, 28.333 seconds (`.work/node031-regression.iGmeff/reload.log`).
The original oracle response-arrival close exceeds its 10-second diagnostic
bound; Rust independently reports TargetStillOpen. The post-commit close case
still passes. No fixture boundary, assertion or timeout was weakened.
Full screenshot **12/12** and lifecycle **45/45** pass on this addon, but the
prior intermittent attachment/window failures are not declared fixed.
Current shared coverage is **52/3/2** after Keyboard integration; counts below
describe the earlier Reload artifact.

NODE-030 now exposes **`page.reload({ waitUntil, timeout })`** through navigation
1.1, the Rust controlled runtime and N-API. JavaScript only validates/forwards
options and wraps Response metadata. The shared corpus improves to **48 match /
7 unsupported / 2 oracle-error**. Ordinary reload, document replacement, history,
POST method/body resubmission and timeout tests pass, but the full reload lane
is **16/18**: Page close at response arrival fails on both sides in different
ways, while the new post-commit close case passes on both. This is not complete
reload lifecycle compatibility.

## Upstream contract

The installed pinned oracle's `lib/server/page.js::reload` starts a new-document
navigation wait alongside the delegate reload. `lib/server/frames.js` explicitly
rejects same-document events as completion for that wait and resolves the final
request's response after the selected lifecycle state. Chromium's
`lib/server/chromium/crPage.js::reload` sends **`Page.reload`**, with no replacement
URL or `ignoreCache` override. This distinction matters for POST resubmission,
cache behavior and history; `goto(page.url())` is not an acceptable implementation.

The semantic operation belongs to Page, not Frame. Its public options are
`waitUntil` and `timeout`; success is response metadata or null. The existing
four wait policies are commit, DOMContentLoaded, load and networkidle. Returning
null for blank/data documents is valid and does not imply failure.

## Native implementation

`crates/pwright-backend-cdp-navigation/src/reload.rs`:

1. Reserve the existing shared navigation-operation admission and byte lease.
2. Read the current main Frame and previous loader identity, validate their
   bounded lengths, and discard the rest of the snapshot before waiting.
3. Subscribe before enabling Page/lifecycle/Network events and issuing reload.
4. Send `Page.reload` exactly once; never synthesize `Page.navigate` or retry it.
5. Ignore foreign-session, child-frame, old-loader and same-document events.
   The first new main-document request identifies the new loader for HTTP;
   a new main-frame commit is the fallback for non-network documents.
6. Feed that identifying event into the existing new-document state machine,
   preserving request/response association, commit and selected lifecycle.

The Goto waiter is factored into an empty-state wrapper and the same seeded-state
implementation. No existing Goto policy is changed. Network failures propagate;
HTTP 404 is a successful navigation response, not a transport failure. Lagged
event streams fail explicitly instead of inferring the missing document.

## Resource and cancellation boundary

This leaf adds no normal dependency, task, channel, timer service, browser
provider, Node driver or process owner. One caller-polled future owns one event
cursor and the existing navigation lease. Its identifiers/URLs are checked
before copies into retained state. The old loader is dropped after selecting
the new document. These fit the existing conservative native-ID/URL and response
budget; there is no new unbounded replay queue. The underlying CDP frame-size
budget still owns deserialization of the Frame-tree response itself.

Dropping the future releases local cursor/admission/byte ownership. **It does
not prove that browser loading has stopped.** This primitive has no autonomous
timeout/cancellation owner. Its typed service is now routed through the
controlled dispatcher; `CommandCancellation<Reload>` invokes Page stop-loading.
The public timeout case passes with native/FFI resource release, but Page-close
settlement remains a failing gate. Networkidle reuses the existing
navigation policy and is not a new guarantee for every descendant Frame/request
interleaving.

## Evidence (2026-09-07)

Evidence directory: `.work/node030-reload.KLxLvo/`.

- `unit-final.log`: **13/13** navigation-leaf tests, including seven added reload
  tests. They cover all four wait policies, response metadata/status, old/foreign/
  child/same-document filtering, pending load after commit, network failure,
  native-ID overflow, lagged stream and shared admission/byte release after drop.
- `regression.log`: umbrella CDP backend **18/18**, navigation leaf **12/12**
  before the additional lag test, standalone navigation composition **2/2**.
- `clippy-final.log`: release all-target Clippy passes without suppression.
- `dependencies.log`: standalone navigation proof remains **15 Pwright packages**.
- `oracle.json` / `oracle.log`: fresh Playwright 1.58.2 observations on dedicated
  Chrome 152, three document kinds times four wait policies. The oracle writes
  only after owned Context cleanup and refuses to overwrite an existing record.
- `live.log`: **12/12 observations match** in one Rust integration test (3.08 s).
  It compares final URL, response URL/status/status text or null, and unchanged
  history length. Rust additionally verifies replacement of the previous loader.
  Setup and comparison errors return through cleanup rather than panicking with
  an owned Context still open. Native Context disposal completes.
- `unit-initial.log` retains the initial test compilation error: CDP events are
  immutable, so a foreign-session event must be constructed with that session,
  not mutated after creation. It is not a browser failure.
- `targets-final.log`: no remaining Pages after closing this task's preflight.
  The existing Chrome process was not restarted and no unrelated Page was closed.

Reproduce after the dedicated Chrome/Pwright preflight and verified serve-lib
registration. Use fresh absolute paths for the oracle JSON:

```sh
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 \
PWRIGHT_RELOAD_FIXTURE=http://macbookpro-yuacx.tail8f3b66.ts.net:9384/pwright-input-policy/index.html \
PWRIGHT_RELOAD_ORACLE=/absolute/task-directory/oracle.json \
node packages/pwright/scripts/probe-reload.mjs

PWRIGHT_CDP_WS_URL='<verified loopback browser WebSocket>' \
PWRIGHT_RELOAD_ORACLE=/absolute/task-directory/oracle.json \
CARGO_INCREMENTAL=0 cargo test --release --locked \
  -p pwright-backend-cdp-navigation --test live_reload -- --ignored --nocapture
```

The served route is a temporary registration and must be verified again before
reuse. Run oracle and native browser tests serially against the shared endpoint.

## Typed command / FFI checkpoint (2026-09-07)

Navigation 1.1 adds `navigation.reload`, method **65539**, command/result tags
**5/6**, and `ReloadParams { wait_until }`. It reuses `GotoResult` and navigation
error details without changing old fields or tags. Capability, binary schema,
server chain and reference are generated from the canonical source and checked
against navigation/common 1.0 baselines. Type declarations retain canonical ID
ordering; compatibility checks still verify the original models exactly.

The optional client exports `PageReloadExt`. Standalone and umbrella compositions
route Reload explicitly; generated server chains now take three command services.
Target validation and controlled cancellation remain in Rust. The host uses the
same topology/URL publication fence as goto. N-API uses the existing navigation
byte allowance and command-admission lane. JavaScript forwards parameters and
wraps Response; it implements no navigation, CDP or operation timer.

New artifact SHA-256:
`8d973a18bf26623288efb0798f0b792a4345b206f7e6db8b9f3d2b868e53930b`.
Evidence directory: `.work/node030-reload-binding.IV5cBY/`.

- `generation-check.log`: generated output is fresh and 1.0-compatible.
- `final-typed-tests.log`: backend **14/14**, client **6/6**, codec **22/22**,
  server **1/1**. Reload keeps command/deadline/cancellation identity; success and
  errors roundtrip, and Goto rejects a forged Reload payload. Existing Goto golden
  bytes still pass. Typed cancellation sends Page.stopLoading; the client works
  through an in-process dispatcher without a wire codec.
- `regression.log`: portable capability **2/2**, Chromium composition **287/287**
  (15 live tests ignored), standalone navigation **2/2**, host **89/89** (5 live
  tests ignored), plus the preceding typed checks.
- Release Clippy passes (including all addon targets and final client/backend
  tests), Node unit **29/29**, package/shared TypeScript and the **224-package**
  safe-host proof pass. Navigation-only remains **15 Pwright packages**.
- `live.log`: initial 16-case lane has 14 passing completed cases and two
  failures, then its 180-second supervisor kills it. POST used unavailable
  Locator.waitFor; the revised fixture uses supported auto-waiting textContent
  and additionally checks the exact resubmitted form body.
- `live-traced.log`: POST passes, but Page close stalls in the **oracle half**
  after response arrival, before running the candidate. Node's 25-second test
  timeout leaves the action alive and the aggregate watchdog later kills it.
- `live-bounded.log`: **15/16**, 27.732 seconds, exit 1 without signal. A separate
  10-second diagnostic bound now lets Page-close failure reach fixture cleanup;
  the reload API's 1-second timeout is unchanged.
- `live-native-close.log`: **15/17**, 29.782 seconds, exit 1 without signal. The
  original oracle failure remains. An independent candidate case also fails:
  Chrome acknowledges Target.closeTarget but the target remains; Rust's bounded
  absence check reports Backend / TargetStillOpen. Context cleanup then runs.
  These are two retained red gates, not a successful close comparison.
- `live-postcommit.log`: **16/18**, **37.482 seconds**, exit 1 without signal,
  cancellations or skips. Both original arrival-boundary failures remain; the
  independent post-commit paired case passes in **1.489 seconds**. Only the
  replacement document contains a script that POSTs a beacon to the fixture;
  close starts after that beacon arrives, while the HTML body remains open.
  A 2048-byte comment precedes the script to avoid short-response buffering.
  Both sides close the Page and reject reload with a closed error; candidate
  Context/Browser cleanup and zero native/FFI ownership checks also pass.
  This new case has an explicit 5-second reload budget for the execution gate;
  the original failing cases retain their 1-second API budgets and assertions.
- `shared/report.json`: **57/57 cases executed**, **48 match / 7 unsupported /
  2 oracle-error**, no value/candidate/setup/cleanup errors, **57.781 seconds**.
  Both original reload cases move from unsupported to match through the public
  adapter, without changed assertions. Supervisor status 1 correctly denotes
  incomplete coverage, with no timeout, signal, overflow or native failure.
- `lifecycle.log`: the same new artifact completes **44/45**, **248.847 seconds**,
  exit 1 without signal or skips. NODE-014 is the sole failure: Rust reports
  window dimensions 0 by 0 versus the oracle's 1282 by 800; viewport/media
  observations match. The old artifact's 45/45 is not proof for this binary.
- `screenshot.log`: **0/12**, **62.496 seconds**, exit 1 without signal. The first
  candidate times out during topology initialization; later oracle connections
  reach WebSocket connection but time out before API attachment finishes.
  No image comparison is reached. This does not establish reload as the cause.
- `targets-after-preflight-close.log`: the original task preflight target
  `4E54FE9BFF10A8CDB353A09AB9CFAC7A` is absent after exact task-owned close.
  Unrelated game Pages remain; Chrome was not restarted. A subsequent boundary
  probe uses a separately recorded preflight and does not alter this evidence.
- `attach-postcommit.log`: one separately compiled release safe-host diagnostic
  attempt completes in **93 ms**, **63 commands / 63 responses**, no pending
  replies, error or bounded-trace overflow; host close releases all metrics.
  It preserves the original five-second connect budget. This attempt does not
  reproduce the earlier screenshot attachment failure and is not an addon
  screenshot pass or evidence that NODE-013 is fixed.
- `postcommit-unit-node.log`: **29/29** offline Node tests pass after the fixture
  addition. JavaScript syntax and `git diff --check` pass; no production code or
  addon changes were needed for the boundary test.
- `targets-postcommit-final.log`: boundary-probe preflight target
  `6C49FF165829C54103BBDE48E041EC47` is absent after exact task-owned close.
  All runners have exited; seven unrelated game Pages remain untouched.

`pnpm --dir packages/pwright test:reload` requires the verified CDP/base URL and
explicit Tailscale `PWRIGHT_TEST_BIND`. Its dynamic form and held-response servers
do not expose static directories. Run browser lanes serially.

### Open close-boundary discrepancy

The fixture proves **request/response arrival**, not document commit: it sends
a title fragment and withholds the body end. At that point Playwright waits at
Page.close; Rust reports the still-present target. This does not establish
commit timing, Chromium behavior or cancellation order as the cause. Do not
replace absence proof with command acknowledgement, ignore TargetStillOpen,
raise confirmation counts or release the body merely to turn this test green.

The separate post-commit boundary now passes, using execution of a script in
the replacement document as causal evidence. It changes response padding and
adds a script; it is not a controlled proof that buffering alone causes the
arrival-boundary failure. The original failure is not replaced by this new case.
The diagnostic below now correlates native request/commit and target-presence
observations at the arrival boundary. Current Page close prepares disposal, requests
native close, confirms absence, then commits local retirement. Any cancellation
ordering change must preserve rollback and opt-in composition: Page lifecycle
must not acquire an unconditional navigation dependency.

## Causal native close diagnostic

Evidence: `.work/node030-close-probe.BdnMe4/`. This is a **test-only host
diagnostic**, not another addon build or a paired compatibility pass. The addon
remains `1319fa027f4bd5ee0d3e2f3932dce1c01ad5b8de86381b6817970ee0ded25dc1`.
The original 18-case reload fixture, assertions and budgets are unchanged.

`packages/pwright/scripts/probe-reload-boundary.mjs` owns a dynamic Tailscale
fixture and a bounded release-test child. Its first document ends normally;
the replacement writes only the original title fragment and holds the body open.
Immediately after that write, the server signals the Rust test through the
child's stdin. This marks **server write queueing**, not network delivery or
document commit. The first HTTP-polling version added enough delay to observe
commit before close and was discarded as a model of the original boundary;
its `probe.log` failure remains recorded.

`crates/pwright-node-host/src/reload/boundary_probe.rs` creates a fresh owned
Context/Page for each control and observes the existing host CDP connection:

- Exact native Page session/target filtering; no observer attachment, browser
  restart, global target close, JS driver, or unrelated Page interaction.
- At most 256 projected records; phase/loader/request strings at most 128 bytes.
  Overflow or event lag invalidates the diagnostic. No URLs or bodies are kept.
  Loader IDs distinguish replayed old-document lifecycle events from the new
  document; request IDs separate its response from unrelated favicon traffic.
- One joined observer task, one joined reload task with its original 1000 ms
  API budget, and a 5-second causal-marker bound. The response-only control
  uses one additional pre-registered cursor, capped at 256 searched events and
  5 seconds, matching the owned document request before its response.
- Every control independently checks native target presence after close, after
  reload settlement, and after owned Context cleanup. Final host metrics must
  have zero leases, operations and cleanup jobs. The fixture supervisor caps
  combined output at 1 MiB and execution at 120 seconds; each JSON record is
  capped at 64 KiB. Supervisor failure is not a diagnostic success.

Latest four-control run: `native-response-final.log`, **5.03 seconds**, normal
exit. All integrity/cleanup assertions pass; the outcomes deliberately include
failed closes:

| Control | Close outcome | Reload outcome | Target present after reload |
| --- | --- | --- | --- |
| Original host close at server signal | TargetStillOpen | 1000 ms timeout | Yes |
| Direct native close at server signal | Command acknowledged | 1000 ms timeout | Yes |
| Direct native close at correlated document response | Command acknowledged | 1000 ms timeout | Yes |
| Stop loading, then host close | Stop acknowledged; close succeeds | ERR_ABORTED | No |

The direct controls call browser-scoped `Target.closeTarget` on the exact owned
target, bypassing the host close/disposal path. In the response-correlated case,
the new request/loader `7A54…59A` reaches the response boundary at **10.115 ms**;
close begins at **10.118 ms** and acknowledges at **10.847 ms**. The same loader's
`Page.frameNavigated` is observed at **11.787 ms**. The target remains after
reload timeout and disappears only after Context disposal. These are **host
observation times**, not cross-process browser timestamps or proof of Chromium's
internal scheduling. The native command succeeding is not retirement proof.

Earlier completed runs are retained rather than selecting a favorable outcome:
`direct-signal.log` and `loader-correlated.log` reproduce host TargetStillOpen;
`native-control-{1,2,3}.log` all close their baseline and immediate direct-native
targets successfully. Stop-loading controls sometimes return
`Not attached to an active page`, followed by either successful close or
TargetStillOpen. A successful stop also changes the reload result to
`ERR_ABORTED`. Therefore **unconditional stopLoading is not an established
remedy**, and immediate direct close success does not establish a host bug.

The installed pinned Playwright 1.58.2 source independently confirms that
ordinary Chromium Page close delegates to browser `Target.closeTarget`
(`crPage.js::closePage`, `crBrowser.js::_closePage`); `Page.close` is its distinct
run-before-unload branch. The current data rules out host close/disposal ordering
as a necessary condition for the symptom, but does not isolate a Chromium
regression or prove that no other client setup matters. The next section now
isolates the same symptom without the host or navigation service. Acceptable
settlement and cancellation semantics remain to be specified before changing
production behavior.

Release verification: `host-final.log` **99 pass / 6 ignored** (four new offline
diagnostic tests), strict all-target `clippy-final.log`, formatter and JS syntax
pass; `node-final.log` **31/31**. These are not additional public API coverage.
`targets-final.json` verifies all **19 recorded owned target IDs**, including
the task preflight, are absent. The two unrelated Pages remain untouched and
the same Chrome process is ready; all diagnostic runners have exited. No
additional commit or push was performed.

Run after the dedicated Chrome/Pwright preflight, using the verified Tailscale
IP and full hostname. Static preflight files still use serve-lib; this dynamic
fixture exposes no directory:

```sh
PWRIGHT_TEST_BIND='<verified Tailscale IPv4>' \
PWRIGHT_TEST_HOST='<verified full MagicDNS hostname>' \
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 \
node packages/pwright/scripts/probe-reload-boundary.mjs
```

## Minimal raw CDP isolation

Evidence: `.work/node030-raw-close.z1I70F/`, on the same dedicated Chrome
152.0.7977.76. `crates/pwright-cdp/tests/reload_close_boundary.rs` imports only
the raw CDP client and diagnostic dependencies. Its normal Pwright dependency
closure is exactly **pwright-cdp + pwright-core**; there is no host, composition,
navigation service, controlled runtime, addon, JS automation or auto-attach.
Node serves the dynamic fixture and supervises the Rust child only.

The raw recipe creates its own dispose-on-detach Context, creates/attaches one
Page, enables Page/lifecycle/Network events, and loads the initial document.
It matches that document's loader-specific load event, then sends Page.reload.
After the original server-write signal (or the separately correlated new-document
response), it sends browser Target.closeTarget exactly once. It samples exact
target presence immediately, at 200 ms and at 1000 ms after acknowledgement.
These are **observation windows, not a reload deadline**. No operation timer,
host disposal or navigation cancellation can issue stopLoading in the two direct
controls. The third control explicitly tests stopLoading as an intervention.

`raw-1.log` and `raw-final.log` both finish normally in **3.54 / 3.50 seconds**.
The response-boundary direct close acknowledges yet leaves the target present
at all three samples in **both** runs. The immediate server-signal control
closes in the first run but remains present in the second. StopLoading returns
`Not attached to an active page` in both runs, followed by respectively a
still-present target and successful native removal. All six owned Contexts
dispose successfully and all recorded targets are absent after that cleanup.
No observer overflow or incomplete stream is reported. These are diagnostic
integrity results, not six compatible operations.

In the final response-boundary record, loader/request `D3DD…A8C3` reaches the
response control at **4.894 ms**, close starts at **4.901 ms** and acknowledges
at **5.700 ms**. The same document's frame navigation is observed at **6.346 ms**.
The target remains at the 200 ms and 1000 ms samples. As above, timestamps are
local observations, not Chromium internal clocks. This rules out the host,
navigation service, runtime and Node addon as necessary triggers. It does not
identify a particular Chromium bug or prove identical behavior on other browser
versions, platforms or client configurations.

Diagnostic boundaries: 10-second Page setup and action limits, 5-second connect
and Context-cleanup limits, at most 256 events per search, 256 projected trace
records and 128 bytes per retained identity/phase. Once the observer starts,
the action deadline settles before that observer is cancelled and joined; there
is no outer action cancellation that detaches its JoinHandle. The existing
120-second / 1 MiB child supervisor and 64 KiB report limit also apply. The
raw connection intentionally uses the standalone CDP client; this is not a
production resource-profile or FFI ownership benchmark.

`cdp-tests.log`: **73** CDP unit tests, **11** generated-type tests and **3**
diagnostic tests pass; the one live diagnostic is ignored in the offline run.
`clippy.log`: strict release all-target Clippy passes. The shared fixture runner
also reruns the original four-control host diagnostic successfully in **1.93 s**
(`host-runner-regression.log`, integrity only). No production code, dependencies,
original parity assertions or addon binary changed.
Offline Node **31/31**, strict TypeScript and the **229-package** safe-host
dependency/JS boundary checks also pass (`node-unit.log`, `types.log`,
`boundary.log`). Final native discovery (`targets-final.json`) verifies all
**11 recorded owned target IDs**, including this task's preflight, are absent;
the one remaining unrelated Page was left untouched. All runners exited and
the Chrome process was not restarted. The foundation commit remains `f76e3f2`;
no additional commit or push was made.

After the same dedicated Chrome/Pwright preflight, select the explicit raw lane
and provide the **currently verified** browser WebSocket URL:

```sh
PWRIGHT_TEST_BIND='<verified Tailscale IPv4>' \
PWRIGHT_TEST_HOST='<verified full MagicDNS hostname>' \
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 \
PWRIGHT_CDP_WS_URL='<verified ws://127.0.0.1:9222/devtools/browser/...>' \
node packages/pwright/scripts/probe-reload-boundary.mjs raw
```

`host` remains the default lane and can also be supplied explicitly. Do not
replace the held body, wait for commit instead of the original signal, retry
close until success or adopt stopLoading merely to remove the red parity gate.
The original public **16/18** result remains open. This independent native
reproducer can now support a browser-level investigation while further public
API coverage proceeds; it does not block all other Node-FFI work.

## Remaining implementation gates

- Resolve the held-response Page-close gate with causal native evidence.
- Add public paired cases for Browser/Worker close,
  queued native settlement and topology/URL publication before Promise delivery.
- Extend POST coverage and verify cache/revalidation, redirect chains, failed requests,
  interrupted reload, beforeunload/dialogs and descendant-frame lifecycle cases.
  Ordinary/POST/timeout successes do **not** cover those behaviors.
- Keep full artifact-specific lifecycle and screenshot regressions separate from
  shared results; ordinary reload success does not prove complete shutdown.

NODE-013/014 intermittent attachment/window discrepancies remain independent
open work. This checkpoint does not declare them fixed or complete VS2/parity.
