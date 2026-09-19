<!-- agent-updated: 2026-09-07T23:58:00Z -->

# Pwright process-memory attribution

NODE-047's selected controller provider allowance is now 86 MiB plus fixed
storage, with a separate 17.25 MiB retained Locator invocation allowance.
These are logical admission limits, NOT measured RSS or memory reductions.
[Accounting](pwright-chromium-drag.md#node-047-resources-and-evidence).

NODE-046's selected drag provider allowance is 83.5 MiB plus fixed storage per
controller after including Keyboard cancel owners. This remains logical resource
accounting, not eager allocation or measured RSS.
[Budget context](pwright-chromium-drag.md#node-046-selection-and-resource-boundary).

NODE-045 increases logical Mouse/CDP admission to preserve accepted batch
semantics. At 552 pending slots, the existing 4 MiB command/response limits yield
about 4.32 GiB of conservative declared wire allowance; the selected drag provider
allowance is 81 MiB per controller. These are NOT measured RSS or eager
allocations. Zero leases after close proves ownership return, not a memory
optimization. [Context](pwright-chromium-drag.md#node-045-maximum-click-batches-and-matching-host-admission).

## Context counts are not memory measurements

The separate [Node reconnect diagnostic](../node-ffi-compatibility-design.md#node-013-retained-world-diagnostic-2026-09-07)
observes named utility worlds surviving disconnect and appearing after an owned
document reload. The bounded sample reuses one Pwright world and adds one oracle
world per reconnect. This is not RSS, retained heap size, init-script inventory
or proof of an unbounded leak. It does not replace the workload recorder or
change this crate's dependency/resource boundaries. Changing topology and
overlapping connections remain uncontrolled dimensions requiring separate tests.

## Decision

OS process-memory observation is an optional diagnostic component, not a core,
runtime, browser-provider, or CDP dependency. The default implementation is the
standard-library-only `pwright-process-metrics` crate and matching command-line
probe. Users that do not select this diagnostic install no sampler or process
table integration. Its optional `ledger` feature depends only on
`pwright-core`; Chromium/CDP workload code remains outside the diagnostic crate.

The probe requires an explicit browser parent PID. An attached CDP endpoint is
not process ownership and does not imply a trustworthy PID-discovery right.
Owned Chromium launch may pass its existing process-lease PID explicitly;
external attachments must obtain the PID through caller policy.

## Measurement contract

One sample reports these disjoint views:

- the selected Pwright client/runtime process RSS;
- the browser parent process RSS;
- renderer process count and summed RSS;
- GPU process count and summed RSS;
- utility process count and summed RSS;
- other descendants and the total browser-tree RSS sum.

Chromium roles are classified from `--type=renderer`, `--type=gpu-process`, and
`--type=utility`. Descendants are attributed through the bounded PID/PPID graph,
so Linux zygote intermediates and nested helpers remain inside the browser tree.
Unrelated processes are excluded. Missing roots, duplicate PIDs, parent cycles,
numeric overflow, malformed rows, and overlapping runtime/browser roots fail
explicitly.

RSS is an operating-system observation, not a Pwright resource limit. Summing
RSS may double-count shared pages; macOS `ps` does not provide Linux-style PSS.
Reports and benchmarks must label the browser total as an RSS sum and must not
compare it directly with the logical retained-byte ledger.

## Resource boundary

`ProcessMetricsLimits` independently bounds:

- process-table snapshot bytes;
- process record count;
- bytes inspected in one command line.

The system source reads at most `snapshot_limit + 1` bytes from `ps`. If that
sentinel byte appears, it terminates and reaps the sampler before returning a
typed size error. Parsing borrows command text only long enough to classify a
role; the retained snapshot contains only PID, PPID, RSS, and a compact role.
The probe starts no background task and keeps no sample history.

## Caller-driven workload recorder

`MemoryWorkloadRecorder` pre-reserves an explicitly bounded observation vector
and never starts a task, sleeps, connects to a browser, or performs browser
work. The caller owns those policies. A `MemoryWorkloadPlan` records:

- one of idle, navigation, repeated Page creation, Handle, network, tracing, or
  teardown workload kinds;
- the exact warmup and steady observation counts;
- the target sample interval, retained as experiment metadata without rejecting
  scheduler jitter; and
- a teardown deadline that is enforced between the pre- and post-teardown
  observations.

The fixed sequence is cold baseline, cleaned warmup samples, steady samples,
pre-teardown, and post-teardown. Every observation contains OS process
attribution, the connection's `ResourceMetrics`, explicit Context/Page/Frame/
Worker/Handle counts, pending calls, and queued events. A run rejects changing
runtime or browser root PIDs, out-of-order phases, backward time, capacity
overflow, incomplete reports, and teardown that exceeds its deadline.

`MemoryWorkloadSeriesRecorder` adds an exact, pre-reserved run count above
complete reports. Every run must use the identical workload plan, return
logical state to its cleaned baseline, and settle the browser process count
before admission. The completed series reports min/max runtime and browser
RSS-sum deltas without treating either range as a pass/fail budget. A workload
may explicitly mark a byte class as reusable variable payload; item owners,
scopes, and leases remain baseline-checked, while the byte value may vary under
its existing hard ledger ceiling. Strict byte comparison remains the default.

Cold startup and reusable connection state are intentionally distinct. CDP
traffic may grow a bounded reusable receive buffer during warmup without
leaking a browser object. Reports preserve the cold baseline but compare
post-teardown logical state with the final cleaned warmup observation when one
exists. Current resource units, live scopes, active leases, and object counts
must be no greater than that comparison baseline; monotonic peak/rejection
counters are diagnostic history and are not expected to decrease.

Logical cleanup and native process settlement are separate report predicates.
Warmup establishes the comparison baseline after the browser process count has
held steady for a bounded window; it is not required to return to the cold
count, because first use may legitimately start a persistent GPU, utility, or
extension process. This run exposed exactly that case when Chrome's extension
service worker changed a cold five-process tree into a stable six-process tree.
After Page/Context teardown, a caller must not assume the first process sample
is steady: Chrome may retain short-lived renderer/helper processes after its
portable objects and Pwright leases are already gone. The Chromium workload
drivers therefore poll the bounded process probe until total browser process
count is no greater than the cleaned warm baseline, or fail within a declared
timeout. Only then do they record `PostTeardown`; they never wait for RSS to
match an arbitrary value.

## First dedicated-Chrome evidence

On 2026-09-04, the release probe sampled the externally managed Chrome 152 CDP
profile while using its own short-lived process as the runtime root:

```text
runtime             1 process       1,654,784 bytes RSS
browser parent      1 process     365,592,576 bytes RSS
renderers           3 processes   478,396,416 bytes RSS
GPU                 1 process     184,123,392 bytes RSS
utility             3 processes   319,766,528 bytes RSS
browser RSS sum     8 processes 1,347,878,912 bytes RSS
```

These values are one diagnostic observation, not a budget or stable benchmark.
They already demonstrate why “Pwright uses 1.35 GB” would be an invalid claim:
the probe process and the browser process tree are separately attributable, and
the Chrome total is explicitly an RSS sum. `chrome-cdp status` remained ready
after sampling; the probe held no browser termination authority.

## First repeated Page/Context workload evidence

On the same date, the ignored release integration test
`live_memory_workload` attached the production Chromium composition to the
dedicated Chrome 152 process. It performed one cleaned warmup cycle followed by
three steady cycles. Each cycle created and closed one BrowserContext, one Page,
and its main Frame through the portable services. Seven observations covered
the cold baseline, warmup, three steady samples, pre-teardown, and
post-teardown.

The post-teardown topology had zero Contexts, Pages, and Frames, and its pending
call and control-queue counts were zero. Current ledger units, scopes, and
leases returned to the cleaned warm baseline. A rerun with the browser-process
settlement gate observed post-minus-cold RSS deltas of `+1,376,256` bytes for
the Rust test process and `+2,015,232` bytes for the Chrome process-tree RSS
sum. These are evidence that the measurement path works, not a memory budget
or a claim of stable overhead.

## First navigation workload evidence

The ignored `live_navigation_memory_workload` release test uses the same
observation and settlement helpers. It first performs a complete throwaway
Context+Page+`goto(wait_until=load)`+close cycle, records the cleaned warm
baseline, then creates one measured Page and performs three Playwright-shaped
`goto` operations against data URLs. Each steady sample has one Context, Page,
and main Frame with zero pending calls and zero queued control messages. Final
Page/Context close returns logical state to the warm baseline.

The first version sampled immediately after portable close. It correctly found
zero logical browser objects but also saw five short-lived Chrome helpers:
browser process count changed from 8 to 13 and the instantaneous RSS-sum delta
was `+585,302,016` bytes. This was a measurement-timing defect, not evidence of
a Pwright ledger leak. With bounded process settlement, the verified run ended
at 8 browser processes and observed `+1,327,104` bytes for the Rust test process
and `+1,605,632` bytes for the Chrome RSS sum. The driver now repeats the whole
fresh attachment/runtime workload three times. A Chrome 152 series held the
browser at 6 processes in every run and observed runtime RSS deltas of
`+131,072..+1,310,720` bytes and browser RSS-sum deltas of
`+229,376..+4,947,968` bytes. These ranges remain diagnostics, not budgets.

## First retained-Handle workload evidence

The ignored `live_handle_memory_workload` release test installs Page lifecycle,
Handle lifecycle, and Handle evaluation as separate optional components. Its
first proposed profile was rejected before Chrome attachment because runtime
capacity 20 could not cover topology capacity 20 plus 8 live Handles. The final
profile states the real worst-case composition explicitly: 8 topology objects
plus 8 Handle objects fit a 16-object runtime.

After one complete create/evaluateHandle/dispose/Page-close warmup, the measured
Page retains three ElementHandles. Correlated steady samples prove that route
count, runtime live-object count, and ledger `RemoteHandle.current` advance
together as 1, 2, and 3. Teardown explicitly disposes one Handle, then Page
close parent-retires the remaining two. Including warmup, final route metrics
are `created=4`, `explicitly_disposed=2`, `parent_retired=2`, `current=0`, and
`retained_bytes=0`; runtime returns to its Browser-only object and the shared
ledger holds zero RemoteHandles. The joint verified run observed
post-minus-cold deltas of `+1,490,944` bytes for the Rust test process and
`+5,586,944` bytes for the settled Chrome RSS sum. These remain diagnostics,
not budgets.

## First idle external-attachment evidence

The ignored `live_idle_memory_workload` release test measures the smallest
valid prepared composition: one independently selected Context-lifecycle
capability, no Contexts or Pages, and an externally owned Chrome attachment.
The cold sample is taken before CDP attach. Warmup and three steady samples hold
only the CDP connection plus the portable Browser identity.

Teardown calls the controlled runtime's transport-close boundary, which
invalidates exactly that one Browser object with zero pending calls. It then
drops every strong `CdpConnection` owner and waits for the connection's sticky
closed signal before recording post-teardown. The retained ledger observes
`LiveObject.current` move from 0 before attach, to 1 while idle, and back to 0.
The test subsequently creates a fresh attachment and successfully calls
`Browser.getVersion`, proving Pwright released its session without terminating
the external browser. The verified run observed `+1,032,192` bytes for the Rust
test process and zero Chrome RSS-sum delta. These remain diagnostics, not
budgets.

## First network-lifecycle workload evidence

The ignored `live_network_memory_workload` release test installs the optional
single-cursor network driver before Page creation and connects the Page and
BrowserContext close services to its non-owning registration handle. A complete
warmup Context+Page+network-navigation+close cycle must release the only Page
lane before the measured Page can be created, so the test proves actual capacity
reuse rather than merely observing a zero counter.

The measured Page performs three data-URL navigations. For each one, the driver
matches the exact navigation Request URL, observes its Response and terminal
event, applies portable Request/Response object lifecycle behind the ordered
Create fence, acknowledges the exact Page batch, flushes terminal controls, and
samples only with zero active native request correlation and an empty control
outbox. Page and Context teardown return the driver lane, topology objects,
runtime objects, and logical resource state to the cleaned warm baseline.

The first correlated run exposed the portable-retention gap directly:
`LiveObject.current` was `6`, `8`, then `10`. Terminal events now own complete
Request and optional Response snapshots, and successful publication immediately
retires the temporary Request root plus Response child. The repeated real-Chrome
workload therefore holds `LiveObject.current` at `[4, 4, 4]`; Browser, Context,
Page, and main Frame are the only objects present at every steady sample.

The correcting Chrome 152 run recorded seven observations. Runtime RSS changed
from `8,388,608` to `10,059,776` bytes (`+1,671,168`), while the settled Chrome
tree changed from 10 processes and `1,609,121,792` bytes RSS sum to 8 processes
and `1,377,501,184` bytes (`-231,620,608`). As with the other first runs, these
are diagnostic observations and cleanup evidence, not a stable overhead estimate
or budget.

The follow-up series runs three independent attachment/runtime instances, each
with that warmup plus three steady navigations and full teardown. The current
production profile selects the task-free reclaim-after-all-read fanout with a
subscriber capacity derived from its network and navigation plugins. A CDP
command-response ordering barrier drains unrelated tail events after every
navigation and teardown; direct fanout metrics require zero queued events at
each boundary. No resource class, including `EventBytes`, is exempt from exact
warm-baseline cleanup.

Two consecutive three-run Chrome 152 series passed this stricter contract. All
six runs held steady live objects at `[4, 4, 4]`. The first series observed
runtime RSS deltas of `+65,536..+1,703,936` bytes and browser RSS-sum deltas of
`-99,860,480..+638,976` bytes while an initially present Chrome helper settled
the browser process count from 10 to 9. The immediately repeated, process-stable
series observed `+98,304..+1,425,408` runtime bytes and
`+884,736..+5,357,568` browser RSS-sum bytes with 9 browser processes before
and after every run. These ranges remain diagnostics for allocator and browser
behavior, not proposed budgets.

After closing a potential `Notify` registration race in the fanout receiver, a
third strict series again passed at 8 -> 8 browser processes. It observed
`+65,536..+1,556,480` runtime bytes and `+933,888..+4,980,736` browser RSS-sum
bytes, with all three steady object vectors still `[4, 4, 4]` and every fanout
boundary empty.

After correcting warmup settlement to admit a stable first-use helper, another
three-run Chrome 152 network series passed at 6 -> 6 browser processes. It
observed `+81,920..+1,490,944` runtime bytes and
`+212,992..+4,849,664` browser RSS-sum bytes, again with `[4, 4, 4]` steady
objects and exact logical cleanup.

Legacy/v1 connections retain the Tokio broadcast API and its replay-ring
semantics. The strict v2 workload deliberately selects the production fanout,
so a consumed event is reclaimed as soon as every current cursor advances; an
event published with no subscribers is never retained.

The complete-capability Chrome 152 concurrency proof adds a high-water mark to
those fanout diagnostics. Its profile derives a safe capacity of 10 cursors
from topology, PageLoad, four possible Page-network lanes, two navigations, and
two Locator actions. The default recipe instead installs one shared network
cursor, so its real four-command run retained three steady cursors and reached
a peak of 6 or 7 while two slow navigations and two Locator clicks overlapped.
It finished with three current cursors, zero queued events, zero lag, and an
externally owned browser that was still reachable. The peak is an observed
composition diagnostic, not an RSS budget.

## First owned-browser teardown evidence

Owned shutdown cannot use an ordinary `PostTeardown` process attribution:
success means the browser root PID no longer exists. The bounded process probe
now produces a non-constructible `ProcessExitMetrics` only when one snapshot
contains the runtime root and does not contain the exact owned PID.
`MemoryWorkloadRecorder::finish_owned_browser` accepts that proof for the final
phase while retaining ordinary two-root attribution for baseline, warmup,
steady, and pre-teardown observations. It still verifies root identity,
monotonic time, teardown deadline, resource leases, scopes, and object cleanup;
it does not invent a zero browser RSS value or claim that missing-root RSS is a
measured browser-tree sample.

The ignored launch-composition workload runs three independently owned headless
Chrome 152 processes with fresh temporary profiles and records six running
observations for each. All three completed through the graceful provider
shutdown path. Pre-teardown browser RSS sums ranged from
`1,204,158,464..1,208,762,368` bytes and runtime teardown deltas ranged from
`+32,768..+589,824` bytes. Every provider report named its exact PID and
profile, every post-teardown snapshot proved that PID absent while the Rust
runtime remained, all resource leases returned to zero, and every test-owned
profile directory was removed. These are lifecycle diagnostics, not a memory
budget; there is intentionally no post-exit browser RSS range.

## Remaining DT-003 work

- run repeated idle, Page/Context, and Handle series, and repeat the
  implemented tracing workload and network series across additional named
  hosts and versions;
- add Linux PSS where available without changing the portable RSS contract;
- establish reviewed budgets only after repeated measurements on named hosts.

The current slice makes process attribution plus correlated idle external
attachment, Page/Context, navigation, retained-Handle, network-lifecycle, and
tracing workloads executable. The first tracing run used one cleaned warmup
chunk plus three measured real-Chromium `goto` chunks. Every sink handoff
returned `TraceBytes.current` to zero; full trace and native target teardown
passed, with runtime RSS `+851,968` bytes and browser RSS-sum `-257,785,856`
bytes as the Chrome process tree settled from 11 to 8 processes. These are
diagnostics, not budgets. An immediate final-source rerun passed the same 11 to
8 settlement and observed `+327,680` runtime bytes and `-232,587,264` browser
RSS-sum bytes. This does not claim that the original Playwright memory issue
has been diagnosed or that DT-003 is complete.
