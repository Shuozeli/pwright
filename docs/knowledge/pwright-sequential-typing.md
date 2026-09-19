<!-- agent-updated: 2026-09-09T03:44:00Z -->

# Sequential typing: Rust input foundation and Node integration gates

NODE-063 refreshes all 38 sequential-typing tests on addon 1b627f19 after public
Frame lookup integration. The correct fixture is `sequential-typing.html`;
an initial run mistakenly used `input.html` and failed 33 cases because it lacks
`resetTyping`. Both logs are preserved in `.work/node063-frame-lookup/`; the
corrected run is `typing-correct-fixture.log`. No typing implementation changed.
[Current artifact and related gates](pwright-frame-lookup.md#node-063-evidence-and-remaining-work).

NODE-050 verifies sequential typing through the opt-in Node addon in the shared
10-case drag/Keyboard/Locator corpus, using exactly the same action executor as
the Rust-host oracle. Public child-Frame typing and mid-typing detach remain open.
[Selection, evidence and remaining work](pwright-chromium-drag.md#node-050-native-lifetime-gates-and-selected-node-addon).

NODE-049 invokes the optional lifetime handoff after sequential-typing focus
and before Page input. Rejected handoff performs no key or text dispatch. Native
local/OOPIF `a中` cases still match; removal mid-sequential-input remains ungated.
[Contract, evidence and remaining gates](pwright-chromium-drag.md#node-049-optional-frame-to-page-lifetime-handoff).

NODE-048's real local/OOPIF sequential cases match native key events, focus and
input value for `a中`. This does not settle removal during an input continuation;
the accompanying Locator press detach cases fail.
[Frame gate](pwright-chromium-drag.md#node-048-native-frame-gate-exposes-a-lifetime-discrepancy).

NODE-047 optionally binds Frame focus separately from owning Page keyboard and
insertText in the Rust host. Sequential typing keeps its literal text semantics,
and selects the same transaction-local keydown provider as Locator press.
Default/addon behavior is unchanged. [Details](pwright-chromium-drag.md#node-047-locator-keyboard-selects-the-owning-page).

## Status

Subsequent NODE-031 now exposes raw `page.keyboard` through the same Page input
owner and Rust host/N-API. Its 24-case public comparison includes a held Shift
shared with Locator.press, plus type/insertText Unicode and fractional delay.
The subsequent same-artifact regression now restores all four held-modifier
catalog entries for both typing methods: **38/38** full input cases and **27/27**
actions pass. The earlier filter is removed, with catalog/assertions unchanged.
This does not settle every broader gate below.
[Current Keyboard and full regression evidence](pwright-raw-keyboard.md).

NODE-029 is **in progress**. Public Node `Locator.pressSequentially()` and the
deprecated `Locator.type()` alias invoke a typed Rust command through FFI.
The new opt-in interleaved Page input policy resolves the first blocking
discrepancy: its initial artifact passes an expanded **30/30** public typing
gate. Cross-action verification then exposed a literal-key/physical-code Shift
mapping error. The corrected artifact passes **30/30 typing + 27/27 actions**
and its shared corpus remains **46 match / 9 unsupported / 2 oracle-error**.
Historical exclusive-mode 27/28 failures and shared 46/9/2 results remain below;
they are not evidence for the new artifact. Full compatibility is not claimed:
the new full lifecycle regression was killed before completion after three
reported failures; screenshot setup first failed 12/12, then a same-artifact
recheck passed 12/12 without a browser restart or successful intervening cleanup.

## Typed public binding

Current Locator tracks are **1.23 / 2.6**, following independently frozen
1.22 / 2.5 baselines. `locator.press_sequentially` uses method **131095** and
command/result tags **45/46**. Its parameters are selector, literal UTF-8 text
and optional duration; timeout/cancellation remain in call metadata. Both
schemas generate capability identities, bounded schema models, server chains
and references. Old-minor rejection precedes encoding or dispatch; tests compare
identical cross-major command/result payloads and metadata round trips.

The safe host and backend reject owned text capacity above 1 MiB before queuing
the action. The codec applies both schema and configured string limits; the v2
delegate validates capacity **before** cloning into the v1 mapping. The existing
Locator resource lease covers the text owner alongside selector/action storage.
N-API takes the existing **6 MiB action ticket before conversion**, under the
64 MiB environment ceiling. No new worker, driver, dependency or browser is
introduced. The backend manifest declares `locator.press-sequentially`.

The default exclusive Page input coordinator is acquired before final strict
focus and retained through the text sequence and delay. Node now selects the
interleaved policy described below, retaining the Page state owner but not its
event gate across focus retries or delays. The backend reuses Press's
selection-reset focus, but not its navigation barrier. `noWaitAfter` is an
accepted boolean compatibility no-op. Errors do not retry partially sent text;
typed cancellation follows the selected input policy. Node converts values and
returns a Promise; focus, key events, delays and cleanup stay in Rust.

The existing FFI policy explicitly rejects unpaired UTF-16 with unsupported;
there is no replacement-character fallback. This limitation is tested and is
not equivalent to complete Playwright string compatibility. Interleaving a
second input during a held-key delay was a **confirmed discrepancy** in the
exclusive Node recipe. The interleaved recipe corrects that recorded example.

## Pinned upstream behavior

The inspected reference is local Playwright **1.58.2**, installed under
`tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/lib/`:

- `client/locator.js`: `pressSequentially()` delegates to `type()`, which sends
  Frame.type with strict selection.
- `server/frames.js`: retries a missing/disconnected selected element before
  invoking ElementHandle._type.
- `server/dom.js`: `_type` focuses with `resetSelectionIfNotFocused = true`,
  then invokes Page Keyboard.type. Unlike `_press`, it does **not** install a
  navigation-signal wait. Do not copy the complete Press implementation.
- `server/input.js`: iterate text by Unicode code point. A literal US-layout
  character uses key down, optional delay, key up; another character waits first
  and uses `Input.insertText`. This is not Fill or a DOM value setter.
- The layout's literal aliases encode their own case: typing `a` with Shift
  already held still emits key/text `a`, with the Shift modifier bit. Physical
  `KeyA` resolution is a different operation. Typing `Control+A` types those
  characters; it does not parse the whole text as a shortcut.

## Implemented ownership boundary

`pwright-input-model::InputText<'a>` validates a borrowed UTF-8 byte length of at
most 1 MiB. It does not allocate/copy text, bound the backing owner's capacity,
or own a resource lease. The new host/codec path separately admits the owned
String's **capacity** and lifetime as described above. Errors report byte counts,
not the input text.

`pwright-backend-cdp-input::type_text` accepts an existing policy-selected
`CdpInputTransaction`, bounded text, resolved platform flag and a caller-supplied
delay-future factory. Iteration uses a four-byte inline scalar buffer. There is
no text collection, new timer, task, transport, browser or selector dependency.
CDP encoding retains its existing per-command allocations and connection limits;
the primitive is not an allocation-free or RSS claim. The normal standalone
Pwright dependency closure remains exactly **five packages**.

The caller owns admission, the Page action and **one overall deadline**. Zero
delay must not create a timer. Delay is between down/up for layout characters
and before insertion otherwise. Empty text invokes no delay and sends no input.
On an error, no remaining characters are sent and text is never retried. The
caller finishes the transaction or invokes selected-policy cleanup after
cancellation; default-exclusive acknowledged keys remain recoverable there. This does not
establish upstream timeout-state parity (see existing NODE-022).

The live oracle exposed a pre-existing temporary-modifier discrepancy:
`transition_modifiers` used abbreviated CDP dispatch without location, producing
DOM `location=0` for left-side modifier aliases. It now uses typed rich dispatch
with `location=1` on both down and restoration. Modifier masks, fixed state size,
canonical transition order and ownership are unchanged. A regression checks all
four modifiers. This fix also applies to existing pointer modifier transactions.

## Input-foundation verification

Evidence: `.work/node029-sequential-typing.vs2gsP/`.

- `unit-complete.log`: **26 backend + 1 model** release tests pass, including
  native down/up/insert failure, literal case, empty text, delay ordering,
  cancellation during physical/Unicode input, cleanup, and byte admission.
- `locator-regression.log`: **100 Locator backend + 3 composition** release tests
  pass; the unrelated live composition case remains ignored in this run.
- `clippy-verified.log`: input/backend all-target release Clippy passes without
  suppressions. `dependencies-final.log`: input-only proof passes, five packages.
- `oracle.json` / `oracle.log`: freshly recorded Playwright 1.58.2 observations
  on dedicated macOS Chrome **152.0.7977.76**. Fixture capture is bounded at 1024
  events per case; overflow fails. Output refuses to overwrite prior evidence.
- `live.log`: initial **9/13** comparisons match; the four modifier cases expose
  the missing location field. Keep this failure, do not overwrite it.
- `live-final.log`: the location fix gives **13/13** exact DOM-event/value
  comparisons. `live-complete.log` records a later test-only metadata-guard
  failure: reduced `navigator.userAgent` is unsuitable for exact Chrome patch
  matching. The guard now checks typed native `Browser.getVersion` metadata.
- `live-verified.log`: final test target passes **13/13**, including printable
  ASCII, CJK, accents, combining sequences, supplementary/ZWJ characters,
  literal chord text, CR/LF/tab/NUL, repeats, empty input, a nonzero delay,
  each held modifier, and contenteditable. Event comparison includes key/code,
  keyCode/charCode, location, modifiers, repeat, input data/type, composition and
  trusted flags. This is exact event order/value parity, not timing precision.

Both runners create/dispose only their own temporary contexts and targets and
disconnect from the externally owned browser. Rust assertions run after context
disposal and target/context absence checks. The browser is never restarted or
closed. Cancellation unit tests use injected pending/error futures; the live
test does not prove Node cancellation, Worker teardown or UTF-16 conversion.

### Reproduce

Start/status the dedicated Chrome profile, then open a served fixture with
Pwright from a fresh task directory. Register `tests/fixtures` with serve-lib,
bound to the verified Tailscale IP, and verify the exact MagicDNS URL. Set
`PWRIGHT_CDP_WS_URL` to the **current** loopback WebSocket from `chrome-cdp status`.
Choose a new absolute `PWRIGHT_INPUT_ORACLE` filename for every oracle run.

```sh
export PWRIGHT_TEST_CDP=http://127.0.0.1:9222
export PWRIGHT_INPUT_FIXTURE=http://macbookpro-yuacx.tail8f3b66.ts.net:9384/pwright-shared/sequential-typing.html
pnpm --dir packages/pwright exec node scripts/probe-sequential-typing.mjs
CARGO_INCREMENTAL=0 cargo test --release --locked -p pwright-backend-cdp-input --test live_typing -- --ignored --nocapture
```

Run those commands from the repository after exporting the two required
caller-specific variables above. The oracle is a standalone diagnostic, not a
replacement for the supervised shared Node-FFI gate.

## Public verification and remaining gates

Evidence: `.work/node029-public-typing.R4GzWn/`. Initial artifact SHA-256:
`4a098d753a1951e830269a2dd48679cd0e2c5a798970ad3ffc257fbd42710dcd`.

- `typing.log`: **27/27** public tests pass, including both aliases over the
  nine non-modifier input cases, focused/unfocused input selection, empty text
  focus, strictness, missing-target timeout, readonly keys, noWaitAfter, explicit
  UTF-16/byte-bound rejections, Page close after a confirmed first key, and Worker
  termination with exact native Context/target absence. Helpers assert zero
  remaining command leases/operations/cleanup/pending calls after close.
- `shared-final/report.json`: complete 57-case run, **46 match / 9 unsupported /
  2 oracle-error**, zero value/candidate/setup/cleanup errors, 47.801 seconds.
  `type` now invokes the public API; no original expected value or classification
  was relaxed. Overall supervisor exit 1 remains correct, not success.
- `shared.log` retains the first fixture preflight failure: the case-metadata
  JSON was accidentally under a directory whose shared verifier requires HTML
  MIME. It now lives in `tests/data/sequential-typing-cases.json`; all consumers
  use that path. No MIME assertion was weakened and the original file was moved,
  not lost. The served HTML fixture is unchanged.
- The expanded public test checks a second input while the first is paused
  after a confirmed native key-down. Do not count the initial 27/27 as proof of
  this newer concurrency case; final-artifact results follow below.

### Final binding artifact and confirmed overlap discrepancy

The backend-manifest-complete artifact is
`cfc74e536ed6260e8137226f74fea7b62396c6b62b9a743733c826c4c61b4b4b`.
`composition-verified.log` passes 18 aggregate backend + 287 composition + 4
standalone Locator composition tests. The added v2 test routes sequential text
through the bound utility world, shares Page input, and verifies released action
and route resources. `rust-tests-verified.log` covers the capability/client/
codec/server/backend/host matrix, including 103 Locator backend, 30 capability,
26 client, 48 codec-unit, 40 codec-integration, 89 host-unit and 16 host/server
integration-unit successes (ignored live targets are reported separately).
The generator passes 57 library and 2 CLI tests; both baseline/output `--check`
commands pass. Input-model/backend regression passes 27, Node unit passes 20,
both TypeScript checks pass, and final all-target release Clippy is clean.
Normal closures remain 18 Pwright packages for standalone Locator composition
and 224 packages for the safe host.

`shared-verified/report.json` verifies this **same final artifact**, rather than
carrying forward the earlier binary's result: complete 57-case run, **46 match /
9 unsupported / 2 oracle-error**, zero value/candidate/setup/cleanup errors.
It ran from 11:33:38.173Z to 11:34:40.874Z on 2026-09-07 (62.701 seconds).
Supervisor exit 1 is expected for this incomplete compatibility gate.

`typing-expanded.log` exposes an initial **test isolation** problem: awaiting
the second call before closing the first call's Page held cleanup behind the
first 30-second delay. The test's 20-second timeout allowed pending FFI work to
affect the following test. That evidence is retained; it is not called a pass.

`typing-isolated.log` now finishes in 23.374 seconds with **27 pass / 1 fail /
0 cancelled**, supervisor exit 1. The remaining failure is a direct mismatch:

1. Confirm the first call has emitted native `keydown(a)` and is waiting during
   its 30-second character delay.
2. Start `type('C', { timeout: 1000 })` on the same Locator.
3. After a bounded two-second observation, Playwright's second call is resolved
   and the value is `aC`. Rust's call is still pending and the value is `a`.
4. Close only this test's Page, then await both calls before comparing. Rust's
   second call ends with Error from closure; it is **not** misclassified as a
   completed one-second TimeoutError. All helper cleanup assertions pass and
   the following tests, including Worker termination, pass.

The observation timer is diagnostic only; it does not replace the API's timeout
or turn a pending operation into a successful result. Root cause: the backend
holds `CdpInputTransaction` across the entire sequence/delay; the second action
waits in `begin_transaction`. Its cancellation hook also invokes shared
`cleanup_pending`, which waits on the same lock. There is no per-call ownership
proof that the waiting command acquired input before cleanup. This is broader
than literal text generation and may affect other concurrent input actions.

Next design/implementation must separate shared Page physical-key state,
per-action progress/cleanup ownership, and the chosen serialization policy.
Preserve bounded admission and existing default resource contracts; an opt-in
Playwright-aligned policy must let a second keyboard action progress without
cleanup stealing another action's keys. Do not remove the failing test, increase
its API timeout, retry text, or mark pending cleanup as successful to hide it.

Remaining gates include same-Page overlapping input, mid-typing element/document
replacement, exact timeout-state parity and unpaired UTF-16 preservation. The
existing pressed-key cleanup differs from some upstream timeout behavior
(NODE-022); do not close that discrepancy from ordinary typing success.

Raw Keyboard down/up also needs a separate optional capability and persistent
held-key lifecycle; the existing transaction `finish` releases tracked keys.
Do not expose raw Keyboard by bypassing that ownership contract.

### Concurrency implementation constraints (pre-policy design record)

Reinspection of pinned upstream `server/input.js` adds an important distinction:
`Keyboard.down` updates a Page-wide set of physical key codes and derives repeat
from that set; `Keyboard.up` deletes that code. This is **not a per-call key
reference count**. Two actions pressing the same physical key can therefore
produce a repeated down, followed by an up from either action. Normal explicit
key-up semantics must not be confused with cancellation cleanup ownership.
Upstream updates these sets before awaiting raw dispatch; our current exclusive
transaction records acknowledged dispatch. Native-error parity also needs an
explicit decision and tests before changing that ordering.

The narrow implementation sequence is:

1. Add deterministic ownership/admission tests first: cancel before acquiring
   input, cancel after down, two actions using the same key, and action B
   finishing while action A remains delayed. Missing-target resolution must not
   retain the Page input gate indefinitely. Keep the current failing public
   overlap test as the end-to-end acceptance gate.
2. Separate event-level Page state serialization from whole-action exclusion.
   Select the policy explicitly in composition; preserve existing exclusive
   Rust callers. A Node-compatible path must share physical state with all
   other input routes, not create a private typing-only coordinator.
3. Give each admitted action a bounded progress/retirement record, with identity
   available both when execution starts and when cancellation runs. Today
   `Service::call(request)` has no OperationContext, while
   `CommandCancellation::cancel(context, reason)` does. Cancellation currently
   discards that context. Do not infer ownership from the current lock holder,
   add an unbounded CallId map, or make browser-wide cleanup the completion proof
   for a command that never acquired input.
4. Distinguish command timeout from Page/Context shutdown. In the compatible
   policy, retaining Page key state after timeout may be required (NODE-022),
   while native in-flight operations and their leases still must retire before
   reporting completion. Page shutdown must quiesce admitted work and dispose
   only owned resources. A `try_lock` failure is not a cleanup receipt.
5. Re-run same-key repeat/up, different-key overlap, cross-Frame shared state,
   focus change, timeout-state, native-dispatch failure and Worker/Page teardown
   comparisons, plus dependency/resource proofs and the unchanged shared corpus.

These are implementation constraints, not a new completed scheduler or a claim
that the existing timeout-state difference is fixed. No coordinator policy was
changed in the public-binding artifact above.

## Opt-in interleaved Page input (2026-09-07)

`CdpInputPolicy::Exclusive` remains the default. Its prior whole-action mutex,
acknowledged-key state, modifier restoration and cancellation recovery are
unchanged. `Interleaved` selects a separate, fixed Page-owned state record and
event-level dispatch gate; no action holds that gate through selector retries,
typing/Press delays, or the whole mouse sequence. State is mutated before each
native dispatch, as in pinned Playwright. Individual native calls are still
serialized through their acknowledgement: this prevents multithreaded state /
dispatch reordering but does **not** prove upstream pipelined-native-ack parity.

The original policy was injected through `CdpLocatorProductionProfile::with_input_policy`
and the factory's `create_input_coordinator`; Chromium creates that coordinator
once per routed Page and passes the existing shared owner to Frame routes.
The Node host explicitly selects Interleaved. An explicitly supplied coordinator
continues to be the caller's ownership/policy choice; it is not replaced behind
the caller's back. No wire schema or JavaScript input engine was added.

The subsequent [independent Page input checkpoint](pwright-raw-keyboard.md) moves
the Node host source to `with_page_input`: one policy and retained-state quota,
independent of Locator installation. Every coordinator clone and transaction
keeps its state lease. Legacy profiles without this opt-in preserve the original
Locator-selected behavior. This source change is not yet in a rebuilt Node addon.

Interleaved keys are a fixed **16-code Page set**, not per-call references.
Repeated downs occupy no new slot. New codes fail with a typed limit error
before dispatch when full; an explicit up frees its slot. Mouse state is fixed
coordinates and a three-button bitmask. No new collection, clock, task, worker,
CallId map or dependency was added. This is finite state under the existing
Page/action admission limits, not a total-heap or RSS measurement. Existing
Locator text leases and N-API action tickets remain unchanged.

An interleaved action's `finish` restores only an explicitly requested temporary
modifier set; it does not release all Page keys/buttons. On cancellation the
input leaf has no command-local key cleanup obligation: Page state deliberately
survives, including after a native dispatch failure. The runtime still owns
operation cancellation and native retirement; returning from input cleanup is
**not** a native-retirement receipt. Page/Context closure and Worker teardown
still must settle their owned work and dispose exact owned resources. This
avoids conflating timeout-state compatibility with native command completion.

Evidence directory: `.work/node029-interleaved.jRHGpv/`.

- Initial interleaved artifact SHA-256:
  `e20765bf8497f512470b2bcc4266ebb5d0110c4bad633b1017d7fc729a781e81`.
  `typing.log` passes **30/30** in 29.705 seconds, including exact native event
  snapshots for different-key overlap, same-key repeat/up and a missing-target
  timeout during another action's held-key delay. The original overlap test
  was retained and strengthened with full captured event comparison. All helper
  resource checks and exact Worker Context/target disposal pass.
- `actions.log` is retained as **25 pass / 1 fail**: NODE-022 now keeps Shift
  after timeout correctly, but the subsequent `press('a')` incorrectly emits
  uppercase `A`, producing `Aother` instead of upstream `aother`.
- Upstream `server/input.js::buildLayoutClosure` stores literal aliases without
  shifted variants. `resolve_key` now applies held Shift only to physical codes
  such as `KeyA` and `Digit1`, while literal `a`/`1` retain their literal text.
  A pure mapping regression covers ASCII letter/digit/punctuation aliases and
  physical keys; an added public paired case compares four literal/code chords.
- `key-alias-tests.log`: **32 input + 103 Locator backend** release tests pass.
  The earlier full policy matrix passes 31 input, 103 Locator, 287 Chromium
  composition and 89 host tests (15 composition and 5 host live cases ignored).
- Input-only and Locator-only-composition proofs remain **5 / 18** Pwright
  packages; the safe host boundary remains **224** normal packages. No Node
  driver/transport/process imports enter production JavaScript.
- Rebuilt alias-corrected artifact SHA-256:
  `5a746ac883be46d8fe4516df6c65a92e24457b39bb47af60a61cb3b34f3ba900`.
  `typing-final.log` passes **30/30** in 29.065 seconds; `actions-final.log`
  passes **27/27** in 26.557 seconds, including the unchanged NODE-022 assertion.
  `shared/report.json` completes 57 cases at **46/9/2**, zero other errors,
  in 61.122 seconds and records this exact hash (supervisor exit 1 for incomplete
  coverage). `clippy-final.log`, 20 Node unit tests and TypeScript also pass.
  These final-artifact gates are separate from the first interleaved artifact.

Remaining concurrency coverage includes crossed Frame/world focus changes,
mid-action document/element replacement, cancellation at native enqueue/ack
boundaries, modifier changes between a physical down and up, and Page-level
state limits under repeated timeouts. Passing one overlap or timeout example
does not close these broader requirements or expose the raw Keyboard API.

### Broader regression and teardown evidence

The same final artifact did **not** pass the full lifecycle gate.
`lifecycle.log` records 18 passed / 3 failed completed cases out of 45 before
the 180-second supervisor killed the process (SIGKILL, exit 1, no complete TAP
summary). Reported failures are native outer-window geometry (oracle 0x0,
candidate 1282x800 with matching viewport/media), Worker readiness timeout and
topology-initialization timeout. These resemble recorded NODE-013/014 failures,
but this run does not establish their cause or rule out regression.

`screenshot.log` then fails all 12 cases during attachment, in 73.007 seconds:
both oracle connect and candidate topology timeouts occur before rendering
comparisons. `screenshot-recheck-no-cleanup.log` subsequently passes 12/12 in
15.091 seconds using the same artifact and Chrome process. **No successful
cleanup or restart occurred between these runs**; initial CLI attempts used a
native target ID where an internal tab ID was required and did not remove the
orphan. The recheck is evidence of intermittent setup behavior, not a causal
cleanup fix or permission to discard the red gate.

After all runners were terminal, the lifecycle fixture Page
`2236DDD7A423FE26814ED630DBB92532` remained. Its unique task fixture URL and marker
identify it as ours. A separate Pwright state directory explicitly reattached
that exact native target as `owned_lifecycle_orphan`, then closed it successfully.
`orphan-close.log` and `after-orphan-close.log` prove its absence; unrelated
browser-game Pages were preserved. No production browser or shared profile was
restarted. This manual cleanup is not a passing automated teardown assertion.
The later `.work/node013-lifecycle-trace.ccVzSR/lifecycle-watchdog.log` recovers
complete same-artifact evidence: **45/45**, 192.715 seconds, supervisor exit 0.
An intermediate bounded trace first records 44 passes before the old aggregate
180-second timer truncates the final connect only 3.4 seconds into its unchanged
30-second API deadline. The lifecycle supervisor now separates 600 seconds total
from an independent 60-second per-case bound (setup/action/cleanup included).
No API deadline, expected result, input policy or native binary changed. The
exact intermediate orphan was cleaned through task-specific Pwright state;
unrelated Pages and the shared Chrome process were preserved. This is successful
full regression evidence, not a causal fix for the earlier intermittent failures.
[Supervision contract and retained logs](../node-ffi-compatibility-design.md#node-013-lifecycle-supervision-evidence-2026-09-07).
