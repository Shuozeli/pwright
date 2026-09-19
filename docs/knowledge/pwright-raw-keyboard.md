<!-- agent-updated: 2026-09-08T01:19:49Z -->

# Raw Page Keyboard: Rust primitives and Node integration plan

NODE-050's isolated `drag-runtime` addon passes all 24 public Keyboard cases and
10 shared native drag/Keyboard/Locator cases. Original Page input state survives
Frame detach but is canceled on owner closure in the 18-case Rust Frame gate.
These are selected-artifact results, not a default-addon rebuild.
[Selection, evidence and remaining work](pwright-chromium-drag.md#node-050-native-lifetime-gates-and-selected-node-addon).

NODE-049 transfers selected Locator input lifetime to the owning Page after
focus. Raw Page Keyboard semantics are unchanged. Twelve native Frame cases
match, including Page key reuse after Frame removal; full Node parity is open.
[Contract, evidence and remaining gates](pwright-chromium-drag.md#node-049-optional-frame-to-page-lifetime-handoff).

NODE-048 proves native child/OOPIF key routing, including repeat after timeout.
A distinct Locator lifetime defect remains: Frame removal cancels an already
Page-owned key delay in Rust. The ten-case gate fails at 8/10.
[Evidence](pwright-chromium-drag.md#node-048-native-frame-gate-exposes-a-lifetime-discrepancy).

NODE-047 extends the same transaction-local Page interceptor to Locator press
and sequential typing, while retaining Frame focus. Ten native host cases now
include Locator Escape and timeout/repeated-key state. Addon selection is
unchanged. [Details](pwright-chromium-drag.md#node-047-locator-keyboard-selects-the-owning-page).

NODE-046 adds optional transaction-local native keydown interception. The
selected Rust host cancels drag on Escape after logical state updates, consumes
only successful cancel keydown and retains pending native cancellation through
Page quarantine. Five native scenarios match pinned Playwright; the addon is
unchanged. [Contract and evidence](pwright-chromium-drag.md#node-046-optional-rust-keyboard-escape-interception).

## Status

NODE-031 is in progress. The optional existing `pwright-backend-cdp-input` leaf
now exports `press_keys` and transaction `key_down_named` / `key_up_named`.
Real Chrome comparison matches **14/14** pinned Playwright raw Keyboard cases.
Keyboard 1.0 capability, codec, client extensions and generated server chain now
pass **19** tests. The optional typed native backend now passes **18** unit tests
and the same **14/14** real-Chrome comparisons. The optional single-Page wire
composition now passes **11** tests and **14/14** full wire-to-Chrome comparisons.
Aggregate Page input is now independently selectable and retained-state-budgeted;
its initial seven new tests pass. The optional controlled adapter and Rust
host/N-API/public `page.keyboard` are now implemented. The new release addon
passes **24/24 public API comparisons**, including exact unknown-key messages,
Locator sharing, navigation, Page isolation and close-during-delayed-press.
Shared coverage is now **52 match / 3 unsupported / 2 oracle-error**, with no
value mismatch, candidate, setup or cleanup error. See the latest checkpoint below.

## Upstream and API boundary

The local pinned Playwright 1.58.2 `lib/server/input.js` owns Page Keyboard state.
`press` splits the key expression, dispatches down in order, waits once if a delay
was supplied, then dispatches up in reverse order. Every down and up resolves
its string against current modifiers. `ControlOrMeta` uses the **driver process
platform**, not navigator.platform in a possibly remote browser. The Rust
primitive takes an explicit platform flag; the future host must supply it.

Raw Keyboard does not choose, scroll or focus an element. In particular,
`page.keyboard.press` cannot be implemented as `page.locator(...).press` or a
DOM value setter. An unknown later token can fail after an earlier modifier
was pressed. Future raw commands must preserve that prefix state, not copy the
Locator's eager whole-expression validation policy into this separate API.

`KeyTokens::parse_raw_keyboard` specifically retains the empty last token in
`Shift+`, and one empty token for an empty expression. Resolution fails at that
token's ordered step: `Shift+` leaves Shift pressed. The original strict `parse`
keeps rejecting empty/trailing tokens before dispatch. Both parsers retain the
16-token bound; exceeding it remains a declared resource-limit rejection, not
unbounded Playwright parity.

Implementation tasks remain:

1. **Contract layer implemented:** optional backend-neutral Keyboard 1.0 with
   five Page-targeted commands, bounded owned parameters, codec, independent
   client extensions and generated server bindings. Canonical schema owns
   methods 851969–851973.
2. **Typed native services, operation admission and single-Page composition
   implemented:** integrate the recipe into the aggregate host (still pending). Reuse the exact
   Page input coordinator used by Locator and future Mouse commands; do not create
   an independent keyboard state per call, command service or JavaScript wrapper.
3. Integrate controlled runtime admission, native retirement and Page/Context/
   Worker lifecycle. Command cancellation and physical key release are different
   contracts; neither an empty command queue nor a cleanup acknowledgement proves
   native work has retired. Preserve caller-selected input policy and budgets.
4. Add the safe-host and N-API methods with admitted conversion/owned-string
   capacity checks before copying. Expose a thin Page-owned Keyboard wrapper in
   Node, using upstream option shapes rather than adding fake Locator timeouts.
   No JS CDP, Node driver, key parser, delay loop or browser launch fallback.
5. Run paired public API tests before enabling the shared press/keyboardPress/
   keyboardType adapters. Include focus preservation, modifier state shared with
   Locator/Mouse, repeated keys, errors, overlapping calls, delayed native
   settlement, Worker teardown and post-close calls. Retain full-artifact lifecycle
   and screenshot gates separately from input-only successes.

## Implemented mechanics and ownership

`KeyTokens` retains at most 16 borrowed slices. `press_keys` adds no collection,
timer, task or input copy. It borrows the caller's transaction and token storage;
the caller supplies one delay future and owns timeout/cancellation. Token count
does not bound the original string owner's capacity. Typed arguments now check
capacity, but the future host must also admit resource leases before conversion
and queuing. `KeyInputError` distinguishes
unknown aliases from native errors without retaining the key expression.

Named operations in the interleaved policy acquire the existing native event
gate **before** reading modifiers/resolving the alias and updating Page state.
The gate remains through that event's native acknowledgement, not the whole chord
or its delay. Keyup resolves its physical-code alias again; a KeyA pressed as `a`
can therefore release as `A` if another action held Shift during the delay.

The existing ResolvedKey methods retain their behavior and share the same private
dispatch functions. Existing Locator and typing code is not redirected through
the raw press helper in this checkpoint. Default exclusive input still releases
acknowledged keys during finish/cancellation recovery. Opt-in interleaved input
keeps Page logical state across command finish, cancellation and native dispatch
failure until explicit keyup or Page retirement. A failed native key event does
not trigger retry, synthetic release or a claim of native retirement.

The primitive adds no dependency: its normal closure remains four Pwright
packages (input leaf, input model, CDP, core). Four new optional contract crates
are workspace members but are not installed into the host. The Node addon has not been
rebuilt for this additive primitive checkpoint; do not attach its evidence to
the previous addon SHA as new public Keyboard verification.

## Verification (2026-09-07)

Evidence: `.work/node031-raw-keyboard.lzq93H/`.

- `unit.log`: **40/40** input tests, including eight new raw-key tests. They
  cover chord order and one delay, cross-transaction repeat state, modifier changes
  during delay, unknown-prefix state, interleaved cancellation, default-exclusive
  cleanup, native failure without retry and literal plus/smart modifier parsing.
- `clippy.log`: release all-target Clippy passes without suppressions.
- `locator-regression.log`: existing Locator backend **103/103** tests pass.
- `dependencies.log`: unchanged normal four-Pwright-package input closure.
- `oracle.json` / `oracle.log`: fresh Playwright 1.58.2 observations, written only
  after all owned Contexts and the oracle connection are closed. Refuses to
  overwrite evidence; snapshot event arrays are capped by the existing fixture.
- `live.log`: **12/12** cases match in one Rust integration test, **2.69 seconds**.
  Comparison covers full DOM key/input events, text value and unknown-key outcome
  plus subsequent modifier behavior. Every case owns a fresh Context; native
  target and Context absence are verified after cleanup, including failure paths.
- `final-unit.log`: **42/42** input and **103/103** Locator tests after adding the
  separate raw parser and two tests for its empty-tail timing and unchanged
  strict parser/token bound. `final-clippy.log` passes all input targets.
- `oracle-expanded.json` / `oracle-expanded.log` and `live-expanded.log`:
  **14/14** comparisons pass in **3.05 seconds**, adding empty expression and
  empty trailing token with subsequent Shift-sensitive input. The original
  12-case evidence is retained, not overwritten.
- `final-targets.log` / `expanded-final-targets.log`: both task-owned preflight
  targets (`2932D510D7634C23931CA36918D7F0BA` and
  `652FBF3561D3480FE89E4618C261EBC8`) are absent after exact close. All runners
  exited; four unrelated game Pages remain. Chrome was not restarted.
- The existing addon SHA remains
  `8d973a18bf26623288efb0798f0b792a4345b206f7e6db8b9f3d2b868e53930b`:
  no addon rebuild, shared reclassification, commit or push occurred here.

Cases include literal/physical keys, Shift, literal plus, ControlOrMeta, repeat,
keyup after a Shift change, literal input with held Shift, unknown-key prefix,
keyup of an unheld key, keypad and multiple non-modifier chord tokens. They
include empty-token error timing and resulting Page state, but do
not prove arbitrary Unicode/UTF-16 behavior, every platform layout, public Node
overlap or native in-flight cancellation. The delayed interleaving proof in this
checkpoint is a deterministic Rust test, not a new live Node concurrency test.

After dedicated Chrome/Pwright preflight and verified serve-lib registration:

```sh
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 \
PWRIGHT_INPUT_FIXTURE=http://macbookpro-yuacx.tail8f3b66.ts.net:9384/pwright-shared/sequential-typing.html \
PWRIGHT_INPUT_ORACLE=/absolute/fresh-task-directory/oracle.json \
node packages/pwright/scripts/probe-raw-keyboard.mjs

PWRIGHT_CDP_WS_URL='<verified loopback browser WebSocket>' \
PWRIGHT_INPUT_FIXTURE=http://macbookpro-yuacx.tail8f3b66.ts.net:9384/pwright-shared/sequential-typing.html \
PWRIGHT_INPUT_ORACLE=/absolute/fresh-task-directory/oracle.json \
CARGO_INCREMENTAL=0 cargo test --release --locked \
  -p pwright-backend-cdp-input --test live_keyboard -- --ignored --nocapture
```

The fixture URL is a temporary serve-lib registration; verify it again before
reuse. Run browser lanes serially and never restart Chrome or close another
task's Pages to obtain a passing result.

## Typed Keyboard 1.0 checkpoint

Source: `crates/pwright-cap-keyboard/schema/keyboard.yml`, capability
`pwright.keyboard`. This is an unpublished initial contract, not a released ABI.

| Command | Method | Command/result tags | Parameters |
|---|---:|---|---|
| keyboard.down | 851969 | 1 / 2 | key |
| keyboard.up | 851970 | 3 / 4 | key |
| keyboard.press | 851971 | 5 / 6 | key, optional delay |
| keyboard.insert-text | 851972 | 7 / 8 | text |
| keyboard.type | 851973 | 9 / 10 | text, optional delay |

All commands target Page and return unit. Call metadata preserves caller deadline
and cancellation identity independently of key delay; Node's actual option shape
remains a later adapter responsibility. Independent client traits permit a
down-only channel and work without a main Frame.

`KeyboardKey` owns at most **128 capacity bytes**, `KeyboardText` at most **1 MiB**.
Owned constructors check capacity even when visible length is short; copy
constructors check length before copying. Key syntax is not prevalidated, to
preserve ordered unknown/empty-key errors. These are UTF-8 types, not a complete
Node UTF-16 contract. Encoding borrows input; decoding transfers owned strings
into the validated types. Generated field bounds still apply if a codec's
per-string limit is larger, and a caller can select a stricter codec ceiling.
Error message/detail strings each have a 4096-byte schema limit. No attachments
are accepted. Frame/host admission and native retirement remain separate duties;
bounded strings are not resource leases.

Commands and results verify outer/inner call identity. Commands also verify Page
identity and method. Payload variants must match the typed operation even when
a result header contains no method. Generated common duration constraints apply;
the codec preserves every operation failure kind rather than reclassifying it.

Evidence directory: `.work/node031-keyboard-contract.q15hJq/`.

- `typed-tests-final.log`: capability **3/3**, client **2/2**, codec **13/13**,
  server **1/1**. Parameters, delay, control metadata, success and all error kinds/
  details roundtrip. Forged route identity/method/variant, attachments and excess
  key bytes reject. The generated chain dispatches all five commands, returns
  decodable replies and advertises the complete capability.
- `clippy.log`: all four crates pass release all-target Clippy without bypasses.
- `contracts.log`: exact normal closures are **3 capability / 6 client / 7 codec /
  10 server** Pwright packages. No native backend, Locator, Tokio, Node or generator
  is required. Canonical method ownership and all generated outputs/reference
  pass the new CI check.
- `input-boundary.log`: prior input-only example stays at **5** Pwright packages
  including itself; `host-boundary.log`: safe host remains **224** normal packages
  and runtime JS still has no CDP/driver/process-launch imports.
- `check.log` and `typed-tests.log` preserve initial development failures: a wrong
  metadata field name and a client test that dropped its connection owner before
  invoking the weak Page handle. Corrections use the actual cancellation field
  and retain the Client. These were offline test/construction errors, not browser
  discrepancies.

Run `node scripts/check-v2-keyboard-contracts.mjs` and release tests for the four
`pwright-{cap,client,codec,server}-keyboard` crates. There was no browser run,
addon rebuild, shared reclassification, commit or push in this contract checkpoint.
At that checkpoint native services were pending; the following checkpoint adds
them. Page routing/composition and Node FFI remain separate, unverified gates.

## Bounded typed native backend checkpoint

`pwright-backend-cdp-keyboard` is a separate optional leaf, not a dependency of
the contract crates or the current Node host. It provides independent down, up,
press, insert-text and type services. `CdpKeyboardServiceFactory::bind` requires
the caller's existing Page CDP client and `CdpInputCoordinator`; all five services
retain that same state. It performs no selector resolution, focus, DOM setter,
navigation wait, process launch or autonomous scheduling.

The caller injects `KeyboardPlatform` and `CdpKeyboardDelay`. No clock fallback
exists; zero/absent delay does not invoke the clock. Press waits once per chord,
type once per character, and insert-text sends one native insertion. Playwright's
held-key behavior requires explicit `Interleaved` selection at Page construction.
The backend preserves `Exclusive` when supplied, including its finish and
cancellation key recovery; it never silently changes the caller's policy.

### Admission and retirement boundaries

- The production profile declares an argument-capacity ceiling (at most 1 MiB),
  nonzero concurrent-operation count, explicit resource scope, and checked byte
  requirements: `(argument ceiling + 2 * 4096) * operation count`.
- `Service::call` reserves an operation slot and a `BackendStateBytes` lease
  **before returning its future**, charging actual argument capacity plus 8192
  diagnostic bytes. All Page bindings and command kinds from that factory share
  admission. Oversized arguments and exhausted slots/ledger fail before CDP.
- Dropping an unpolled, waiting or cancelled future drops its owned parameters
  before its lease and releases the slot. A failed ledger reservation rolls back
  the slot. Successful/failed execution retains admission through policy-selected
  transaction finish. Cleanup failures retain the `Cleanup` category.
- Native error formatting is bounded before classification can allocate a
  lower-case protocol-message copy. Oversized diagnostics become explicit
  `ResourceExhausted`, not a silently truncated backend failure.
- This budget covers operation-owned argument storage and diagnostic allowance,
  **not total process memory**. Factory/Page binding allocations, fixed input
  state, transport/native request buffers and results retained after return need
  their respective caller/host owners and budgets. Portable input capacity
  validation is not pre-conversion admission; the future host must add that.
- `CommandCancellation` runs only policy-selected key recovery. Local lease
  release, cancelled polling and keyup acknowledgement are not proof of native
  command retirement. Composition must still integrate Page/Context closure,
  controlled runtime cancellation and late native completion.

### Evidence

Directory: `.work/node031-keyboard-backend.DRTn5c/`.

- `unit-final.log`: **18/18** backend tests cover eager admission, unpolled drop,
  cross-Page quota, capacity versus length, ledger rollback, shared input state,
  unknown-prefix timing, delay/insertion behavior, both cancellation policies,
  cleanup and native failure categories, diagnostic/chord/profile limits, and
  undeclared production requirements.
- `regression.log`: input **42**, Locator **103**, contract **3/2/13/1** tests
  pass; this first combined run used the initial 15 backend tests. The final
  backend run adds three admission/error tests without changing those crates.
- `oracle.json` / `oracle.log`: fresh Playwright **1.58.2**, Chrome
  **152.0.7977.76**. `live.log` and `live-final.log` compare **14/14** cases through
  the typed native services, including full DOM events/value and subsequent key
  state after errors. Each attempt verifies exact owned Context/target absence
  and zero active backend operations/leases. These live cases cover down/up/press;
  insert-text/type are unit-tested here, not a new live parity claim.
- `clippy-final.log`: release all-target strict Clippy. Backend proof is exactly
  **8** normal Pwright packages; contract proofs remain **3/6/7/10**. Dedicated
  `scripts/check-v2-keyboard-backend.mjs` now runs in CI separately from contracts.
- `input-proof.log` and `host-proof.log`: input-only example remains **5** Pwright
  packages; safe host remains **224** normal packages, without JS driver/CDP or
  process-launch imports. These proofs were rerun after the backend addition.
- `unit-initial.log` retains a test-construction compile error: `ResourceScope`
  is an enum, corrected to the explicit extension namespace constructor.

Reproduce the native comparison with the primitive's verified fixture/endpoint
and fresh oracle commands above, selecting `-p pwright-backend-cdp-keyboard`
instead of the input primitive crate. No Chrome restart or unrelated Page closure
is required. This task's preflight Page `F95903C741099A6C08FB5A4898489FA8` was
closed and its absence verified separately.

The Node addon was not rebuilt: SHA-256 remains
`8d973a18bf26623288efb0798f0b792a4345b206f7e6db8b9f3d2b868e53930b`.
No shared adapter change, new public Keyboard support, commit or push is implied.
Next: optional composition and Page routes with lifecycle/retirement budgets,
then Rust host/N-API and a thin Node facade, followed by public differential tests.

## Optional Page wire composition checkpoint

`pwright-composition-chromium-keyboard` now binds all five generated command
handlers to one explicitly identified Page. It requires the caller's existing
client and input coordinator. The single-Page `ChromiumKeyboardInstallation`
builds an inert production wire recipe; `ChromiumKeyboardPage::bind` also accepts
a shared backend factory, allowing multiple Page routes to share one operation
quota. No browser discovery, full Chromium composition, Locator, runtime,
executor or Node dependency is introduced.

### Route ownership and cancellation

- Typed commands validate Page identity **before** native operation admission.
  The wrapper calls the backend eagerly, preserving its admission-before-poll
  guarantee rather than storing an unbudgeted owned parameter until first poll.
- Route requirements account for the route allocation payload, Arc counters,
  inline native service handles and their shared binding payload. They exclude
  allocator bookkeeping, injected clients/input state and host baseline objects;
  this is an ownership budget, not a process-RSS measurement.
- The route lease lives inside the shared Page object, not just the installation.
  Escaped Page/handler Arcs retain it. Native services are deliberately not exposed
  through a clonable accessor that could outlive the route lease. The public state
  identity predicate does not expose ownership.
- Installation checks the complete concurrent backend + route + encoded-frame
  plan. Direct binding also checks backend + route together; checking only the
  smaller route requirement incorrectly allows a backend-only plan to pass the
  production ledger's subset check. Each actual reservation still enforces the
  shared global/per-scope limits.
- `revoke_admission` is irreversible: new and admitted-but-unpolled commands
  report `TargetClosed`. An operation already polled remains owned by the caller's
  controlled runtime. Revocation neither wakes nor aborts it, and does not release
  its operation lease early. The caller must settle/drop it and invoke the selected
  cleanup contract as appropriate; none of these actions invents native retirement.
- Cancellation checks Page identity but remains callable after revocation so
  default-exclusive key recovery is possible. Interleaved logical-key behavior
  is unchanged. Identity failure never performs cleanup on another Page.
- Call metadata is retained by the typed envelope but this inert leaf does not
  enforce deadlines or run a supervisor. The embedding runtime must enforce that
  policy and budget ingress decoding, queued results, transport and native work.

### Verified evidence and next integration

Evidence: `.work/node031-keyboard-composition.8uxKbj/`.

- `unit-final.log`: **11/11** tests cover wrong target, eager admission, escaped
  route lease, pre/post-admission revocation, ongoing-operation ownership,
  post-revocation cleanup, wrong-target cancellation, shared factory quota,
  incomplete production plans, and all five encoded command/result transactions
  through the generated chain into the real native backend with a fake CDP client.
- `regression.log`: **11** composition + **18** backend + **42** input + **103**
  Locator + **3/2/13/1** contract tests pass in release mode.
- `oracle.json` / `oracle.log`: fresh Playwright **1.58.2** on Chrome
  **152.0.7977.76**. `live.log`: **14/14**, **3.43 seconds**, using production
  encoding, generated dispatch, Page route and native services, then decoding
  replies and comparing full DOM events/value/error state. Each attempt confirms
  exact owned target/Context absence and zero remaining operation/route/wire
  leases. Live coverage remains down/up/press; insert/type are covered by the
  five-command offline wire test, not newly claimed live parity.
- `clippy.log`: strict all-target release Clippy. Composition proof is exactly
  **16** normal Pwright packages and is a separate CI check. Contract **3/6/7/10**,
  backend **8**, input example **5** and safe-host **224** proofs remain green.
- `unit-initial.log` preserves incorrect test use of the handler method/tuple
  event variant. `unit.log` preserves the production-plan test that found the
  missing concurrent backend-plus-route check; implementation was strengthened,
  not the assertion weakened.

Run the same fresh-oracle workflow with
`cargo test --release --locked -p pwright-composition-chromium-keyboard --test live_keyboard -- --ignored --nocapture`.
This task's preflight Page `05F26761E212D0D8732B5727934FE532` was closed and absence
verified. No Chrome restart or unrelated Page closure occurred.

At that checkpoint the aggregate `CdpPageServiceEntry` still named its input
`locator_input` and created it only from Locator. The following checkpoint removes
that selection dependency. Keyboard controlled command routes and native
completion/Page-Context-Worker teardown remain open, followed by host/N-API and
the thin Node wrapper.

The addon remains unchanged at SHA `8d973a18...e53930b`; shared **48/7/2**, reload
**16/18**, lifecycle **44/45** and screenshot attachment **0/12** are prior-artifact
results, not rerun or improved by this composition gate. No commit or push.

## Independent, retained-state-budgeted Page input

The input leaf now offers `CdpInputProductionProfile` and
`CdpInputServiceFactory`, separate from Locator and Keyboard commands. Its explicit
profile selects policy, scope and maximum simultaneously retained states. The
factory checks its own state count as well as the shared ledger before allocating
a coordinator. Exhausted ledger reservation rolls back the count. Zero capacity
is a deliberately disabled profile. The declared byte allowance covers the fixed
policy-specific state allocation, its Arc counters and the admission allocation;
allocator metadata and factory baseline storage are not a total-heap promise.

Admission is retained inside **every coordinator clone and input transaction**.
Cloning a coordinator into another service cannot escape its state budget; removing
the original Page route does not release an escaped state or active transaction.
State storage drops before admission, including the async `finish` path. Local
drop still does not prove native command retirement.

`ChromiumAttachProductionProfile::with_page_input` selects this resource independently
of Locator. When Locator is installed too, the explicit Page policy is its single
policy owner; legacy profiles with no independent Page input retain the old
Locator-selected policy and optional/unbudgeted construction behavior. The root
checks capacity against declared Page count and validates/deduplicates its explicit
scope. Retained old states and candidate replacements share the configured count;
callers need capacity for their intended overlap, not just visible Pages.

`CdpPageServiceEntry.page_input` and `CdpPageServices::page_input(PageId)` now carry
the shared owner. Frame IDs do not resolve as Pages, crashed Page access fails,
and a removed route returns no owner. Input state is reused across Frame/world
rebinding only within the same connection epoch, target/session, BrowserContext
and crash state. The full route cache now checks the epoch too: reusing native IDs
in a new connection epoch must not accidentally preserve old state. This is local
identity selection, not proof of native crash recovery or teardown completion.

Node host **source** now selects Interleaved through the Page resource at scope
`extension(120,17)`, with **32 retained states** against its 16-Page topology limit.
Its Locator no longer selects the policy independently. The addon has not been
rebuilt for this source change; prior artifact results cannot verify the new host.

Evidence directory: `.work/node031-page-input.uRolsR/`.

- `input.log`: **46/46**, including four new tests for last-clone ownership,
  transaction ownership under both policies, ledger rollback and zero capacity.
- `regression.log`: input **46**, Keyboard backend **18**, aggregate Chromium
  **294** (15 live tests ignored), Keyboard composition **11**, safe host **89**
  (5 live tests ignored). Seven new aggregate tests cover independent selection,
  shared Locator policy/state, main-frame native rebinding with held Shift,
  escaped owners after detach, disabled legacy selection, declared capacity,
  and reused native IDs in a fresh epoch. Ignored tests are not passes.
- `check.log` and `composition.log` retain initial compile errors: missing explicit
  core import/dependency, new exhaustive error classification and test imports.
  Production error categories were added explicitly rather than hidden by a wildcard.
- `clippy.log`: input, aggregate Chromium and host pass strict all-target release
  Clippy. Keyboard contract/backend/composition proofs remain **3/6/7/10**, **8**
  and **16**; input example **5**, Locator composition **18**, host **224**.
  The input leaf now declares its already-transitive core dependency directly.
- `host-live.log`: the new Rust host source connects to dedicated Chrome and
  closes successfully; metrics report zero active resource leases, operations
  and cleanup tasks afterward (**1/1**, 0.02 seconds). This is an attach/close
  resource gate, not a public Keyboard or full lifecycle-discrepancy result.
  Preflight Page `FB5881BC37ACCADB798C1395E0DE07DD` was closed separately and its
  absence verified; unrelated browser work was left alone.

Remaining integration: bind Keyboard routes to this owner, couple route publication
and revocation to the controlled runtime, then implement Rust host/N-API and the
Page-owned Node facade. Real Keyboard lifecycle/discrepancy gates remain required.

## NODE-031: optional controlled dispatch checkpoint

The optional composition now exposes a `runtime` feature implementing
`TargetedService` for all five command parameter types. Pass the **same bound
Page** as the borrowed operation handler and retained cancellation Arc to
`ControlledDispatcher::dispatch_with`. The existing runtime owns object-kind
validation, connection/target permits, pending calls, deadlines and abandoned
cleanup; the adapter creates no second runtime or executor. It checks the actual
target against its bound Page before eager backend admission and again at first
poll. A different live Page must not silently use this route's native session.

Down and up now have distinct `DownParams`/`UpParams`. Type-based dispatch could
not distinguish the previous shared parameter type. These are unpublished
Keyboard 1.0 schema type identities; method IDs, frame tags and key field bounds
are unchanged. Canonical schema, generated capability/models/server/reference,
client, codec and native callers were updated using the actual schema generator.
This is not a claim of backward compatibility with a published old schema.

Cancellation retains the exact original Page binding even after its owner drops
or admission is revoked; it does not look up a replacement Page by ID. A started
Exclusive press is cleaned up through the existing cancellation contract, while
Interleaved preserves the configured logical-key behavior on timeout. Revocation
alone still does not abort a started command. Route storage remains leased until
the last cleanup-held Arc is released. Native retirement, injected client storage
and host-owned arguments waiting for runtime permits require separate budgets.

Evidence: `.work/node031-keyboard-routing.oTrp98/`.

- `controlled-final.log`: **18/18** release tests (11 existing plus 7 runtime
  tests), covering distinct down/up dispatch, wrong live Page, wrong Frame kind,
  disposal of active work, queued revocation, abandoned original-route retention,
  and deadlines under both input policies.
- `controlled.log` and `controlled-rerun.log` preserve test-setup failures:
  the initial test catalog omitted declared parent kinds, and the Shift assertion
  incorrectly expected `keyDown` rather than Chromium's `rawKeyDown`. The
  fixture and assertion were corrected; no native behavior was changed for them.
- `clippy-final.log`: all six Keyboard crates, including the opt-in adapter and
  all test targets, pass strict release Clippy.
- `regression.log`: input **46**, native Keyboard **18**, Locator **103**, and
  contract layers **3/2/13/1** release unit tests pass on the current source.
- `proofs.log`: fresh generated outputs and contract closures **3/6/7/10**,
  native backend **8**, default composition **16**. Opting into runtime adds
  exactly `pwright-runtime` (**17** Pwright packages), no other package. CI
  checks both dependency selections and runs the runtime-feature release tests.
- `oracle.json`, `oracle.log`, `live.log`: a fresh pinned Playwright 1.58.2
  oracle and updated wire composition match **14/14** exact DOM event/value/error
  cases on Chrome 152.0.7977.76 (3.56 seconds). This verifies the regenerated
  wire/native path, not the controlled adapter against a real browser.

Next is **host integration**, not another runtime: atomically project the selected
Page's native client and admitted input owner, budget retained native binding and
pre-dispatch argument storage, and retain those leases with the exact cancellation
handler. Then expose the Rust host/N-API/Page facade and rerun artifact-specific
Keyboard lifecycle and shared discrepancy gates. No addon rebuild, new commit,
push, or public `page.keyboard` support is claimed by this checkpoint.

## NODE-031: public Rust-backed Keyboard checkpoint

`Page.keyboard` is a stable facade exposing `down`, `up`, `press`,
`insertText` and `type`. JS stores the native session and portable Page ID,
checks public options and delegates; it owns no key state, CDP, focus, retry,
timer or driver process. N-API admits conversion/settlement before copying
bounded strings and calls five Rust host methods. Fractional delay is preserved
as Duration. Keyboard adds no implicit Locator focus or navigation wait.

`CdpPageServices::page_input_binding` projects the client and admitted input
from one immutable Page entry. It neither resolves through a Frame nor retains
all of the old entry's unrelated services and strings. An added root test proves
the original client/state survive detachment. Aggregate tests pass **295**
(15 ignored); input state still owns its independent lease.

Host scope `extension(120,18)` owns 4096 state bytes and **16 retained
invocations**. Before argument copy, route projection or queuing, each reserves
argument bytes plus 8192 call/native-binding bytes. This includes the concrete
host CdpSession, Arc controls and two native IDs bounded to 256 bytes by the
host topology profile; connection/input keep their own budgets. Each invocation
also owns a composition route lease. One factory at `extension(120,19)` admits
**8 executing operations**. Queued calls and abandoned cleanup share the retained
count. The exact cancellation Arc owns the invocation lease, keeping native
client storage admitted after the outer future drops. N-API separately reserves
6 MiB plus 32 KiB for overlapping UTF-16/UTF-8 conversion and settlement.

The first public run was **21/24**: three failures exposed discarded unknown-key
tokens and a generic Backend prefix. `KeyInputError<'a>::UnknownKey(&'a str)`
now borrows the actual failed token with no input-layer allocation. The bounded
backend owns its diagnostic and the host preserves `Unknown key: "…"`.
Partial-prefix state was not changed. The final public gate compares **exact
error messages**, not merely a loose unknown-key classification.

Evidence: `.work/node031-keyboard-routing.oTrp98/`.

- `public-keyboard.json/.log` preserves the first **21/24** result on
  `690303ab...cfb5329`; the three diagnostic failures were not hidden.
- Final addon SHA-256:
  `1319fa027f4bd5ee0d3e2f3932dce1c01ad5b8de86381b6817970ee0ded25dc1`.
  `addon-build-final.log` records its release build. No new commit or push.
- `public-keyboard-final.json/.log`: **24/24** against Playwright 1.58.2 on
  Chrome 152.0.7977.76. Fourteen existing raw cases, five text/empty/fractional
  delay cases and five integration/lifecycle cases use the same public scenarios
  for both providers. Candidate disconnect leaves zero leases/operations/cleanup.
- `shared/report.json`: unchanged **57**-case corpus is **52 match / 3
  unsupported / 2 oracle-error**, with zero other errors. Four Keyboard cases
  moved from unsupported to match via public `page.keyboard`, not CDP/JS
  fallbacks. The supervisor exits 1 because full compatibility is still false.
- `host-tests.log`: root **295** and initial host **94**; final regression:
  input **46**, native Keyboard **18**, runtime-feature composition **18**,
  host **95** (5 ignored). Six new host tests cover retained ownership/count,
  argument bounds, rollback, declared native overhead and exact error mapping.
- `final-clippy.log`: strict all-target release checks pass for host/addon/
  input/backend/composition. Node contracts **31**, shared verifier **30**, both
  TypeScript checks and dependency checks pass. Host closure grows from **224
  to 229** normal packages, adding Keyboard capability/backend/codec/server/
  composition. Leaf default/runtime closures remain **16/17**.

Run `pnpm --dir packages/pwright test:keyboard` with explicit
`PWRIGHT_TEST_CDP`, verified `PWRIGHT_INPUT_FIXTURE` and an absolute unused
`PWRIGHT_KEYBOARD_REPORT` file. Use serve-lib for the fixture and the dedicated
Chrome/Pwright preflight. The runner records owned Context cleanup and rejects
diagnostic errors rather than counting both sides failing as a match.

This is not full Playwright conformance: arbitrary Unicode/UTF-16, keyboard
layouts/interleavings, native-retirement/Worker shutdown and remaining public
APIs need more coverage. Bounds remain 128 key bytes, 1 MiB text and 16 pressed
physical codes; unpaired UTF-16 explicitly rejects. Prior full reload
arrival-close, window geometry and screenshot attachment gates were not rerun
on this artifact; ordinary shared screenshots do not replace those full gates.

## Same-artifact post-Keyboard regression

The subsequent run **does** execute the full input/action/screenshot/reload/
lifecycle lanes on the unchanged `1319fa02...ded25dc1` release addon.
Evidence: `.work/node031-regression.iGmeff/`.

The typing runner previously filtered out every catalog entry with a held
modifier because raw Keyboard was absent. It now runs **all entries**, setting
and releasing Shift/Control/Alt/Meta through the public Keyboard API. Both
`type` and `pressSequentially` exercise those four entries, restoring eight
comparisons without changing the catalog, fixture, assertions or API deadlines.

| Full lane | Result | Duration | Evidence |
| --- | --- | --- | --- |
| Sequential typing, including held modifiers | 38/38 | 29.782 s | `typing.log` |
| Locator actions | 27/27 | 25.034 s | `actions.log` |
| Screenshot pixels, ownership and Worker cleanup | 12/12 | 12.679 s | `screenshot.log` |
| Reload | 16/18 | 28.333 s | `reload.log` |
| Lifecycle, defaults, publication and Worker teardown | 45/45 | 68.321 s | `differential.log` |

All runners exit normally, without supervisor kills, cancelled tests or skips;
only reload exits nonzero. Its two failures remain unchanged: the oracle's
response-arrival Page close exceeds the **10-second diagnostic** bound, while
Rust reports the exact native target still present after Target.closeTarget.
The independent post-commit close case passes. The original 1-second reload
timeout, bounded native absence checks and fixture response boundary were not
relaxed or replaced.

Offline Node contracts **31/31**, TypeScript and the **229-package** safe-host
boundary also pass (`unit.log`, `types.log`, `boundary.log`). The addon hash is
unchanged; no build or commit was needed for restoring test coverage. The owned
preflight target `B4636A3B473FA9A7B389877223E71D79` was closed separately and its
absence verified, leaving unrelated game Pages untouched.

The previous screenshot attach failures and NODE-014 zero-window snapshot did
not reproduce in this run. Their causes remain unresolved; this green artifact
run is **not a demonstrated fix for either intermittent issue**. Public Keyboard
24/24 and shared 52/3/2 evidence from the preceding checkpoint remain valid for
the same addon, but neither substitutes for the full lanes listed here.

The subsequent [bounded reload diagnostic](pwright-page-reload.md#causal-native-close-diagnostic)
now correlates request/commit events and reproduces an acknowledged-but-still-open
native target while bypassing host Page close. It changes only test code, not
this addon or the original red gate. Native command acknowledgement must not be
treated as retirement; a lifecycle fix must preserve cancellation cleanup and
rollback without introducing an unconditional navigation dependency into Page
close. StopLoading was timing-sensitive in the controls and is not adopted.
