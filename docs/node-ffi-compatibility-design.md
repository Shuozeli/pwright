<!-- agent-updated: 2026-10-12T07:11:38Z -->

## Typed `*` capture selector checkpoint (2026-10-12)

The Rust query planner now preserves one CSS/text capture point for Page
`$eval` and `$$eval`. It retains separate prefix and suffix trees until the
terminal command; later filters and descendant Locator composition modify
the suffix, then the existing typed `FilterHas` selector returns each
matching captured ancestor once in DOM order. An initial eager rewrite
applied later filters to the wrong element and was corrected after source
review. Multiple captures and capture with `nth` retain Playwright-shaped
errors; unsupported custom-engine captures fail closed. The selected release
host source and test compilation pass with handles and selector registration
enabled; the selector module passes 89/89 release tests. New native packaging and official
browser comparisons remain pending.

## Browser-root CDP session checkpoint (2026-10-12)

The opt-in `browser-cdp-session` feature composes with, but remains separate
from, Page/main-Frame `cdp-session`. Rust attaches each public Browser session
at the connection root, distinguishes its lifetime from Page/Context owners,
and shares one eight-session native capacity and bounded event lease path.
Both Page-creation entry points capture a discovery-enabled Browser-session
fence before native creation; the host waits for the exact new target ID, and
the Node facade publishes queued target events before returning the Page.
The fence handles omitted and explicit first-match-wins CDP discovery filters
that include Page targets. If the post-create event flush fails, the facade
closes the newly created Context or Page before rejecting, preserving both
the primary and cleanup errors if compensation also fails.
Retirement, discovery disable/re-enable, queue overflow, and an expired event
history terminate this fence without silently treating an unrelated event as
success. The post-edit single-job release addon check and 32/32 focused CDP
host tests pass. The separate topology-driver test target remains unverified:
its release compilation raised swap by over 600 MiB and was stopped; a
test-only ownership compile error found there is corrected but not rechecked.
A new native artifact and unchanged official browser tests are still
required; the old combined artifact does not attest this feature.
The normal browserless Node suite passes 773/773, including four cleanup
adapter tests, with strict TypeScript. Both CDP leaf release checks and
zero-added-package dependency proof pass on the current source; a combined
native artifact still must be rebuilt. Selected release host source and test
compilation pass; topology-driver test execution remains open.

The CDP-session Node conversion now uses the crate-wide `napi` facade, and its
two opt-in raw classes are conditionally listed in the module-local TypeTag
registry. The native-class source audit, full `pnpm test:boundary` gate, and
single-job `cdp-session` addon release check pass; the selected release
module-local class-tag proof passes 1/1. This boundary correction does not
add official browser-test or conformance-ledger credit.

## Retained ElementHandle selectOption value slice (2026-10-12)

The separate opt-in `element-handle-select-option-value` leaf adds
`ElementHandle.selectOption('blue')` with a string value or label to the
original retained Handle. The Rust host owns the timeout/retry, bounded DOM
selection and event dispatch; Node only validates a small argument shape and
forwards the Handle. Descriptor and Option-Handle overloads, `force`, and
navigation-commit semantics are explicitly not implemented. The isolated
feature depends only on `handles` and `timeout-settings`, not Locator DOM
actions or geometry. Focused Rust release tests pass 5/5; Node browserless
units pass 738/738, strict types, dependency boundary and Clippy pass. The
cumulative `official-conformance-page-retained-select-option-value` release
addon has SHA-256
`bae00851f127108b003264f46a4ea8982a9ee4ad1b3de1ab3b70ef2335faaa06`
and independently passed native-load smoke. The pinned official
`elementhandle-misc.spec.ts` file has not been rerun, so its ledger remains
unchanged.

## Playwright CSS pseudo-selector source checkpoint (2026-10-12)

The Rust Locator query parser now recognizes direct and `css=` `:visible`,
one-based `:nth-match` across bounded CSS arms, and the root
`:is(:nth-match(...), :nth-match(...))` composition needed by the pinned
Core `selectors-misc.spec.ts` cases. It composes existing typed visible,
DOM-order union, and nth selectors; quoted, escaped, bracketed, and nested
argument separators do not split arms. The complete shape is preflighted and
reserved before constructing the retained tree. Unsupported nested Playwright
CSS extensions fail closed instead of being silently delegated to native CSS.
Ten new AAA tests bring the focused release host lane to 63/63; strict release
Clippy passes. This source-only checkpoint does not claim that the official
`:visible` or `:nth-match` browser cases pass, because the dedicated CDP
endpoint remains unavailable. The unchanged official ledger has no new credit.

A pinned-upstream source audit then found and repaired three parser edges:
case-insensitive pseudo names, integer-valued CSS number spellings such as
`1.0` and `1e0`, and pseudo-looking text inside CSS comments. The expanded
focused release host lane passes 24/24 and strict Clippy. The selector runtime
also now traverses nested Shadow Roots in upstream CSS order and sorts typed
unions/within results in composed DOM order; its injection version is 14 so an
older script is replaced. Runtime release tests pass 12/12, strict Clippy
passes, and a fake-DOM dynamic check covers nested/sibling Shadow Roots. These
remain browserless checks, not official conformance results.

## Page/Frame waitForSelector retained wait (2026-10-11)

The opt-in `handles` Node addon now exposes `NativeLocator.waitForSelectorElement`
backed by Rust `HostLocator::wait_for_element`. Its native-artifact proof
requires the new method, so an older `handles` binary fails closed.
`Page.waitForSelector` and `Frame.waitForSelector` map Playwright's four states,
default non-strict first match, strict selection, and Page/Context inherited
timeout to one Rust-owned wait and optional retained `ElementHandle`
publication. Typed and Browser-program selectors, including XPath, now poll
one state-and-element browser result per attempt; the positive-state Handle is
the node actually observed. Hidden/detached completion returns `null`.
The Node boundary validates options and retains the upstream legacy
`waitFor: 'visible'` alias. This is an implementation and browserless-test
checkpoint only; it has not rerun the pinned official Chromium
`page-wait-for-selector-1` file. Registered/Layout selector plans and complete
upstream timeout call logs remain open. The language wrapper matches the invalid-state
diagnostic and adds the selector/state call-log header on typed native wait
failures; full per-attempt element previews are still absent.
When a Frame is confirmed detached, the wrapper also maps the Rust-recognized
CDP execution-context-loss variants (including context-ID loss and inspected
target closure) to `frame.waitForSelector: Frame was detached`. It preserves
the original backend error for an active/unknown Frame and for Page calls.
This source-only adapter check passes 27/27 focused and 735/735 full Node
browserless tests; the pinned official detached-Frame case has not been rerun.

The separate `element-handle-wait-for-selector` leaf now evaluates CSS/XPath
descendants against the original retained Handle and returns the matched node
from the same state-observation snapshot. It never re-queries from Page/Frame
or follows a replacement execution world after navigation. Omitted timeout
uses the creation world's Page/Context setting; explicit zero is unlimited.
The language wrapper adds the locator call-log header on typed failures.
The `official-conformance-page-retained-wait` profile composes this leaf with
the earlier Page and scroll capabilities. Its release addon SHA-256 is
`905b9b7dd8bc171e0aab2d401899730da640854d7f10d22084d6ac6dc676c533`;
native-load, 735/735 Node package units, strict types, and dependency boundaries
pass. The environment now shares one immediate-fail quota of 24 selector waits
across Sessions, leaving eight of 32 FFI Command slots for unblockers;
focused release tests and strict full-profile Clippy pass, while
browser-backed concurrency proof is pending. The four pinned Core
`ElementHandle.waitForSelector` failures retain their ledger outcomes until an
official browser rerun; broader selector engines and preview-history parity
remain open.

## Retained ElementHandle scroll leaf (2026-10-11)

`element-handle-scroll-into-view` is an independent opt-in Node addon leaf:
it depends on `handles` and the matching Rust host leaf, not on
`element-handle-dom` or Locator DOM actions. `ElementHandle.scrollIntoViewIfNeeded`
forwards the exact retained native Handle to Rust. An omitted timeout now
resolves through the Rust-owned Page/Context default; explicit zero remains
zero. The native artifact checks its marker and N-API method together; the
unselected facade rejects the call. The isolated
`official-conformance-page-retained-scroll` profile adds
this leaf to the pinned Core Page feature union without altering the earlier
Page profile. At that checkpoint browserless Node units passed 710/710. The isolated
release addon with inherited timeout and a stale-binary guard has SHA-256
`60d757a38a958732aff254e87bf92e9d1af2e7be34b301f795c07f312b338d3c`;
its native-load smoke passes. The cumulative full-Page profile now also builds
and passes native-load smoke (SHA-256
`5122f41c5e7d93672662367b8b6ed9a4e65e778ba0db1d9230cc34459d41d405`),
710/710 browserless package units, strict types, and dependency boundaries.
Its official browser behavior remains unverified.

## Retained ElementHandle focus and Text-node click (2026-10-11)

The selected `element-handle-dom` Node addon now exposes the host's existing
exact-retained-Handle focus command. `ElementHandle.focus()` only forwards its
native owner; the facade rejects options until timeout/no-wait semantics are
implemented and fails explicitly when the profile is absent. No selector or
second owner is constructed. A separate retained-click backend repair
retargets a Text node to its current parent Element within actionability and
both pre-input route checks while keeping each CDP call bound to the original
Text-node object ID. It checks connectivity and current Frame identity on
every phase without consulting the mutable page-global `Node`. The Node package
passes 693/693 browserless tests, strict types and selected-addon release
check / Clippy; the fake-CDP locator release suite passes 146/146. A rebuilt
native addon and unchanged
official Core/Python browser reruns remain required for conformance credit.

## Opt-in official URL expectation bridge (2026-10-10)

The pinned complete 64-case Core Page.goto rerun passes 61/64 candidate
versus 63/64 independent oracle expected outcomes. One candidate-only failure
is `toHaveURL` after a JavaScript redirect: upstream reads
`page.context()._options.baseURL`, then invokes
`page.mainFrame()._expect('to.have.url')`. The opt-in `wait-for-url` Node
profile now supplies that Context field and a narrow Frame expectation port.
JavaScript preserves serialized string/RegExp matcher semantics while Rust's
`frameWaitForUrl(..., 'commit', timeout)` retains observation, lifecycle,
admission and timeout ownership. The selected Page exposes the official
matcher identity, accepts the matcher's optional `signal`/`title` fields
(a live cancellation signal fails closed), and returns the official
`received: { value: url }` shape. Unsupported expressions fail closed, and
the unselected default profile retains its v1 shape. Twelve focused AAA
tests, 587 package tests, strict types and dependency boundaries pass. This source
change postdates that earlier paired official report. A fresh complete 64-case
candidate/oracle rerun now gives :492 official candidate pass evidence, but
candidate :217 unexpectedly fails with a network Request Frame/topology
invariant error; one later isolated candidate :217 diagnostic and a second
complete 64-case candidate rerun both pass :217. That second run is 62/64
candidate versus 63/64 oracle expected outcomes, and the new immutable Core
strict ledger promotes exactly :492 to expected. The intermittency/sequence
dependence of :217 remains unresolved. The other candidate-only Page.goto
failure, `:859`, still needs exact Rust Frame-detach publication, not a JS
alias. See `docs/upstream-playwright-conformance.md` for the reports.

## Opt-in exact Frame-detach event source checkpoint

The pinned Page.goto :859 failure waits for
`page.waitForEvent('framedetached')` before awaiting navigation. A new opt-in
`frame-detach-events` source path carries exact CDP `FrameRemoved` effects
through bounded host publication, N-API metadata, and the Node Page emitter.
It publishes tree retirement before the event and preserves the canonical
Frame wrapper, including child-first unobserved descendants. Each live child
pre-admits a detach notice, worst-case tree revision/queue slot, and one-shot
Frame binding; the selected capacity adds 127 binding states and 254
revision/publication slots, while the unselected default remains unchanged.
A later tree revision now retains an unconsumed retired identity until the
event-loop completion port releases its unused binding. Focused release
`frame_tree` and `frame_trees` suites pass 33/33 and 28/28; Node package tests
pass 592/592 after a JS cleanup test, strict types and dependency boundaries pass, host strict release
Clippy passes, and default/selected addon release checks compile (both still
show an unrelated existing `prepare_input` dead-code warning).

This remains **source-only and incomplete**. Initial child admission still
occurs after the topology transaction commits, so exhaustion can leave a
committed child without a publishable Frame; a transactional precommit
ownership reservation is required. Reconnect cleanup now captures exact
`FrameRemoved` effects for retained-Page children and publishes them in
child-first order; feature-on topology release tests pass 53/53 and default
tests pass 50/50. JS now calls the completion port even if detached-wrapper
materialization throws, preserving the original exception. Partial ancestor
cache rollback remains unimplemented. The selected full-profile addon has not been rebuilt or native-loaded,
and unchanged official :859 has not been rerun. Do not claim general
`framedetached` support or strict-ledger credit from the local tests.

The next repair must prepare child ownership *before* runtime/routes/topology
commit, not merely preallocate a `FrameNode` after it. A selected-only RAII
plan should derive bounded child additions from the candidate topology,
prevalidate identity and reserve URL entries, detach lifecycle/tree/binding,
and each affected Page's revision/queue slot. Any failed reservation drops the
uncommitted candidate; successful commit consumes the prepared objects with no
further fallible admission. `context_storage_state::Observer` must forward
the new preparation hook. Focused tests need to exhaust each resource in turn,
exercise Nth-child rollback and route-preparation failure, then prove successful
attach→navigate→detach identity and event ordering. The CDP event cursor also
needs explicit retry/resync semantics when precommit preparation rejects an
already-read event. This is a reviewed design, not an implemented guarantee.

## Page.goto main-Frame routing source checkpoint (2026-10-10)

When `frame-navigation` is selected, the Node `Page.goto` facade now sends
the resolved URL and effective Page navigation timeout to the existing Rust
`frameGoto` port for its main Frame. Without that optional native method it
retains the v1 `pageGoto` route. This mirrors upstream Playwright's Page-to-
main-Frame delegation without moving navigation logic into JavaScript. The
focused adapter passes 30/30; package tests pass 575/575, and strict
TypeScript and dependency-boundary checks pass. The change postdates all
recorded Core Page.goto browser reports and native artifacts. No official
browser outcome or strict ledger row changes until a freshly built addon runs
the unchanged upstream cases in both lanes.

## Official Page.goto burst artifact (2026-10-10)

The explicitly selected `official-conformance-page-goto-burst-20` artifact
composes the existing official Core Page feature union with only
`frame-navigation-burst-20`. Node's fixed-path loader checks the compiled
burst marker alongside the official capability markers and retained
`ElementHandle.dispatchEvent` method. This adds no JS browser operation or
Cargo feature definition; the default, prior official, and standalone burst
profiles retain their separate paths and feature sets. The release addon
SHA-256 is
`3dfb2ac8698de70f5b2416688b2d15de879251d9d7bf5c5549474ecb9fbd5c83`.
Browserless Node units pass 565/565, strict types and dependency boundaries
pass, and all three selected artifact native-load probes pass 1/1 each. The
unchanged pinned 64-case official Page.goto file has not run against this
artifact.

## Opt-in trace-directory owned launch (2026-10-10)

`PWRIGHT_NATIVE_PROFILE=playwright-trace` adds a narrow Node-to-N-API-to-Rust
port for Chromium `launch({ executablePath, tracesDir })`. A nonempty string
is resolved against the call's cwd, recursively prepared before executable
validation or temporary-profile allocation, and retained as a caller-owned
directory in the Rust host. An empty string follows upstream's falsey path
behavior. Default and legacy `owned-launch` profiles remain unchanged. The
selected addon SHA-256 is
`13d6db323b599d57608d99b7f18f8a06856c6425e1931dfdb718f74d356c765d`;
four browserless native tests, 533 Node units, native-load, types and boundary
checks pass locally. The exact Core dummy-Node line-64 diagnostic passes 1/1
for both oracle and candidate, but its process guard does not attest browser
profile ownership and no strict-ledger row is promoted. The Rust v8 JSONL
writer is independent and not yet wired to Context tracing; `.network`, ZIP,
snapshots and body capture remain absent. The CI selected-profile gate has
not run on GitHub Actions; its build clears the repository's global
`RUSTFLAGS=-D warnings` because nine existing host/addon dead-code warnings
remain in this profile.

## Owned-launch fixture option boundary (2026-10-10)

The Node launch adapter now merges Playwright worker defaults only after the
explicit `userDataDir` and `port` assertions. It accepts the pinned fixture's
`handleSIGINT: false` after boolean validation because this profile does not
install a SIGINT handler; `true` and non-boolean values still fail before
native acquisition. At this earlier checkpoint, an exact official Core
dummy-process diagnostic advanced past this check but stopped at inherited
`tracesDir`, which was unsupported.
It must not be silently ignored: upstream creates its directory and carries
the path into Context tracing. A separate opt-in tracing capability is needed
before positive support. Node package tests pass 529/529 and strict types pass;
the official candidate line-64 case remained diagnostic-only and `not-run` in
strict accounting.
The selected Rust owned-launch composition now captures bounded startup
stdout/stderr and passes a `Browser logs:` failure through HostError, while
the default provider remains noncapturing. The rebuilt opt-in owned-launch
addon passes a browserless public Node-to-N-API-to-Rust synthetic-process smoke
test (1/1) with bounded logs and cleanup. CI now builds and tests this profile
on Node 20, though GitHub Actions has not run the new step. The original
`owned-launch` profile still rejects the inherited `tracesDir`.

## Opt-in native persistent-Context preflight (2026-10-10)

The `persistent-context-preflight` Node profile proves two dedicated N-API
ports: `persistentContextCertificateError` and
`persistentContextArgumentError`. Its marker and both exports are checked
fail-closed against the selected artifact; no default, browser-registry or
owned-launch profile acquires this Rust leaf. Certificate materialization
stays in the JS client: it completes all `Promise.all` file reads, converts
material to JS-truthiness booleans (including empty `Buffer` as truthy), then
calls the Rust certificate decision. Only after a successful certificate
decision does JS unwrap boxed strings and call the Rust engine-argument
decision. JS throws the exact public `Error` prefix. Valid options still end
at the unsupported persistent-launch boundary.
For the selected Rust profile, pinned classic Firefox rejects `-juggler*`
after profile-argument misuse; `moz-*` BiDi Firefox does not apply that
classic-only check and continues to reject page arguments. A truthy non-array
`ignoreDefaultArgs` bypasses default-argument validation, but `[]` does not.
The default JS profile remains unchanged, including its separate classic
Firefox `-juggler*` discrepancy.

Release artifact SHA-256 is
`e1288f11e26c9d54aadb6f35f985a5c444fb4154539bada11a358cef18677ca3`.
The pure Rust leaf passes 28/28 release units and strict Clippy; the selected
addon release build and native-load pass. Default and selected Node
profiles each pass 512/512 browserless units, the real native bridge 4/4,
and the selected focused suite 23 passed / one intentional skip. The pinned
Playwright Core v1.58.0 guarded, browserless public-method comparison covers
16 rejection and three accepted-preflight inputs. Strict TypeScript,
formatting and exact dependency closure checks
pass. These synthetic tests do not promote an unchanged official corpus row,
prove positive TLS parsing, or demonstrate owned browser launch. The default
focused suite passes 15 with nine intentional opt-in skips; its classic
Firefox discrepancy remains open.
The Node 20 CI job now builds and validates this opt-in profile separately.
That job does not set the local pinned-Core oracle path, so its public-method
oracle comparisons remain skipped in CI; the guarded comparison above is
reproducible locally against the pinned checkout.
The added boxed-empty certificate cases initially failed in both Node paths:
raw object truthiness bypassed the official origin/material errors. Both paths
now unwrap protocol strings before the Rust or JS truthiness decision. The
real native test script selects its profile to prevent an all-skipped pass.

## Response waiter and terminal body readiness (NODE-432, 2026-10-07)

The fixed manifest selects all nine `chromium-page` invocations from the
unchanged `page-wait-for-response.spec.ts` file with no title filter. Candidate
v1 passed 1/9. Oracle v1 and a clean-Chrome v2 both recorded 8/9 because Chrome
154 does not emit the second request expected by the cached-image case.
Candidate v2 reached 7/9: its remaining candidate-only failure was an async
predicate whose `response.text()` raced CDP completion and received
`No resource with given identifier found`.

The repair preserves separate portable and browser-native identities in each
Response notice. `response-events` still publishes through the existing fixed
256-item Rust queue. The separately selected `response-finished` feature
attaches an opaque bounded Rust owner, and both `Response.finished()` and body
readiness share one cached native wait. Only `response-body` uses the native
CDP request identity after that wait. Node remains responsible for synchronous
listener arming and language predicates; it does not subscribe to CDP, infer
responses from request events or retain another response registry.

Minimal release artifacts prove the feature boundary. `response-events`
(`8ac46030c57f5429608ded9202a6320213e8d4ab9372d283a91a872dd79d49d1`)
has no response-finished marker, completion method or body capability.
`response-finished`
(`c744980497c6533bb6195af0a32451c14e724b230721452665db4038357a815a`)
adds completion but still omits body. The full official artifact is
`30bb029c4d89f3f179b886d226d9e761ffa6bed01724e92afdeef149760db853`.

Final candidate v3 and paired oracle v3 have identical normalized results:
eight expected outcomes and the same cached-image unexpected result. Candidate
report/provenance SHA-256 values are
`1184ec4468625640f5813a40ce810bafc64d618999ad17fe164eeb7b7f45229d`
and `98c3ef41c3f33db6bc27cafe443e0637cc6e9fab44a84779cc0a6b17b8469dd2`;
oracle values are
`3ae728c3616190e864b8a71cf4d8402f9961e34082ca9ceefb8fb000fe2cba59`
and `562462ea3359330ce0458c4e02d0888aee8528e710ca077be6c32a9202066fd7`.
Manifest SHA-256 is
`e68c6447de393e52b6ddbbf7bf2e0cd4387bf4c3b837ebb352e76c476a3ff596`.
Candidate source SHA-256 is
`07c4c8edebe84a65532f25ac9fee233eddde2eb110cc748812bf57bf44cd11ae`.

Exact nine-row merges reach candidate 1,542 expected / one skipped / four
unexpected / 46,930 not run
(`7bfaf60461d835f5f7e87a197801ef9e3c72aed23adcc1cfdccfe4448245de7a`)
and oracle 1,245 expected / three unexpected / 47,229 not run
(`ae0fc98c9a2582ac39d88c44c59efcf7094d2bf29f792b63266f32ec19ff0322`).
Both 48,477-row audits retain classification hash
`32520af75e4ac14352a9060e6f7156900bb52c4a3d99fe927311ba7490328af0`.
Their summary SHA-256 values are
`1b45e84c5246f7cb309141301d1d2ddc5446fdacc68c42b3485e219eefe77aaa`
and `4d5b20e06caf7bf6288602337f14815a4af864675962b9a72d288b9ed8a8cfc3`;
the normalized nine-row projection is identical at
`d1ac62096c1eeab0ea8f16d078975c829c6601db8b2168f68acc35be22401aa1`.
Package 407/407, strict types, boundaries, native loads, exact official host
850 passed / 29 explicit ignores, strict addon Clippy, formatting and diff
checks pass. The complete Core and Python corpora remain open.

## Page request waiter parity (NODE-431, 2026-10-07)

The complete unchanged Chromium Page `page-wait-for-request.spec.ts` file has
eight public-API cases and no title filter. Oracle v1 passed 8/8. Preserved
candidate v1 passed 4/8 and exposed four independent gaps: explicit timeout
wording, ignored Page-default timeout, absent long-matcher call log, and loss
of the upstream `window.builtins` fixture after document replacement. The last
item was a candidate-runner input defect, not request-wait behavior: upstream
installs its UtilityScript builtins in every execution context, while the
preload had installed them only in the initial document.

The runtime repair shares one synchronous EventEmitter waiter between Page
`waitForEvent()` and `waitForRequest()`. It preserves public predicate and
cross-realm RegExp evaluation in JavaScript, captures the API call stack before
returning, reads the Rust-owned effective Page default timeout, emits the
quoted event `TimeoutError`, caps matcher logs at 50 Unicode code points, and
retires listeners and timers on resolve, rejection, timeout and observable
Page close. Rust remains authoritative for Context subscription activation,
request observation, bounded queue admission, lifecycle state, default-timeout
state, base-URL resolution and URL-glob compilation. No second observer or
native per-wait task exists.

The official candidate preload now installs only the pinned upstream builtins
fixture through a Context init script so every document receives the same test
input. This is harness reproduction and is not candidate runtime behavior.
Candidate v2 passed 7/8 and isolated an off-by-one log preview; Playwright's
50-code-point cap includes the ellipsis. Final candidate v3 and paired oracle
v2 pass 8/8. Report SHA-256 values are
`e2d90b8701e5d7a9ff8c126165e8d632101e422339c32bc7addd63641789e5d1`
and `b7a837a697d6670d4bf3aaa71366eee8beac3586cea4f05ddf25915d3535df0`;
verified provenance sidecars hash to
`5b3b143c23be31643a821a4e663256eb92322b683d10727a45adf00724a8075a`
and `7ae11e60cddd36f07b7058725a4bde63229d5ac2de7acedfc564dd05434264af`.
Manifest `b5936ea...cdcd`, artifact `fc00727...54790` and candidate source
`76aa0e9...d0b8c` are recorded by provenance.

Strict eight-row merges reach candidate 1,534 expected / one skipped / three
unexpected / 46,939 not run (`c19a233...7abc`) and oracle 1,237 expected / two
unexpected / 47,238 not run (`4030917...5241`). Both changed-row sets normalize
to `b44d380...f892`; both 48,477-row audits retain classification hash
`32520af...8af0`. Package 393/393, strict types, boundary checks, three native
load profiles, minimal feature checks, exact official host 843 passed / 29
explicit ignores, strict addon Clippy, formatting and diff checks pass. The
complete Core and Python corpora remain open.

## Bounded Page request waiting and post-data decoding (NODE-430, 2026-10-06)

`page.waitForRequest()` reuses the existing Rust `request-events` installation
instead of creating another request registry or one native waiter per call.
The host activates request observation before publishing a new Context, admits
request, request-finished and request-failed snapshots into a fixed 256-entry,
16 MiB queue, and exposes one bounded N-API watch. The Node Browser consumes
that stream sequentially and publishes the shared `Request` facade to the
exact Page. Terminal notices delete the short-lived wrapper cache.

The public waiter installs its Page EventEmitter listener synchronously before
returning its Promise. String matching delegates glob compilation and base-URL
resolution to the existing metadata-only Rust owner; JavaScript performs only
the final RegExp or public predicate call. Timeouts reuse the Page's Rust-owned
default timeout and retire only the language listener. This preserves
Playwright's Promise-all arming behavior without introducing an asynchronous
native arm race.

Post-data decoding is a second optional leaf. Rust parses JSON or URL-encoded
form bytes and preserves the official invalid-JSON diagnostic before N-API
materializes the value. The base request-event snapshot contains raw bytes but
does not link `serde_json`. Building `request-events` and
`request-post-data-json` separately exposed two N-API object-shape bugs hidden
by the full feature union; the request snapshot now uses mutually exclusive
complete object definitions, and CI compiles both minimal unions explicitly.
AAA tests cover synchronous arming, skipped and asynchronous predicates,
predicate failure, timeout cleanup, stable Request identity, terminal cache
retirement, Rust parser behavior and foreign-copy accounting.

The first official post-implementation run passed 5/6 and exposed one final
binding ambiguity: N-API omits an `Option` field both for a parsed null and for
an unavailable capability. Request snapshots now include an explicit
parser-selected boolean only in the parser-enabled object shape. Candidate v3
and oracle v3 then record the same 6/6 outcomes; the two Chromium
expected-failure attempts both reach their real file/blob and `sendBeacon`
assertions with null post data. Verified report hashes are
`3349ede...67955` and `7452f56...8e2c`; exact six-row merges produce candidate
`8e8e6b7...be91` and oracle `55be77a...57aa`, while both 48,477-row audits
remain `32520af...8af0`. Full-corpus conformance remains open.

## Navigation Response request snapshots (NODE-429, 2026-10-06)

`Response.request()` is an independently selectable composition, not part of
header mutation. The addon `response-request` feature composes
`frame-navigation` with the host's existing bounded `request-history` owner.
When Page, Frame, reload or history navigation receives a native request ID,
the Rust host resolves that ID to the immutable captured `NetworkRequestRecord`
and retains its shared owner beside `ResponseMetadata`. It never rebuilds the
sent request from Page or Context header settings.

Before N-API clones URL, method, resource type, headers, post data, navigation
state and Frame identity, the host computes and reserves a conservative
foreign-copy allowance from that exact retained record. The Node layer then
wraps the snapshot through the same public `Request` facade used by route and
network APIs. `Request.headers()` exposes lowercase keys as Playwright does,
while the retained header array remains the source of truth. Profiles without
the leaf report an explicit unsupported operation instead of fabricating a
partial request.

The two complete unchanged Chromium files record identical candidate/oracle
outcomes: Context is two expected; Page is four expected plus one shared
unexpected pass because Chrome 154 now satisfies a Referer assertion still
annotated with upstream `test.fail`. This replaces the old candidate's masked
`TypeError: response.request is not a function` with the real assertion result.
Artifact `4d06472...9f17`, source `669a396...77b5`, manifest
`eb26ee2...d8f` and all four report/provenance pairs verify. Exact ledger diffs
touch only the seven intended keys; full-corpus conformance remains open.

## Composable Touchscreen and tap actions (NODE-428, 2026-10-06)

Raw coordinate touch input and selector-driven tap actions are two independent
Rust selections. `touchscreen-input` owns Page-scoped input serialization,
modifier state, paired Chromium `touchStart` / `touchEnd` submission,
cancellation cleanup and the narrow `page.touchscreen.tap(x, y)` N-API port.
The selector action composes strict Page, Frame, Locator and ElementHandle
resolution, actionability, hit-target interception, trial mode and temporary
modifiers over that primitive. Node validates public values and delegates; it
does not synthesize DOM touch or mouse events.

The complete pinned Chromium Library `tap.spec.ts` tranche contains nine
unchanged invocations. A preserved v2 candidate failed 0/9 because the
candidate fixture adapter had not forwarded `hasTouch`; after that explicit
Context prerequisite was applied, preserved v3-v5 candidates pass 8/9. The
only remaining red is the public requirement that tap return without waiting
for a navigation whose response is deliberately held.

The navigation investigation found two distinct ordering boundaries. First,
Playwright's Chromium driver submits `touchStart` and `touchEnd` together
before awaiting either reply, so both Rust input policies now do the same while
retaining an active-touch cleanup record until `touchEnd` succeeds. Second,
Playwright invokes tap with `waitAfter: 'disabled'`; Pwright was nevertheless
awaiting both a topology publication fence and a Page-binding publication
fence after the native input completed. Tap alone now bypasses those two
post-action fences. Other Locator actions retain the existing publication
boundary.

After the two foreign pages exited naturally, final candidate v6 and the
independent Playwright 1.58.0 oracle each passed all nine unchanged tests with
no title filter. Candidate/oracle report SHA-256 values are
`f165c1c1244c86d82c71e424f9294539d5e5b222b19408019db32c45edc71086`
and `e3eb0bd0b4a0dd7dbe12b0be4cb774ab4a9f2a41170fd0cf6e9ae7cea21e664e`;
verified provenance values are
`63a140749fd0654ba7a136121e8e822acd073f13d6c673b76c61caa92294ce35`
and `e886e57be1b57fc46b41e25180b492fa2d07b366b375d3ab08ab31b3c736e07b`.
The focused manifest is `076d0c7...705b3`, candidate source is
`e0c5f40...cfb74`, and the explicitly selected official artifact is
`5fb122e3837f9df2742aa6ebbf82c38e0f3e379b60e426f2d9f60853b9459b91`.

Strict candidate and oracle comparisons each promote the same nine inventory
keys from not-run to expected, with one attempt and zero inventory drift.
Candidate reaches 1,521 expected / one skipped / two unexpected / 46,953 not
run, ledger `d084416...8bac`; oracle reaches 1,219 expected / one unexpected /
47,257 not run, ledger `1589687...8b0`. Both invocation audits remain
`32520af...8af0`. Package 375/375, strict TypeScript, both native-load profiles,
ABI/dependency boundaries, backend release 89/89, exact-profile host release
801 passed plus 22 explicit browser ignores, formatting, diff check and strict
host/addon Clippy pass. Complete-corpus conformance remains open.

## Touch event materialization and Context emulation (NODE-427, 2026-10-06)

The complete unchanged Chromium Library
`locator-dispatchevent-touch.spec.ts` file now passes 1/1 in both the Pwright
candidate and independent Playwright 1.58.0 oracle. No title, assertion or
project is filtered. Browser safety preflight initially found an unrelated
nonblank dedicated-Chrome page, so no official red run was started. Two direct
candidate diagnostics instead isolated the discrepancies without disturbing
that page: `TouchEvent` rejected plain records because they were not real
`Touch` objects, and after constructing them the public fixture stopped at the
explicit unsupported `Browser.newContext({ hasTouch: true })` boundary. These
diagnostics are not presented as official report evidence.

The Rust host's static `DISPATCH_EVENT_SOURCE` now recognizes a TouchEvent
constructor and materializes all three touch lists with the browser's native
`Touch` constructor. Existing Touch instances are preserved; plain records get
the resolved Locator element as their default target. Strict Locator
resolution, timeout, event construction, browser execution and error
classification remain in the selected `locator-dom-actions` Rust capability.
Node forwards only the public event name and initializer.

Context touch emulation already existed as `DeviceContextOptions` plus a
bounded 16-Context Rust registry installed by the session-initialization fence.
The missing layer was selection. The addon now exposes an independent
`device-emulation` feature marker and the narrow boolean
`newContextWithHasTouch` port. `context-api-request` and `page-viewport`
declare that prerequisite instead of receiving it implicitly, and the package
has a standalone `device-emulation` artifact profile. The official and
Context-request-proxy profiles name the leaf explicitly. The public adapter
validates the boolean, rejects unsupported feature combinations before native
work and otherwise delegates creation once; it owns no CDP command, registry,
emulation state or teardown.

The rebuilt official artifact SHA-256 is
`69a91eae35af4047d44f508be17f36629e4653ff64c4802fd79ff789962a7de1`.
Candidate source SHA-256 is
`250a6d1de2abdcefadfedd8b903a107afd6de2cd502ef277cc615f9d2329b5d1`;
the focused manifest is `0b300d7...d354`. Candidate and oracle reports pass
1/1 with SHA-256 values `9b5a550...918b` and `2ed7cd4...be5e`;
their verified provenance sidecars are `e3907f9...cb73` and
`51acd91...bd3`. A separate task-owned Context smoke reproduces all three
official TouchEvent strings and reports `navigator.maxTouchPoints === 1`.

Strict ledger comparison promotes exactly the same single `library` /
`chromium-library` row from not-run to expected, with one attempt and zero
inventory-field changes. Candidate reaches 1,512 expected / one skipped / two
unexpected / 46,962 not run (`7d0d844...1260`); oracle reaches 1,210 expected /
one unexpected / 47,266 not run (`fb3aef0...06ed`). Both invocation audits
retain `32520af...8af0`. Package 368/368, strict TypeScript, native load,
dependency boundaries, formatting and diff checks pass. The exact selected
host release suite passes 796 non-ignored tests plus 22 explicit browser-only
ignores in its library target and 818 plus 30 ignores across all selected
targets; strict host and addon Clippy also pass. Full Core and Python corpus
execution remains open.

## Locator query composition (NODE-426, 2026-10-06)

The complete unchanged Chromium Page `locator-query.spec.ts` file records an
honest candidate 12/24 against independent Playwright 1.58.0 oracle 24/24. No
title, assertion or project is filtered. The 12 reds cover seven
regular-expression `hasText` / `hasNotText` filters, three ordered text-array
expectations, one invalid nested FrameLocator composition and one private
before-query timing hook.

The Locator facade accepts `string | RegExp` for the two text filters. A
cross-realm-safe RegExp check extracts only source and flags; the addon applies
the existing Rust `HostLocator::filter_regex` recipe with an exclusion bit.
Matching, descendant scoping and strict resolution remain Rust-owned. The
ordered-array matcher similarly exposes the existing
`HostLocator::expect_texts` operation and passes bounded exact strings plus
normalization, case, `useInnerText` and negation policy. The official cases in
this tranche contain exact strings. Mixed regular-expression arrays remain an
explicit unsupported boundary for the later expectation tranche, not an
implicit success or skip.

Two independently-created FrameLocator wrappers are now equivalent when their
typed selector recipes are equal; JavaScript object identity is irrelevant.
When `and()` or `or()` receives different lazy iframe-owner paths, the Rust
host retains both recipes as a deferred invalid combination. Construction is
therefore synchronous and side-effect free just like Playwright, while the
first host operation raises the official cross-frame diagnostic. A preserved
23/24 intermediate candidate shows that only this error timing remained after
the browser semantics were complete.

`Locator.count({ __testHookBeforeQuery })` is classified
`public-api-with-implementation-private-hook`: it is implemented solely to run
the unchanged official behavior test and is not advertised as a general
public option. Node validates that no other option is present and turns the
callback into one bounded thread-safe promise. Rust's
`count_with_before_query` selects the current execution-world record, awaits
that promise, and then evaluates count using the same record. If the hook
navigates the page, the old world is destroyed and the retriable context-loss
path yields the same result as Playwright's server-side `queryCount` ordering.

Final artifact SHA-256 is `c12153d...d1bc`. Candidate and oracle pass 24/24
with verified reports `54e5e20...d913` and `60508d7...5f8c`;
candidate/oracle provenance is `deecead...ad7` / `fb1b7c8...2413`.
Preserved evidence includes the 12/24 red report `f674c52...ca65` and the
23/24 intermediate report `442f10b...b79d`. Strict diffs promote exactly the
same 24 rows without changing inventory. Candidate becomes 1,511 expected /
one skipped / two unexpected / 46,963 not run (`083773b...7d1`); oracle
becomes 1,209 expected / one unexpected / 47,267 not run
(`6961c30...eaf`). Both invocation audits hash to `32520af...8af0`.

Package 363/363, strict TypeScript, native load, dependency boundaries,
formatting, diff check and exact feature-union Clippy pass. The exact official
Node-profile host library target passes 795 non-ignored tests plus 22 explicit
browser-only ignores; across all selected host targets the totals are 817 plus
30 explicit ignores. This closes all 123 Locator Chromium cases, while the
full official Python and Core corpora remain open.

## Locator miscellaneous viewport, capture and selection (NODE-425, 2026-10-06)

The complete unchanged Chromium Page `locator-misc-2.spec.ts` file records an
honest candidate 12/16 against independent Playwright 1.58.0 oracle 16/16. No
title, assertion or project is filtered. The four reds are the missing public
`Locator.selectText()` and `Page.setViewportSize()` methods, the latter also
blocking the Locator screenshot and bounding-box cases, plus the wrong public
capitalization/prefix for unknown keyboard keys.

`page-viewport` and `locator-screenshot` were already independent Rust host
features. The Node addon adds feature markers and one narrow method for each;
the public wrappers normalize only Playwright option shapes. Page-scoped
device overrides, strict selector resolution, actionability, scrolling,
stability, final document geometry, CDP capture and bounded result bytes remain
Rust-owned. Locator capture explicitly removes Page-only `full_page` and
`clip` options at the binding boundary: `Some(false)` is a real Page option,
not an acceptable substitute for absence in the Locator capability. An AAA
Rust regression locks that distinction.

`selectText` reuses the existing `locator-dom-actions` feature and action
resolver. A preserved 15/16 real-Chrome run proves viewport, selection,
screenshot and geometry behavior. The only remaining failure showed that
`Press` enters the keyboard-specific host route. That route now receives an
explicit typed-error mapper in both ordinary and drag-aware compositions;
only `InvalidValue("unknown key: ...")` is translated to Playwright's
`Unknown key: ...`, while all other errors keep their existing mapping.

Final artifact SHA-256 is `3edcb5c...71db`. Candidate and oracle pass 16/16
with verified reports `fe9ba30...482c` and `f7010a8...5173`;
candidate/oracle provenance is `c53a522...bc90` / `969c169...06fd`.
Preserved evidence includes the 12/16 red report `8290864...96ad6` and the
15/16 intermediate report `eec922c...739b0d`. Strict diffs promote exactly the
same 16 rows without changing inventory. Candidate becomes 1,487 expected /
one skipped / two unexpected / 46,987 not run (`8c78583...c841`); oracle
becomes 1,185 expected / one unexpected / 47,291 not run
(`7d75835...4ef9`). Both invocation audits hash to `32520af...8af0`.

Package 360/360, strict TypeScript, native load, dependency boundaries,
formatting, diff check and exact feature-union Clippy pass. The exact official
Node-profile host library target passes 793 non-ignored tests plus 22 explicit
browser-only ignores; across all selected host targets the totals are 815 plus
29 explicit ignores. Twenty-four Locator Chromium cases and the full official
corpora remain open.

## Locator miscellaneous actions (NODE-424, 2026-10-06)

The complete unchanged Chromium Page `locator-misc-1.spec.ts` file records an
honest candidate 11/13 against corrected independent Playwright 1.58.0 oracle
13/13. The initial oracle attempt is retained only as invalid infrastructure
evidence because its contaminated Chrome session rejected
`Browser.setDownloadBehavior` before all 13 test bodies. No title, assertion or
project is filtered in the clean run. The candidate reds are exactly the
missing `Locator.dispatchEvent()` facade and the unselected file-input leaf.

`locator-dom-actions` remains an independent Rust host feature. The Node addon
adds only a compiled feature marker and a `NativeLocator.dispatchEvent` port.
The facade validates public option and timeout shape, then forwards the event
type and opaque initializer once. Bounded language conversion, object/null
validation, strict Locator resolution and the static DOM event constructor
program remain in Rust. The API therefore composes with existing Locator
recipes without introducing a Node driver or a second action implementation.

The upload case needs no new core logic. The pre-existing `file-input` leaf is
added only to the explicit `official-conformance-page-bindings` feature union,
and its existing facade and Rust preparation/transport owners are reused.
Exact native-profile tests reject an artifact missing either marker, while
default and narrower profiles continue to omit both capabilities.

Final artifact SHA-256 is `356f84b...c39e`. Candidate and corrected oracle
pass 13/13 with verified reports `566e6fd...6a85` and
`28b9a9a...568b`; candidate/oracle provenance is `59fbc53...5ca0` /
`26886bc...51ed`. The preserved 11/13 red report is `a780dca...7054`.
Strict diffs promote exactly the same 13 rows without changing inventory.
Candidate becomes 1,471 expected / one skipped / two unexpected / 47,003 not
run (`1afb6c7...ecaa`); oracle becomes 1,169 expected / one unexpected /
47,307 not run (`1a73f91...ccb9`). Both invocation audits hash to
`32520af...8af0`.

Package 355/355, strict TypeScript, native load, dependency boundaries,
formatting, diff check and exact feature-union Clippy pass. The expanded
official Node-profile host library target passes 790 non-ignored tests plus 22
explicit browser-only ignores; across all selected host targets the totals are
812 plus 29 explicit ignores. Forty Locator Chromium cases and the full
official corpora remain open.

## Locator FrameLocator composition (NODE-423, 2026-10-06)

The complete unchanged Chromium Page `locator-frame.spec.ts` file first
records candidate 3/22 against corrected independent Playwright 1.58.0 oracle
22/22. The initial oracle attempt is retained only as invalid infrastructure
evidence because its already-contaminated Chrome session rejected
`Browser.setDownloadBehavior` before all 22 test bodies. No title, assertion or
project is filtered in the corrected run.

`Locator.contentFrame()` and `Locator.frameLocator()` hand the existing native
Locator recipe to a `NativeFrameLocator`; nested selectors, semantic leaves,
`first` / `last` / `nth` and `owner()` derive another bounded recipe. Node
retains only the public wrapper and description metadata. It never resolves a
Frame, polls DOM state, decides strictness or owns a navigation retry. The
independent routed-frame fixture also required the missing `Route.fulfill()`
port: its facade only normalizes public status, headers, body, JSON and content
type, while Rust validates and performs the one Fetch settlement.

Rust resolves the owner frame at each attempt. Reads and actions retry known
execution-context replacement while preserving the caller's original deadline
and call log. Missing frames return the Playwright state result appropriate to
count, visibility, hidden and text collection reads; owner ambiguity remains a
strict error. A matched non-frame owner is inspected once before its temporary
Handle is released so the diagnostic can report `<div></div>, <iframe> was
expected`. Terminal CSS `:has-text()` is parsed into the existing typed
substring filter instead of being executed by ad-hoc Node code.

Final artifact SHA-256 is `fbc48ff...1382b`. Candidate and corrected oracle
pass 22/22 with verified reports `076f817...fb71a` and
`0135d60...e6e55`; candidate/oracle provenance is `ded1730...0502` /
`27230d7...54f4a`. The preserved 3/22 red report is `cdefa69...fc520`.
Strict diffs promote exactly the same 22 rows without changing inventory.
Candidate becomes 1,458 expected / one skipped / two unexpected / 47,016 not
run (`176cc90...a3b`); oracle becomes 1,156 expected / one unexpected /
47,320 not run (`6106eae...546`). Both invocation audits hash to
`32520af...8af0`.

Package 352/352, strict TypeScript, native load, dependency boundaries,
formatting, diff check and exact feature-union Clippy pass. The exact official
Node-profile host suite passes 788 non-ignored tests plus 14 explicit
browser-only ignores. Adding the pre-existing upload verification leaf passes
790 plus 22 explicit browser-only ignores. These are two named feature
selections, not interchangeable totals. Fifty-three Locator Chromium cases
and the full official corpora remain open.

## Locator convenience composition (NODE-422, 2026-10-06)

The complete unchanged Chromium Page `locator-convenience.spec.ts` file first
records candidate 13/22 against independent Playwright 1.58.0 oracle 22/22.
The nine reds are the absent `Locator.evaluateHandle`, incomplete timeout and
Handle-preview presentation, and the public `describe` / `description` /
`toString` surface. No title, assertion or project is filtered.

Structural descriptions are immutable Node metadata over Rust-owned Locator
recipes. CSS, semantic, role, filter, nth and combination derivations create a
new description and deliberately discard an earlier custom label; `describe`
returns a new Locator with a bounded custom label and never mutates selection.
Deferred Locators restore the outer structural/custom metadata after native
resolution, so a lazy FrameLocator cannot leak its internal owner query into
diagnostics. Rust still owns timeouts and retries; Node adds only the public
call-log line to a returned timeout.

`evaluateHandle` admits one bounded conversion, resolves one strict exact
Element under the original timeout, then invokes the existing retained-Handle
program. The language callback can materialize only its bounded function and
argument; it cannot receive the temporary DOM owner. Rust publishes only the
result Handle. Drop-based cleanup releases the unpublished Element and any
unpublished result on conversion failure, evaluation failure, publication
failure or cancellation. If the result is a Node, a fixed Rust browser program
captures the bounded Playwright preview before publication.

The final artifact is `23f0bf4...390f05`; candidate and oracle pass 22/22 with
verified reports `b659794...146ae` and `86d5d28...458ed`. Candidate provenance
`fa6b260...5eafb` pins source `e283df2...f405f`, manifest
`092087b...98de`, the artifact and upstream commit; oracle provenance is
`13cebd2...73fff`. The strict candidate and oracle ledger diffs each promote
exactly 22 not-run rows and change no inventory fields. Candidate is now 1,436
expected / one skipped / two unexpected / 47,038 not run (`6db265c...b8df`);
oracle is 1,134 expected / one unexpected / 47,342 not run
(`61f077a...862d`). Package 349/349, host release 807 non-ignored plus 22
explicit live-browser ignores, strict TypeScript, native load, dependency
boundary, formatting, diff check and full feature-union Clippy pass.

## Supported Locator file reverification (NODE-421, 2026-10-06)

Five unchanged official Chromium Page files exercise only capabilities already
present below the Node facade: `locator-evaluate.spec.ts` (6),
`locator-list.spec.ts` (1), `locator-highlight.spec.ts` (1),
`locator-click.spec.ts` (5) and `locator-is-visible.spec.ts` (8). Two fixed
manifests collect all 21 tests without title filters. Their SHA-256 values are
`626fb18...e5498de` and `447cd37...e11224`.

Independent Playwright 1.58.0 oracle and Rust-backed Pwright candidate both
pass 21/21 in dedicated Chrome. Candidate report SHA-256 values are
`70c5dc8...731ade` and `af5bb38...a712d9`; oracle report values are
`89ac549...3f6bb8` and `0c9ca82...1eb7ed`. All four provenance sidecars verify.
The candidate artifact remains exactly `8e704cc...6f00a`; this checkpoint adds
evidence, not an implementation change.

The strict candidate diff replaces only generated evidence fields on the same
21 expected rows and leaves counts unchanged at 1,414 expected / one skipped /
two unexpected / 47,060 not run (`06e0444...63546`). The strict oracle diff
changes exactly those 21 rows from not-run to expected, reaching 1,112 expected
/ one unexpected / 47,364 not run (`a813017...58324`). Inventory metadata is
identical in every one of the 48,477 rows.

The complete Locator family is 123 Chromium Page tests across 11 files. Fixed
manifests now cover all of them; the convenience manifest collects 22 and the
frame/misc-1/misc-2/query suites collect 22/13/16/24. These are inventory and
execution boundaries, not passing claims for the remaining 97 cases.

## Locator ElementHandle composition (NODE-420, 2026-10-06)

The complete unchanged Chromium Page `locator-element-handle.spec.ts` file
first records candidate 0/5 versus independent Playwright 1.58.0 oracle 5/5.
The candidate report SHA-256 is
`cbf16291f4aa6b48659b70408b9dd668506da27bdef141320fdf3d652e489fae`;
oracle report SHA-256 is
`0f1655db8726aabcbf7e222b9b1afb0c4b76527c36402c598e7771d2d045ae0d`.
No title or assertion is filtered. Three reds prove the absent public
`Locator.elementHandle()` / `elementHandles()` facade. Two prove that the
retained Rust query rejected a typed parent followed by complex relative or
absolute XPath.

The JavaScript repair is deliberately thin: `elementHandle()` forwards to the
already strict native operation, while `elementHandles()` forwards to the
existing Rust batch query. The Rust query plan now stores an immutable bounded
sequence whose segments are either a typed `Selector` or a tagged browser
source. Descendant composition flattens those segments under the caller-owned
recipe budget; compilation nests each later segment under `__pw.within` and
checks the fixed compiled-source bound after every step. Typed selectors are
never reverse-serialized into language-owned strings. Only `Within` is
installed; `And`, `Or`, `Has` and `HasNot` involving browser programs remain
explicit errors.

For `elementHandle()`, Rust compiles the browser program once, owns the total
deadline and repeats a strict optional browser probe. Zero matches continue
waiting, one match enters existing native Handle publication and multiple
matches fail immediately with strict-mode diagnostics. Timeout zero preserves
the existing unbounded convention. `elementHandles()` remains one non-waiting
ordered snapshot with batch publication and cleanup below FFI.

Final artifact SHA-256 is
`8e704ccbf94524d85424914ea6aef548f29871e79437c84d034223ed80a6f00a`.
Final candidate and oracle both pass 5/5; report SHA-256 values are
`7dfc27e89e46c0e3c85e027ecfc6b00623359e9d743e6cc931ca74ba7ae71a49`
and `0f1655db8726aabcbf7e222b9b1afb0c4b76527c36402c598e7771d2d045ae0d`.
The final candidate provenance sidecar SHA-256 is
`36a737def5c17fc7756ef5b2b8f18a1f8c95a3d50cc41cb6b3cdec3bbd0593a8`;
it pins source `9a3131d...8c492`, manifest `39d5683...45db` and the fixed
upstream commit. Oracle provenance is `23643ec...d6ae`.

Strict merge changes only those same five rows from not-run to expected.
Candidate reaches 1,414 expected / one skipped / two unexpected / 47,060 not
run (`6fa68d3...9db39`); oracle reaches 1,091 expected / one unexpected /
47,385 not run (`6da3adc...83d6e`). Host release passes 615 non-ignored units
with 14 explicit live-browser ignores; package units pass 346/346. Strict
TypeScript, native load, dependency boundary, formatting, diff check and the
full selected-profile Clippy gate pass. This completes one official file, not
the full Core or Python corpus.

## ElementHandle DOM reads and official red baseline (NODE-418/419, 2026-10-06)

NODE-418 records three complete, unchanged Chromium Page files at the pinned
Playwright commit. Candidate and oracle both pass
`elementhandle-query-selector.spec.ts` 7/7 and
`elementhandle-eval-on-selector.spec.ts` 6/6. The complete
`elementhandle-convenience.spec.ts` is an intentional red baseline: candidate
passes 5/15 while the independent oracle passes 15/15. The ten candidate
failures are two rich synchronous Handle previews plus missing public
`getAttribute`, `inputValue`, `innerHTML`, Handle visibility/state methods and
Page selector convenience/action facades. No assertion or file filter was
removed.

The NODE-418 candidate ledger records 1,399 expected / one skipped / 12
unexpected / 47,065 not run, SHA-256
`5c1cd26cc8a8bf22bf57436627de682344245b8eba68ca61dbf1e5d3506c5766`.
The oracle ledger records 1,086 expected / one unexpected / 47,390 not run,
SHA-256
`8e87e050ba9b088b849d8296b19bed0856a92fce99c82ec2214a5aa28fc1add7`.
The query, eval and convenience candidate report SHA-256 values are
`c5967af28ca871969773ab7a47a296544c17acde2d985592fe91820024ae7c78`,
`005d19e49cc3d9d242f9ae957e9a64d46d4c8dd9d4abccaa2cbaae0470ae3cb3`
and `8f27f41a81b80ab49b69b9f4d33e8bd88f6a49149aec024c764d9b73eb9684a1`;
the paired oracle values are
`8c1b2cfcd09729b6df1836fb8dd8047b4fbd42d52be32f6146a42b1ee03f8aeb`,
`ef3f3bc845dad361863f519556d0aff8db6b52341b843d546ba708e480b8c43c`
and `1bd4bbc82e0a18502555a55a802084fea82f65a5bf3c57bff8b940442908df1d`.

NODE-419 implements the non-preview discrepancies without moving browser
semantics into Node. One optional Rust `element-handle-dom` read owner keeps
the exact retained browser object and returns bounded text or boolean values.
`inputValue` and enabled/editable/checked state reuse the versioned selector
runtime's retarget and state primitives; visibility has explicit disconnected
semantics. The addon exposes one generic read port. `ElementHandle`, `Page` and
`Frame` are thin public facades, and Page/Frame actions reuse the independently
composed Locator action owner.

Nine Rust source/semantic tests and the first rebuilt
`official-conformance-page-bindings` artifact (SHA-256
`6290125fb3f659e07c506f51a8248102175854b87237ac5262386a764e978218`)
then produced a complete unchanged candidate rerun at 12/15. Its result
SHA-256 is
`98fa228cd117457ca23c2fd7f52868483ebc81caf6b9d0e268dfa6626c2e87a9`.
The three remaining failures were exactly the two rich Handle previews and a
Playwright `nth=0` chain used for option/optgroup state.

The follow-up keeps both repairs composable and Rust-owned. The typed and
registered selector compilers now model `nth=<i32>` as a chain operation,
including negative indexes, instead of sending it to CSS. ElementHandle
creation starts one asynchronous Rust preview read over the exact retained
Node. A fixed Rust-owned browser program implements Playwright's attribute
ordering, boolean/void elements, text-vs-ellipsis rule, Unicode code-point
truncation and newline/tab glyphs; Node only caches the bounded returned
`JSHandle@...` string and suppresses preview-only refresh errors, matching the
upstream lifecycle.

The final release artifact SHA-256 is
`02475436552ed68237548b0c4d5840993fde0a1db5abdea21e875cfe778db47f`.
After the unrelated Page exited on its own, the complete unchanged candidate
file passed 15/15 from an otherwise empty dedicated Chrome. The verified final
report SHA-256 is
`eee2d882cac73ba18522c244ee8706fadd2adb9aa4387818a6c0e15a6e214b68`;
its provenance sidecar SHA-256 is
`13072281e83257e77320f42d728eed0d547512b12784d2d0376b08092afa6954`.
The sidecar pins candidate source
`670fa82a0c30920c3dd6f3fc824f011b3c7ada7f42ccf17a9c4e526e6a4d3fe7`,
manifest `eb64a772...6fa35` and upstream commit `961381e...affc75`.

The strict 48,477-row merge changes exactly these 15 library invocations: five
remain expected and ten move from unexpected to expected. The candidate ledger
is now 1,409 expected / one skipped / two unexpected / 47,065 not run,
SHA-256
`b5532ffbcd2819662585b698b766bb3c4e9a434a8d4540f4b790a482eedc5940`.
Host release units pass 609 with 14 live-browser ignores; package units pass
346/346. Strict TypeScript, native load, dependency boundary, release check,
full feature-union Clippy, formatting and diff checks pass. This closes the
NODE-419 file, not the full Core or Python corpus.

## Composable Page and Frame query selectors (NODE-417, 2026-10-06)

Page and Frame `$` and `$$` now share a Rust-owned selector-plan boundary.
Typed CSS and text remain portable `Selector` values. Full XPath is retained
as a separately tagged, budgeted browser program, so it is never represented
as CSS and the Node facade never parses it. The Rust compiler recognizes
explicit `xpath=`, automatic `//` and `..`, parenthesized XPath and relative
XPath inside `>>` chains. Ordinary chain parts still compile through the same
typed selector compiler.

The versioned backend runtime exposes one XPath primitive over
`Document.evaluate`. It converts a leading absolute path to relative only when
the root is not a Document, returns Elements in XPath order, and traverses no
language-owned registry. Page/Frame `$` uses the existing optional native
Handle publication. `$$` evaluates once to an indexed carrier, publishes the
complete ordered ElementHandle batch through Rust and settles the carrier
before returning. Neither path calls page-overridable `Array.from`.

The unchanged official `queryselector.spec.ts` candidate first passed 10/17;
the seven failures were exactly its XPath rows. The repaired candidate and
Playwright 1.58.0 oracle pass 17/17 with verified provenance and a strict
17-row ledger diff. Candidate reaches 1,396 expected / one skipped / two
unexpected / 47,078 not run, SHA-256
`675ed26f79d073b8fe1511b54b496065779313e99f0eba100fec632531a58a38`;
oracle reaches 1,073 expected / one unexpected / 47,403 not run, SHA-256
`fdd3dc71b4560a66c04336a2cf68a20bd00a8d63a69d940bfcac1c81a9f2d950`.
The release artifact SHA-256 is
`386d03a6a893c931b6aae993056b5bfef1727fbcd4e4638158217a3c64a75d3c`.
Package 335/335, strict TypeScript, native load, boundary, release check,
full-profile Clippy, formatting and diff checks pass. Other Locator operations
over browser-program selectors remain explicit unsupported boundaries rather
than silently changing semantics.

## Atomic optional Page query (NODE-416, 2026-10-06)

Page/Frame `$` now performs one Rust-owned browser program that returns either
the first matching Node or exact JavaScript `null`. The CDP backend classifies
that retained result before publication. Null releases route admission without
creating a Handle; Nodes enter the existing bounded publication path; utility
world Nodes are adopted; non-Node objects are released before the original
protocol failure is returned. `Locator.elementHandle()` keeps its separate
strict, waiting contract.

The Node addon publishes the optional result as native ElementHandle or null,
and JavaScript performs only public wrapper creation. It does not derive
`.first()`: the first live official attempt proved that typed filter is invalid
for a registered browser-program selector. Rust's query program itself owns
non-strict first-match selection for both ordinary and registered selectors.

The repaired candidate and Playwright oracle pass path, main/isolated-world,
error, non-root and bad-return rows 5/5 in each lane with verified provenance.
The candidate ledger advances by exactly five rows to 1,379 expected / one
skipped / two unexpected / 47,095 not run, SHA-256
`8be091c0356f65de9a05b89b5a826d07ca6090342884fabb8296128a34ee8ea6`.
The artifact SHA-256 is
`2018ea0912efd134c8e5b9984b168f722fcdd307900399e2b02a9250398d3d9e`.
Package 331/331, strict TypeScript, native load, boundary, release check and
full composed Clippy pass; focused native creation tests prove null admission
reclamation, skipped adoption and non-Node cleanup.

## Rust-owned registered ElementHandle and Browser.newPage bridge (NODE-415, 2026-10-06)

Registered-selector `page.$` now obtains its positive ElementHandle from the
same bounded Rust selector compiler as `$eval` and `$$eval`. Rust chooses the
required main or utility world, validates that the engine returned a Node,
adopts utility-world nodes into the canonical main world, and transfers the
already admitted native owner into the ordinary durable Handle route. Failure
before publication compensates the source, adopted result and route
reservation; JavaScript only wraps the published native Handle.

Page/Frame `$` now derives `first()` in the typed Locator recipe before the
strict Handle primitive is invoked. This preserves Playwright's non-strict
first-match behavior when more than one element matches without weakening
`Locator.elementHandle()` itself. The preliminary count and Handle creation
remain two browser reads, so DOM mutation between them is still an explicit
atomic-query gap rather than a claimed fix.

The Node addon also exposes the pre-existing Rust `Browser.newPage`
composition already used by Python. Rust creates the implicit Context and Page
atomically, owns their exclusive relationship and closes the Context through
`Page.close()`. The Node facade currently accepts no options, `baseURL`, or
explicit `acceptDownloads: false`. Download acceptance remains an independent
unselected capability and `acceptDownloads: true` fails before native work;
other Context options are likewise rejected rather than ignored.

The official-profile artifact SHA-256 is
`6196f520808b3ceb2059ff2a88cfa5fc85faa90ee34d728f25d4c9336d072dfe`.
Invalid-name, predefined-engine, duplicate-registration and unknown-engine
diagnostics are covered at the public Node boundary. Package units pass
329/329, strict TypeScript, native-load, dependency boundary,
official-feature Clippy, backend release 86/86 and composed release 61/61 pass;
the freshly rebuilt artifact passes the browserless official
selector-registration regression 3/3 in library and 6/6 in BiDi with verified
provenance sidecars. Result SHA-256 values are
`7906d3150e173216c4aa042403964ded687e13b1142e2c0894087458d5914fb4`
and
`797c48a2daf8bb0a13ee9a55ed4603b5df6c160cc1d9502549f7d076ca593631`.
The Core ledger
does not move because those nine rows were already expected. The official
browser-backed selector-world row has
not run: the safety preflight found an unrelated nonblank `仙侣奇缘 · Bevy`
Page in the dedicated Chrome and refused to start. No external Page was reused
or closed, so real-Chrome Handle/adoption and remaining selector rows stay open.

## Browser-backed selector execution (NODE-414, 2026-10-06)

The first registered-engine browser row now keeps selection in Rust from the
public Page/Frame facade through the existing registered program compiler.
The addon exposes the host's non-strict `evaluate_first_with` operation as
`NativeLocator.evaluateFirst`; `$eval` uses that path so multiple matches
select the first element, while `$$eval` uses the existing collection lane.
Node only converts the callback and argument. A preliminary Rust `count()` on
`page.$` supplies selector parsing, case-sensitive unknown-engine diagnostics
and zero-match `null` without granting JavaScript a selector parser.

Before this bridge, the official candidate failed on `tag=DIV` while the
pinned Playwright 1.58.0 oracle passed. The repaired candidate and oracle pass
the same Chromium library row 1/1; both provenance sidecars verify. Report
SHA-256 values are
`64ebff250009fcfc02f759b5fdca0c8fa34b07f33b6ed67de438bb2d8ee17af4`
and `451b431e79c69c8f5c205c2aa4363ec2b5a1c87a6b92ff9afde7c078a3a7996e`.
The composed native artifact SHA-256 is
`264137762c0f112e6d91ff21a3da4eed9284d566992f0b125124c1ab0caa802b`.
A strict one-row ledger diff reaches 1,374 expected / one skipped / two
unexpected / 47,100 not run, SHA-256
`f363803e73aeae00ca2226c76bd5f5b53f74d9e947fc7c65725bb19198f4a465`.
All 319 package units, strict TypeScript, package boundary, release/native-load
and full composed-profile Clippy gates pass. Registered-engine Handle
publication for positive `page.$`, path/content-script cases and bad-return
diagnostics remain separate capabilities and are not claimed by this row.

## Node selector-registration capability (NODE-413, 2026-10-06)

The Node package now exports Playwright-shaped `selectors.register()` and
`selectors.setTestIdAttribute()` through CJS, ESM and TypeScript. The facade
only normalizes a function, string, inline content or filesystem path and maps
the official lowercase Node API error. It lazily creates one
`NativeEnvironment`, so registrations made before a CDP attachment remain in
the same Rust-owned registry later cloned into the session and locator graph.

The capability is independently selected as `selector-registration`. Its
native marker proves both environment methods, the default binary stays cold,
and the selected profile adds no Pwright package dependency. Rust retains the
existing 64-engine, 64-byte-name, 1 MiB-source and 8 MiB-aggregate bounds,
predefined/duplicate validation, custom-program compilation and configured
test-id attributes. Registered CSS construction, derived FrameLocator paths,
semantic test-id leaves and strict action diagnostics all consult that shared
owner when the feature is present.

The initial official candidate failed all nine selected browserless
library/BiDi instances at an absent `playwright.selectors`; the oracle passed
9/9. The selected release artifact (SHA-256
`9fb0f50bc05c54c4831d8ee51623c3d1145721467b7f62c07b4bab073a39461a`)
repairs candidate to 9/9, and all four candidate/oracle sidecars verify. A
strict ledger diff promotes exactly those nine rows, reaching 1,373 expected /
one skipped / two unexpected / 47,101 not run with ledger SHA-256
`60b1c6f95b95f7bddb2539aacf47c02496dd8e8b41238ab731f7bb63c829a3ab`.
Node units are 317/317, strict TypeScript passes, Rust registry units are 8/8,
and release check, clippy, dependency, boundary and native-load gates pass.
Positive browser-backed custom-engine behavior remains unclaimed here.

## BiDi launcher metadata completion (NODE-411, 2026-10-06)

The NODE-408 `errors` and complete 143-entry `devices` exports are independent
of the browser transport and project name. NODE-411 therefore runs those exact
two assertions through the official BiDi configuration's six library
projects: BiDi Chromium, Chrome canary/stable, and Mozilla Firefox
regular/beta/nightly. The manifest marks the slice browser-independent; the
runner neither starts nor connects a browser.

Candidate and independent Playwright 1.58.0 oracle pass 12/12. Both provenance
sidecars verify package/harness/source inputs. Candidate/oracle report SHA-256
values are
`0fe174854e824de943effc833a670790a9319c73dc3ccee76fec3d89d1e5f2d3`
and `f47172abcb9f91afebf86d368bb4855649fc5803d141fb0e360cffaefcccb83b`.
A strict ledger diff changes only those 12 rows to `expected`, producing 1,364
expected / one skipped / two unexpected / 47,110 not run with SHA-256
`87d6753547bad1f5f21e84952a2a8ea7af56cc75921e0382ec1ea7486cd1245d`.
The package implementation, Rust ABI, native selection and resource ownership
are unchanged.

## Complete device registry checkpoint (NODE-408, 2026-10-06)

`playwright.devices` is static compatibility data, not browser automation
logic. The initial official browserless launcher-metadata candidate passed the
three `errors` instances and failed all three `devices` instances; the pinned
Playwright 1.58.0 oracle passed 6/6. This isolates one missing public export
without involving Chrome, CDP or a native artifact.

The Node package now vendors the exact upstream 143-entry
`deviceDescriptorsSource.json` snapshot from commit
`961381ec73a9fa2661d92504eab1a6bd6eaffc75`, SHA-256
`ceb47c6d0366b273151121ade0217f1e96a4043594c8188fcc12c0c9b30f9353`.
Its Apache-2.0 license and provenance record ship beside the JSON. CJS and ESM
share the same object and the TypeScript surface includes all public fields,
including optional screen dimensions. Importing the registry does not select
or load a native profile, and no partial single-device shim is used.

The final candidate and independent oracle both pass 6/6 with verified report
SHA-256 values
`2a4ca9220591a7166d0a9dd74d0581631eb0400c40fda59299eb86f92af3ed07`
and `e5f6ceb209f5b00ad63ca513584861487e1755b62cf33ea4e4eae407b1650a75`.
A strict diff promotes exactly six rows. The cumulative Core candidate ledger
is 1,352 expected / one skipped / two unexpected / 47,122 not run, SHA-256
`f382b081ad8346ace986db9cf8254370b34a67b8c514bca9379f573d3f40f906`.
The package passes 312/312 tests and strict TypeScript. Rust ABI, capability
selection, browser ownership and resource accounting remain unchanged.

## Explicit-profile Page init-script revalidation (NODE-372, 2026-10-06)

The complete unchanged Page init-script file passes 9/9 against the current
explicit `official-conformance-page-bindings` artifact and 9/9 independent
oracle. It covers source/path/content normalization, multiple registration,
CSP, cross-origin replacement and iframe once-only installation while Rust
continues to own bounded registration and exact session fanout. The candidate
rows were already present as expected; the oracle ledger alone advances nine
rows to 1,051 expected / one unexpected / 47,425 not-run.

An initial command omitted the profile and correctly exposed only the default
artifact, producing an unmerged 1/9 diagnostic. The runner now requires a
candidate profile explicitly, sets it in the child environment and fails
before execution if it is absent. This is execution-lane hardening, not a new
JavaScript browser owner or a change to the init-script dependency graph.

## Context CSP and extra-header official checkpoints (NODE-369/370, 2026-10-06)

NODE-370 exposes `bypassCSP` through an independently buildable
`bypass-csp` addon profile. JavaScript validates the public boolean, rejects
uncomposed proxy/header combinations and selects one native creation method.
Rust admits the Context policy, retains at most 16 selected Contexts, installs
`Page.setBypassCSP` on every owned Page/OOPIF session before publication and
revokes the policy on Context teardown. The facade contains no CDP method,
session list or retry loop.

Moving feature-gated FrameLocator and console methods into feature-gated N-API
impl blocks fixes a separate composition defect found by the standalone build:
default and `bypass-csp` addons now compile without accidentally requiring
handles or console events. The standalone and official release artifacts both
build and load. Focused Node policy/profile tests use AAA and the package passes
306/306 plus strict TypeScript, native-load, boundary, formatting and whitespace
gates. The unchanged CSP candidate progresses 0/4 to 4/4. The harness first
blocked the oracle while another task owned a nonblank page; after that page
exited naturally, the unchanged oracle passed 4/4 and exactly those four rows
advanced in its complete ledger.

NODE-369 changes no code or dependency edge. The complete unchanged Context
extra-header file passes 2/2 candidate and 2/2 oracle through the existing
Rust header creation/override path. At that paired checkpoint the ledgers were
candidate 1,342 expected / one skipped / two unexpected / 47,132 not-run and
oracle 1,038 expected / one unexpected / 47,438 not-run. The later CSP merge
advances the oracle to 1,042 expected / one unexpected / 47,434 not-run.

## Rust-owned Dialog and Page route checkpoint (NODE-367/368, 2026-10-06)

The official Node artifact now selects the independent `page-dialogs` and
`page-routes` leaves. A `NativeDialog` carries the exact Rust
`DialogAction`; the JavaScript `Dialog` wrapper owns only Page identity and
presentation fields. Page and BrowserContext receive the same wrapper object.
With no public listener, the facade performs Playwright-compatible automatic
dismissal, except that `beforeunload` is accepted. With a listener, only the
user's one-shot native `accept` or `dismiss` settles the action; Rust enforces
duplicate settlement, prompt bounds and teardown revocation.

`Page.route` retains the public callback in Node but registers its bounded
string matcher in Rust. A single native watcher publishes Rust-selected route
calls; Request metadata is a bounded copy and `continue`, `fallback` and
`abort` settle the native owner. A synchronous callback may return without
settling so an official test can call the same Route later. Missing/expired
registrations fall back in Rust. Regex/predicate matching, override objects,
`fulfill` and `fetch` remain explicit unsupported operations instead of being
silently emulated in the facade.

The complete unchanged `page-click-react.spec.ts` passes 3/3 candidate and
3/3 oracle. The complete unchanged `page-wait-for-load-state.spec.ts` passes
18/18 in both lanes. The cumulative candidate ledger is 1,340 expected / one
skipped / two unexpected / 47,134 not-run; the oracle ledger is 1,036 expected
/ one unexpected / 47,440 not-run. The only two executed candidate failures
are implementation-private `__testHook...` options and remain visible pending
an applicability or composable Rust-hook decision. Node passes 301/301 plus
strict TypeScript and native-load smoke. The exact selected host passes 745
tests with 14 explicit environment-gated ignores, and strict selected
host/addon Clippy passes. This is not a complete Core or Python corpus claim.

## Post-commit client-redirect checkpoint (NODE-366, 2026-10-06)

Frame navigation keeps the original command response identity separate from
the later lifecycle target. When a client redirect begins after the original
document commits, the Rust lifecycle owner adopts that same-Frame navigation,
waits for the requested terminal state and still returns the original
Playwright response. `navigation-error.cjs` adds only the language-facing
timeout call-log text after Rust has selected timeout, URL and wait state; it
does not own a timer or navigation state. The complete unchanged
`frame-goto.spec.ts` selection passes 4/4 in both candidate and oracle lanes.

## Console subscription transition checkpoint (NODE-365, 2026-10-06)

Page and BrowserContext remain ordinary Node `EventEmitter` owners for public
listener identity, ordering, `once`, prepend and removal semantics. A small
facade now mirrors only the first and last `console` listener transitions into
the selected native session: zero-to-one enables the existing bounded
Rust-owned subscription and one-to-zero releases it. Multiple JavaScript
listeners therefore do not multiply native subscriptions, and removing every
listener permits a later clean subscription. Release after native owner close
is nonfatal because that owner can no longer retain a live subscription.

The addon exports synchronous, bounded-ID Page and BrowserContext listener
setters only under `console-events`; it introduces no callback registry,
browser observer or queue in JavaScript. Artifact verification requires these
real exports in addition to the feature marker, rejecting stale binaries that
would otherwise silently drop console delivery. This is additive and keeps
the native ABI version unchanged.

A diagnostic confirmed complete Enter key DOM delivery before this repair and
confirmed ordered `manual-before`, `press`, `manual-after` console publication
after it. The unchanged official `page-basic.spec.ts` candidate file passes
18/18. The shared external-CDP oracle is 17/18 only because its Chrome process
lacks the Playwright launch-time automation flag; owned candidate Context
sessions set the equivalent CDP automation override explicitly. Node passes
289/289, strict TypeScript and native-load smoke; the exact selected host
passes 710 tests with 14 explicit environment-gated ignores, and strict
host/addon Clippy passes. This is not a complete-corpus claim.

## Rust-owned FrameLocator and matcher parity checkpoint (NODE-364, 2026-10-06)

The selected `handles` addon now exposes the existing lazy
`HostFrameLocator` through a thin `NativeFrameLocator`. Node retains only that
opaque recipe and presentation metadata; it does not resolve, cache or poll a
child Frame. Every derived native Locator keeps the iframe-owner recipe in
Rust, so replacing an iframe retires the old execution context and the next
matcher attempt resolves the new child Frame. Older native artifacts retain
the bounded Handle/contentFrame fallback.

The same boundary delegates `APIResponse._fetchLog()` to the existing public
Rust-backed response log and keeps 1 ms matcher one-shot behavior inside Rust.
The complete unchanged official boolean matcher file passes 86/86 candidate
and 86/86 oracle, including iframe replacement, exact CSS syntax diagnostics,
response failure printing and impossible-timeout cases. Node passes 286/286
plus strict TypeScript; the selected host passes 709 tests with 14 explicit
environment-gated ignores; release native-load and strict host/addon Clippy
pass. This checkpoint changes no ABI version or native-profile dependency
union and is not a full-corpus conformance claim.

## BrowserContext request-proxy checkpoint (NODE-363, 2026-10-06)

The Node surface now accepts the exact backed BrowserContext defaults plus a
Playwright-shaped proxy object. Native `newContextWithProxy` carries normalized
typed options into Chromium and `contextRequest` returns a Rust-owned request
client that reuses the Context proxy and live cookie jar. `page.request` is the
same public owner. Context close accepts `{ reason }` and only disposes its
owned request client after native close succeeds. Proxy and HTTP credential
values are redacted from Rust `Debug` output.

The ABI version is unchanged; feature markers make `context-api-request`,
`context-proxy` and the three request proxy leaves independently detectable.
The `context-api-request-proxy` release artifact builds and loads with both
native methods present, and the exact dependency proof rejects any package
outside its explicit union. The unchanged official Chromium Library Context
proxy and bypass rows pass 2/2 candidate and 2/2 independent oracle. Node
passes 286/286 plus strict TypeScript; selected Rust host tests pass 526 with
eight explicit environment-gated ignores. This checkpoint does not claim
authenticated browser page traffic, arbitrary nondefault Context options,
other browser engines or complete upstream conformance.

## Proxy target-TLS composition checkpoint (NODE-362, 2026-10-06)

Node retains no socket, TLS or certificate-selection state. It validates the
public proxy and certificate objects, selects one of two exact-union native
profiles and forwards values once. Rust establishes the HTTP CONNECT or SOCKS5
tunnel, then the injected Rustls owner upgrades that existing connection with
target SNI, request-scoped verification and exact-origin client identity. The
proxy leaves do not depend on Rustls and cannot silently provide their own TLS.

Both release profiles build and load. The unchanged official proxy+mTLS rows
pass 3/3 per proxy in Library and 6/6 per proxy in Bidi, totaling 18/18
candidate and 18/18 independent oracle without a browser/CDP. Eleven request
crates pass 113/113 release tests; Node passes 282/282 plus strict TypeScript,
Clippy and exact dependency-union proofs. Core advances to candidate 1,309
expected / one skipped / 31 unexpected / 47,136 not-run and oracle 992
expected / 47,485 not-run.

## Bidi client-certificate checkpoint (NODE-361, 2026-10-06)

No FFI, ABI or runtime code changes in this checkpoint. The existing thin
Node facade and selected Rust certificate profile execute the unchanged input
validation row 6/6 and seven direct-request rows 42/42 across all six Bidi
projects in both candidate and independent oracle. The current release
artifact is rebuilt and load-verified before candidate execution. The Core
ledgers advance to candidate 1,291 expected / one skipped / 31 unexpected /
47,154 not-run and oracle 974 expected / 47,503 not-run.

## Bidi proxy-matrix checkpoint (NODE-360, 2026-10-06)

No FFI or runtime code changes in this checkpoint. The existing thin Node
facade and three selectable native profiles execute the unchanged authenticated
HTTP CONNECT, HTTPS proxy-server ALPN and SOCKS5 rows across all six official
Bidi projects. Candidate and independent oracle each pass 18/18 without CDP.
The Core ledgers advance to candidate 1,243 expected / one skipped / 31
unexpected / 47,202 not-run and oracle 926 expected / 47,551 not-run.

## SOCKS5 proxy checkpoint (NODE-359, 2026-10-06)

The new `api-request-socks5-proxy` native profile extends only the existing
browser-independent request profile. Node validates the public proxy object
and selects SOCKS independently from HTTP CONNECT. Rust parses and bounds the
proxy, performs a no-auth SOCKS5 handshake, sends hostnames to the proxy for
remote resolution, supports IPv4/IPv6 targets and parses the bounded reply.
Credentials and HTTPS request origins are explicit later capabilities.

The unchanged official `fetch-proxy.spec.ts:133` row passes 3/3 candidate and
3/3 oracle across Chromium, WebKit and Firefox Library projects. Candidate and
oracle report SHA-256 values are
`19af5d94d8ccc7196d8200f229b7eafec4f0954c687dddf9c9309ef5d2428438`
and `7926ab68bdb6f9614781b8365b047d59176203adb913d5fa168a98edcbfb87e7`.
The eleven request crates pass 110 release tests, Node passes 279/279, strict
TypeScript and Clippy pass, the selected native release artifact builds and
loads, and a real Tailscale-bound native smoke verifies remote DNS. The current
Core ledgers are candidate 1,225 expected / one skipped / 31 unexpected /
47,220 not-run and oracle 908 expected / 47,569 not-run.

## HTTPS proxy-server checkpoint (NODE-358, 2026-10-06)

The existing optional `api-request-http-proxy` profile now accepts an HTTPS
proxy server without adding another profile or dependency edge. Rust passes
that proxy URL through the injected Rustls connector, carries the public
certificate-ignore policy and offers HTTP/1.1 ALPN before issuing CONNECT. The
request origin remains plain HTTP; HTTPS target tunneling is still unsupported.

The unchanged official `fetch-proxy.spec.ts:143` row passes 3/3 candidate and
3/3 oracle across Chromium, WebKit and Firefox Library projects. Candidate and
oracle report SHA-256 values are
`6dba12195eed6bf3f7ba71b3bc2713d97acf83449db4ed28e350aef5b821d74d`
and `429f9fa3ffec495127cc0f7310815d6eca336a488b74d197f74f97f11a5621c8`.
The ten request crates pass 104 release tests, Node passes 276/276, and the
selected native release artifact builds and loads. The current Core ledgers
are candidate 1,222 expected / one skipped / 31 unexpected / 47,223 not-run
and oracle 905 expected / 47,572 not-run.

## HTTP CONNECT proxy checkpoint (NODE-357, 2026-10-06)

The new `api-request-http-proxy` native profile extends only the existing
browser-independent request profile. Node validates `proxy.server`, optional
username/password and rejects unsupported bypass before native construction.
Rust parses and bounds the proxy, zeroizes/redacts Basic credential material,
establishes one CONNECT tunnel per exchange and retains any prefetched tunnel
bytes. The first leaf intentionally supports plain-HTTP request origins only.

The unchanged official authenticated global-request proxy row passes 3/3
candidate and 3/3 oracle across the Library project matrix. Candidate/oracle
report SHA-256 values are
`6d03f9eaeba8e6c18ad8ceb91c36a76a983e20760013bc451629706245ffeb68`
and `d8349d4e448041af91cb34403cab026b3428f3e5989792a759d2dd2414263c2f`.
The ten request crates pass 103 release tests, the Node package passes 276/276,
the selected profile adds exactly one Pwright dependency leaf, and a real
Tailscale-bound native proxy smoke passes 2/2. HTTPS-over-proxy, SOCKS and
bypass remain separate capability work.

## Node ElementHandle DOM source completion (2026-10-03)

NODE-293 completes the remaining source-side extraction in the same optional
`element-handle-dom` profile. Rust reads `Node.textContent` from the retained
owner, publishes an ordered validated element batch for `$$`, and creates,
evaluates against and disposes the temporary query array for `$$eval` before
settling the N-API call. The addon has failure-safe batch publication; Node
only creates public wrappers and compensates every native owner if wrapper
publication fails. Page and Frame `textContent` remain thin calls through the
existing Rust Locator owner.

The XPath path now mirrors upstream scoping: a selector beginning with `/` is
made relative when the scope is not a Document, evaluation stays rooted in the
retained target, and non-Element XPath results are filtered rather than
published. Numeric batch properties must form one exact zero-based contiguous
sequence; malformed batches are cleaned before returning an error.

Package 273/273, strict TypeScript, standalone strict Clippy, the selected
native build/load, dependency boundaries, formatting and diff checks pass.
These are source and packaging gates, not browser conformance evidence. The
unchanged query-selector 7-row, eval-on-selector 6-row and convenience 4-row
candidate/oracle runs are pending because the zero-tab guard found an unrelated
Gemini page in the shared dedicated Chrome. No page was closed, no new report
was merged and the NODE-292 ledger counts below remain authoritative.

## Node ElementHandle DOM checkpoint (2026-10-03)

NODE-292 exposes the already shared Rust `element-handle-dom` owner through an
independently selectable Node addon/package profile. The profile implies
`handles`, but selecting plain `handles` still does not install DOM methods.
The addon exports an additive feature marker under ABI 1, so older/minimal
artifacts remain detectable rather than being silently accepted.

`ElementHandle.innerText()` and `$()` are one typed native call each. `$eval`
first obtains the exact matching retained target from Rust, evaluates only that
owner through the existing Handle evaluator and releases it. Rust compiles the
descendant selector under the existing query budget, preserves root-relative
identity, publishes only an element, acknowledges the no-match owner release
and enforces `HTMLElement.innerText`. `Page.innerText` and `Frame.innerText`
are thin calls to the existing Rust Locator read. Node retains overload/error
presentation and wrapper identity; it does not query the DOM for these paths.

The unchanged official Chromium selections pass query-selector 7/7,
eval-on-selector 6/6 and inner-text 2/2 in both candidate and oracle lanes.
This 15/15 comparison verifies the refactored boundary, not the entire corpus.
`ElementHandle.textContent`, `$$` and `$$eval` still use generic/query-all JS
composition and remain explicit Rust-extraction tasks.

The package passes 272/272 tests and strict TypeScript. Standalone
`element-handle-dom` and full `official-conformance-page-bindings` artifacts
load successfully; release checks, formatting, selected-profile strict Clippy
and dependency/native-class/no-process-launch proofs pass. The boundary proof
also exposed and repaired exact APIRequest native-class feature registries, a
runtime `sw_vers` subprocess, and non-awaited Page-binding acknowledgement.
The merged Core candidate ledger remains red and incomplete at 1,216 expected,
one skipped, 31 unexpected and 47,229 not run.

NODE-226 extends the existing optional `init-scripts` native leaf from
Page-only registration to typed Page or BrowserContext scope. The Node facade
adds `browserContext.addInitScript` as source normalization plus one native
call; the Rust store, current-session fanout, future Page/OOPIF replay and
rollback remain shared with Page and Python. The package contract suite now
passes 269/269 with strict TypeScript. The pinned Python async/sync files pass
18/18 candidate and 18/18 oracle; this does not change the incomplete Core
ledger status.

Client-certificate FFI checkpoint (NODE-223, 2026-09-27): the optional
`api-request-client-certificates` profile accepts Playwright-shaped PEM or PFX
material in Node, then delegates origin matching, lazy decoding, TLS identity
selection and caching to bounded Rust leaves. Malformed public shapes are
rejected before artifact selection. The exact official validation row passes
3/3 and seven direct-request behaviors pass 21/21 in both candidate and oracle
lanes. Candidate request report SHA-256 is
`10a549ed1ff3094493189a926b64e35ae5c770c046fa0e1acb83494803109514`;
oracle SHA-256 is
`0aac6b69dedc4e56c6f63131ac10f7a07b9a71f0cceccbd762c8cec73cb15844`.
The nine request crates pass 96/96 release tests, Node passes 268/268, strict
types and targeted Clippy pass, and a real Tailscale-bound PEM/PFX mutual-TLS
smoke passes. The cumulative Core candidate ledger is 1,214 expected / one
skipped / 31 unexpected / 47,231 not-run, SHA-256
`b9d7b48117f08def6811425eca735414ba803c506d6fa39cdb28ebf2c2e60c92`.
Proxy, browser-context and same-origin multi-identity behavior remain
open; this is not full upstream conformance.

API-request FFI checkpoint (NODE-221, 2026-09-27): the optional native profile
projects a Rust-owned request context, responses, structured cookies and
bounded resolver addresses. Node performs public shape conversion and awaited
storage-state file I/O only; it contains no HTTP client, TLS implementation,
cookie matching, retry loop or browser operation. Origin state is retained in
the native context under an 8 MiB cap, cookie state stays in the bounded Rust
store, and at most 64 resolver IPs cross per request without replacing URL
Host/SNI. Complete unchanged official Library matrices pass candidate/oracle
198/198 `global-fetch` and 60/60 cookie; Bidi matrices pass 396/396 and
120/120. Rust request crates at this checkpoint passed 91/91 release tests, the
Node package passed 266/266,
strict types/targeted Clippy pass, and a real native seven-verb smoke passes.
This did not cover proxy, BrowserContext sharing or the rest of the official
corpora.

Upstream-conformance boolean-diagnostic source checkpoint (NODE-211,
2026-09-27): Rust now preserves the last valid state received during a boolean
expectation and produces the bounded element preview used by official matcher
call logs. The preview path first performs an immediate count, then at most one
Rust-selected static evaluation while time remains inside the original public
deadline; it retains no DOM handle, Promise, timer, queue or browser global.
The addon returns `received`, `preview`, `missing` and `timedOut`; Node only
maps those bounded values to the public `Expect`, `waiting for`, `locator
resolved to` and `unexpected value` lines. Package 257/257, focused Rust
expectation 12/12, strict TypeScript, selected native-load and selected-profile
Clippy pass. No official browser report or ledger was changed because the
empty-browser guard correctly refuses to disturb the restored unrelated page.
The existing 62/24 candidate result therefore still predates the guard and
this diagnostic source change; it is not a rerun or final isolation proof.

Upstream-conformance boolean-expectation checkpoint (2026-09-27): the official
oracle preload keeps upstream Playwright as the implementation and replaces
only its launch fixture with upstream `connectOverCDP` to the same dedicated
Chrome. The complete unchanged `page/expect-boolean.spec.ts` oracle file passes
86/86, report SHA-256
`c7ee31f961819363811fb80669a11f3d69f9cfcaa98ca75543fa998a8dfea0cf`.
The candidate baseline was 37 expected / 49 unexpected; after adding
Rust-owned checked/indeterminate, editable/readonly, enabled/disabled and
focused expectations it is 62 expected / 24 unexpected, report SHA-256
`e9bfbcd0a4cb8dfc95990a27ea52d9cd14f13b89016c076d666536ee72fe3b5d`.
Checked, editable and enabled reuse the existing typed Rust Locator read
capabilities. Indeterminate and focused use bounded Rust-selected browser
programs in the resolved element's world. Node validates option shapes and
delegates; it owns no DOM probe, polling timer or retry loop. At that
checkpoint, the independent oracle ledger was 86 expected / 48,391 not-run, SHA-256
`cd2115bfce1adc903a36fb1f5ffb53d147fc28787c014fec1a9644692e74fdc2`.
The candidate ledger was 416 expected / one explicit skip / 31 unexpected /
48,029 not-run, SHA-256
`94b0870a0b785ecdc527e481172dc23d7a1ae3fb40f859a04ae2d26edded606a`.
The run exposed unsafe coexistence with an unrelated nonblank page in the
dedicated Chrome: fixture cleanup retired it, and its exact recorded URL was
restored. The runner now refuses runtime execution when `/json/list` contains
any nonblank page. The 62/24 report predates that guard and therefore remains
progress evidence, not final isolation evidence. Remaining failures are rich
expectation diagnostics, 1 ms first-probe behavior, one FrameLocator lifecycle
case, CSS syntax wording and missing `page.request`/`APIResponse.toBeOK`.
Full upstream conformance remains incomplete.

Upstream-conformance actionability-progress checkpoint (2026-09-27): the
runtime keeps the original single command deadline. When that deadline wins,
it first drops the operation and completes the existing typed cancellation and
browser cleanup. Only successful cleanup may invoke a synchronous/no-I/O,
default-noop terminal-error projection. The exact Rust Locator service retains
one fixed enum containing the last observed missing, detached, invisible,
unstable, disabled or obscured state. It clears the slot at operation start,
ready/runtime-replaced state and normal completion, then consumes it at most
once after deadline cancellation. Node neither probes the DOM nor starts a
timer or retry loop; it only maps fixed bounded keys to the public Playwright
message. The complete unchanged `page-click-timeout-2.spec.ts` file passes 2/2,
report SHA-256
`24082ff695e75d44e9779e335143283b74c85fa8c596097cc6e25b8419e57370`.
The final controlled unchanged 12-case Page batch records nine expected and
three unexpected results, report SHA-256
`87880edd0948895b22b6ce90e3448baf4cbbe495383e4cfe57f8649610558860`.
Its remaining discrepancies are client-redirect/network-idle lifecycle,
dialog events and the implementation-private `__testHookBeforePointerAction`.
The cumulative Core ledger is 354 expected / one explicit skip / seven
unexpected / 48,115 not-run, SHA-256
`579ccfc8f676c7c2230f9e01e255a963a14113bac2c87ed4b8621ba6ff2e479e`.
Package 256/256, runtime 114/114, Locator backend 115/115, Chromium composition
397/16, selected host 617/14 plus integration 22/8, strict
TypeScript/native-load and selected host/addon Clippy pass. This is measured
progress, not a full-suite pass claim.

Upstream-conformance Locator-geometry/highlight checkpoint (2026-09-27):
`locator-bounding-box` and `locator-highlight` are independent default-off Rust
leaves, with the higher-level highlight leaf explicitly composing bounding-box.
Rust resolves the strict Locator in its existing Frame evaluation owner,
rejects disconnected, invisible or empty geometry, and returns a bounded typed
box. Highlight installs one bounded overlay plus animation-frame owner in the
selected Frame utility world; another highlight replaces that state and world
teardown clears it on navigation. The official under-test environment receives
the inspectable open shadow root required by Playwright's assertion, while
ordinary callers receive a closed root. Node supplies only the bounded public
description, TypeScript result shape and final error presentation. This slice
does not claim perfect cross-origin Frame offset support or continuous selector
retargeting after the initial strict resolution. The complete unchanged
`locator-highlight.spec.ts` file passes 1/1, report SHA-256
`3e5b15bf7242fd3457978739932f0f243d74a007ba26ad48343207310457dec6`.
The controlled unchanged 12-case Page batch progresses from seven expected /
five unexpected to eight expected / four unexpected, report SHA-256
`72d08fa3d73c9e17f5f6e7157e2313b6c7d53ffc6c3ac534321d68e64d168643`.
The cumulative Core ledger is 351 expected / one explicit skip / 10 unexpected
/ 48,115 not-run, SHA-256
`d95617557ce968078630912a613502516b185af752d88866702b4b3e24e28d8f`.
Package 256/256, strict TypeScript/native-load, selected host 616/14 plus
integration 22/8, and strict selected host/addon Clippy pass. The four remaining
batch discrepancies are client-redirect continuation, dialog events, one
private pointer-action hook and one rich actionability retry log. Full upstream
conformance remains incomplete.

Upstream-conformance Locator-expectation checkpoint (2026-09-27): Rust now
owns serial, deadline-bounded Locator expectations for attached/detached/empty
state, deep property values and browser-compatible regular-expression text.
Every attempt reuses the existing Locator resolution/evaluation owner and has
a fixed 100 ms ceiling; navigation-context replacement is retried only while
the selected Frame remains active. Missing-element semantics are explicit:
state expectations follow Playwright polarity, while neither polarity of text
or property matching can complete just because the element is absent. Node
retains only the public Locator description, bounded expected-value conversion
and final error formatting; it adds no browser decision, timer, queue, task or
persistent table. The complete unchanged `matchers.misc.spec.ts` progresses
through 1/2, 1/2 and 2/2; the passing report SHA-256 is
`d36423fbd9755b7c4d9ff912c921994407525aad31ae47ea8c7fd48f2b2884ed`.
The controlled unchanged 12-case Page batch progresses from five expected /
seven unexpected to seven expected / five unexpected, report SHA-256
`89a8344a244e83bb0721b9701d595fa690b23fdee5b3bae8cdc6407fa060bb1f`.
The cumulative Core ledger is 350 expected / one explicit skip / 11 unexpected
/ 48,115 not-run, SHA-256
`1f21303158ae74bc7a9babff110ce4593f287a79552faac215ec203d26e6ba73`.
Package 254/254, strict TypeScript/native-load, selected host 614/14 plus
integration 22/8, and strict selected host/addon Clippy pass. The five remaining
batch discrepancies are client-redirect continuation, Locator highlight,
dialog events, one private pointer-action hook and one rich actionability retry
log. Full upstream conformance remains incomplete.

Upstream-conformance response-body checkpoint (2026-09-27): the independently
selectable `response-body` feature retains the completed navigation document's
bounded native request identity in Rust. The host resolves the exact Page and
Frame, issues Chromium's `Network.getResponseBody`, decodes bounded base64 and
owns the bytes through N-API settlement. Node supplies only Playwright language
semantics and one cached copied Buffer shared by `Response.body()`, `text()` and
`json()`; it chooses neither protocol nor browser route. The complete unchanged
`frame-goto.spec.ts` file progresses from 2/4 to 3/4, report SHA-256
`5c7b5ced18dfffdaf20b7e9b5b114b408d23169c5fe339e0dbec3d200876ee30`.
The controlled unchanged 12-case Page batch progresses from 4 expected / eight
unexpected to five expected / seven unexpected, report SHA-256
`c8f221f387a54bb17aeeb7bec6770f0db4e1ca7c748ea0c4d902f81f2fad11d5`.
The cumulative Core ledger is 348 expected / one explicit skip / 13 unexpected
/ 48,115 not-run, SHA-256
`f226ec1621da2579822e28ef9b470ff95a8a7c2eb0d2a5e1207654c55f85a21a`.
Package 249/249, strict TypeScript/native-load, selected host 609/14 plus
integration 22/8, and strict selected host/addon Clippy pass. The remaining
batch discrepancies are client-redirect continuation, Locator highlight,
Locator empty/attached matcher behavior, dialog events, one private pointer
hook and one rich actionability retry log. Full upstream conformance remains
incomplete.

Upstream-conformance actionability and detached-Frame checkpoint (2026-09-27):
the same controlled 12-case Page batch progressed from 3/9 to 4/8 without
weakening upstream assertions. Rust click actionability now delegates enabled
state to the already installed shared selector runtime. That runtime retargets
a text-matched descendant to its interactive ancestor for enabled/disabled
semantics while the action backend keeps the original element for stability,
geometry and hit testing. The disabled target therefore remains unclicked and
times out; the remaining mismatch is the missing bounded actionability retry
log. Separately, Rust host error projection preserves `FrameDetached` as a
specific `PWRIGHT_TARGET_CLOSED` message instead of collapsing it to generic
Page closure. The complete `frame-goto.spec.ts` file progresses from 1/4 to
2/4. Node owns neither decision. The cumulative Core ledger is 347 expected /
one explicit skip / 14 unexpected / 48,115 not-run, SHA-256
`79cc1dfaadc8e493256539c17103c83251c9a48f6632526a05ea487bd202df37`.
Package 246/246, strict TypeScript/native-load, Rust Locator 114/114, selected
host 607/14 plus integration 22/8, and strict selected-profile Clippy pass.
Full upstream conformance remains incomplete.

Upstream-conformance Page-action facade checkpoint (2026-09-27): Page and
Frame selector actions now pass `page.*` or `frame.*` identity into the same
thin Locator adapter; Locator calls retain `locator.*`. Standard Rust timeout
errors receive Playwright's final punctuation in the language error boundary.
There is still one Rust action owner and no Page-specific browser operation,
timer, state table or retry loop. The complete unchanged
`page-click-timeout-2.spec.ts` file remains 0/2 after progressing beyond both
API-prefix assertions: Rust does not yet retain and publish the last bounded
actionability retry reason, so the official visible/enabled/stable action log
is absent. The complete one-case `page-click-during-navigation.spec.ts` is also
an explicit unexpected result because it uses Playwright's private
`__testHookAfterStable` server option. The cumulative Core ledger is 343
expected / one explicit skip / six unexpected / 48,127 not-run, SHA-256
`967f4dcb53128223f582793f677781106de17898991b96cb8e7c53f844b69d56`.
Full upstream conformance remains incomplete.

Upstream-conformance navigation checkpoint (2026-09-27): the complete
unchanged `page-navigation.spec.ts` file progressed 0/3, 0/3, 0/3, 2/3 and
3/3. Quoted Playwright text selectors and bounded `>>` chains now compile into
typed Rust selector recipes instead of being handed to CSS. Selector runtime
version 8 supplies Playwright-compatible descendant text and submit/button
input-value semantics inside the Rust-owned browser runtime. For a newly
attached paused popup, the selected topology profile polls caller Context
settings, lifecycle setup, `Page.enable`, recursive auto-attach and
`Runtime.runIfWaitingForDebugger` as one fixed five-owner batch. Commands are
queued before resume, every result remains a readiness fence, and the Page is
not published until the later Frame/runtime/world initialization also
completes. This adds no task, queue, cross-session scheduler, Node browser
owner, or package edge. The passing complete-file report SHA-256 is
`de9f28ea77db394e6b6d18f272e0fa81eda03ce2a0e00c22b61adc98b48d61b2`.
A later Page-facade regression run remains 3/3, SHA-256
`6c7049be945943dff9a3c8a0571d59282eebde7d641fc0230b6ebef60beeb144`.
The cumulative Core ledger is 343 expected / one explicit skip / three
unexpected / 48,130 not-run, SHA-256
`3d73808ac360304ed8cd67b364208bde344648752fa31d4c6ba66dcda645d0f3`.
The package passes 246/246 plus strict TypeScript and native-load. The selected
host passes 606 / 14 explicit ignores plus 22 / 8 integration, selector backend
112/112, Chromium composition 397 / 16 explicit ignores, and strict
selected-host/direct-crate Clippy.
Full upstream conformance remains incomplete.

Upstream-conformance request-GC checkpoint (2026-09-27): the complete
unchanged `page-request-gc.spec.ts` file progressed from 0/1 to 1/1. The
optional `request-gc` addon edge forwards into a zero-persistent-state Rust
host leaf. Rust admits the operation against the exact Page, resolves its
current Chromium session and sends `HeapProfiler.collectGarbage` with the
official empty payload. Node contributes only `Page.requestGC()` and no
protocol choice, queue, timer or state. The baseline and passing report
SHA-256 values are
`e16d57c318a3c12cb15770d0ff47ae20e9d642ecd2f97943aeacf8e094b5a35c` and
`18bce9f2941cb1b6be55d7101b9fe8cb71656d535f8e782a30a1f10f48449f88`.
The cumulative Core ledger is 340 expected / one explicit skip / three
unexpected / 48,133 not-run, SHA-256
`1311baaa537257b551c4380f5287463d817b27212c04e7c93fceeffff97f089a`.
The package suite passes 246/246; strict TypeScript, selected-profile strict
Clippy, release build and native-load pass. The full selected host release
suite records 605/14 in the library target plus 22/8 across integration
targets. Full upstream conformance remains incomplete.

Upstream-conformance URL-wait checkpoint (2026-09-27): the complete unchanged
`page-wait-for-url.spec.ts` file progressed from 0/11 to 11/11. The optional
`wait-for-url` addon edge reuses the existing Rust host URL-wait leaf. Rust owns
fixed eight-operation admission, published Page/Frame URL observation,
lifecycle ordering, timeout and cancellation; the wait now accepts the exact
child Frame identity instead of assuming the main Frame. A bounded N-API
thread-safe callback is only the language-semantic port. Node compiles
Rust-prepared string globs into JavaScript `RegExp`, preserves user RegExp
state, and passes a real `URL` to predicates; it does not own a poll loop,
browser Promise, timer or Frame registry. The baseline and passing report
SHA-256 values are
`82171341c8fe4a00f7064bd6c5504fcbd6d0025bc30bf1d630a75e13ac736136` and
`43a5dafb0d6642cffb61061b17a9c01c3640118ac774753838b439ace300f523`.
The cumulative Core ledger at that checkpoint was 339 expected / one explicit
skip / three unexpected / 48,134 not-run, SHA-256
`7492f7e7680c8a4427622a2759deb48d1329464d54afdc0eba0976e05d4b2c17`.
The package suite passed 244/244, and the selected host release library suite
recorded 604/14. Strict TypeScript, selected-profile strict Clippy, release
build and native-load pass. Full upstream conformance remains incomplete.

Upstream-conformance Locator/ElementHandle checkpoint (2026-09-27): the
complete unchanged `locator-evaluate.spec.ts`,
`elementhandle-eval-on-selector.spec.ts`, `locator-is-visible.spec.ts`,
`locator-click.spec.ts`, `elementhandle-type.spec.ts`, and
`elementhandle-press.spec.ts` Chromium Page files pass 6/6, 6/6, 8/8, 5/5,
5/5, and 5/5. `Locator.evaluateAll` is a dedicated
bounded Rust collection-evaluation lane: it resolves the immutable selector in
the exact utility world and passes one exact match array to the user function.
Node supplies only the bounded function/argument conversion. ElementHandle
`$eval`/`$$eval` continue through retained Rust Handle ownership; the latter
exposes a real Array without using mutable Array methods to assemble it.

`Locator.waitFor`, visibility reads, and visibility expectation polling now
delegate to typed Rust operations. Playwright treats replaceable execution-
context loss inside `isVisibleInternal` as not visible while preserving
non-retriable selector errors; the Rust host matches that distinction. The
bounded selector recipe also retains an exact unknown-engine issue before CDP,
so `hey=what` reports Playwright's required diagnostic instead of a browser CSS
syntax error. Locator click, Page/Frame type, key press, and Page/Frame focus
remain thin facades over typed Rust actions.

The complete unchanged `page-focus.spec.ts` progression is 5 pass / one
upstream skip / four fail, 8 / 1 / 1, then 9 / 1 / 0. The last mismatch was an
ordering issue: DOM focus returned before its binding callback became visible
in Node. Rust now retains an acknowledgment bit in the existing fixed 64-call
binding table. After the action's topology fence, it snapshots a call watermark
and waits, with cancellation and no new unbounded queue, until Node acknowledges
that each callback has been invoked. A callback Promise may remain unresolved;
the fence never waits for its result or for result delivery back into the page.
The final report SHA-256 is
`27c9e58d1269159714dba66985032ba84f9cbaec44a3a3f7bbccb115aa2e9a86`.
The cumulative 48,477-record Core ledger is 312 expected / one explicit skip /
three unexpected / 48,161 not-run, SHA-256
`194ab114801c33ff92860ae8804818453acba18064be55f8c54695bf97860b64`.
The package suite passes 239/239; strict TypeScript, selected-profile strict
Clippy and native-load pass. The selected host release suite records 568 passed
/ 14 explicit ignores in the library target plus 22 passed / 7 explicit ignores
in integration targets. Full upstream conformance remains incomplete.

The complete unchanged `page-emulate-media.spec.ts` file progressed from 0/11
to 11/11 after exposing the existing independent Rust `emulate-media` leaf.
Node owns only Playwright option validation and omitted-versus-null adaptation;
Rust retains a fixed 64-Page override table, serializes operations, and sends
the exact bounded CDP emulation command. Optional Page N-API methods now live in
feature-level impl blocks, which makes a standalone `emulate-media` selection
compile without exporting init-script, set-content, or binding methods. The
existing independent `extra-headers` path was composed next, and the complete
unchanged `page-set-extra-http-headers.spec.ts` file passes 5/5. The two report
SHA-256 values are
`cd3ee047a67816c8209e414232299a157a95596f0028d10afcdcced243fbe4da` and
`430901f0b526c2a9e4ecafbcae1ac831d9a900b7f63b5dcbf4120dfc068f0751`.
The cumulative Core ledger is 328 expected / one explicit skip / three
unexpected / 48,145 not-run, SHA-256
`7e6f42140c8de2f314cf1061e9da5f7cca93db0fc341cbb0180fe5b26ad99933`.
The package suite passes 240/240, and the selected host release suite records
603/14 in the library target plus 22/8 across integration targets. Strict
TypeScript, strict selected-profile Clippy and native-load pass. Full upstream
conformance remains incomplete.

Upstream-conformance Page-evaluation checkpoint (2026-09-27): the complete
unchanged `tests/page/page-evaluate.spec.ts` Chromium Page project now passes
96/96, and `page-evaluate-handle.spec.ts` passes 11/11. The optional
`large-evaluation-results` feature keeps ordinary envelopes in the initial CDP
reply using a bounded serializer that does not call the page's mutable JSON or
Array push/map hooks. Only a top-level string above 256 KiB remains in a
caller-owned CDP object group and is copied through fixed 256K UTF-16-unit
projections, with one active large transfer and a 128 MiB result ceiling.
This restores evaluation-during-navigation semantics while passing the official
100 MiB transfer case. The FFI ticket adds its 400 MiB conservative accounting
allowance only after Rust sees a large string; ordinary/nested calls keep the
existing 8 MiB allowance. A direct 100 MiB smoke measured about 259.7 MiB
maximum process RSS and 219.5 MiB peak physical footprint with zero swap; that
measurement is process-level and not a claim of unique per-session RAM.

The official report SHA-256 is
`ff3777120fe00703ab86bd5840256bf78dbad60a21bfd25e177b8b16adb91e34`.
The cumulative 48,477-record Core ledger is 233 expected / one unexpected /
48,243 not-run, SHA-256
`0ff3cb44647397495e794b165b10759133e730fb39c8fa7d1c556e24de27c4d5`.
The local gates pass at 223/223 package contracts, strict TypeScript,
selected-profile strict Clippy, native load, 13/13 backend evaluation AAA and
14/14 FFI-budget AAA. Full upstream conformance remains incomplete.

Upstream-conformance Page-binding checkpoint (2026-09-26): the optional
`page-bindings` feature now composes frame lifecycle with the existing Handle
owner rather than implementing an independent Node transport. Rust installs
and versions one CDP binding, filters foreign binding events before parsing,
owns a fixed 64-registration/64-pending-call queue, and injects the bounded
browser-side argument serializer. The serializer covers cycles, special values,
typed arrays and Window/Document/Node aliases without depending on mutable
Array `map` or `push`; malformed `Array.prototype.toJSON` remains visible at
the Node boundary with Playwright's diagnostic.

For value callbacks, Node only reconstructs the tagged arguments and calls the
user function. The callback result is passed as the original Node value into
the existing deferred Rust evaluation converter; cycles, Errors and embedded
JSHandles therefore use the same bounded Rust codec and owner validation as
ordinary evaluation. For `{ handle: true }`, Rust re-enters the exact Frame,
takes the controller-retained value as a native owned Handle, and publishes the
normal JSHandle/ElementHandle wrapper. Result delivery consumes a one-shot
pending-call permit and is navigation-race tolerant. No raw CDP object ID or
unbounded callback table is exposed to Node.

The first complete unchanged upstream `page-expose-function.spec.ts` run was
21/24. All binding-specific cases passed; the three failures identified absent
`page.addInitScript`, `page.waitForNavigation`, and `page.click` entry points.
The follow-up `official-conformance-page-bindings` profile composes the existing
binding/Handle leaf with `frame-navigation` and a new independent
`init-scripts` feature. Rust owns fixed init-script storage/session fanout and a
receipt-free first-navigation waiter; Page/Frame click delegates to the
existing Rust Locator action. The next-navigation registration is now made
synchronously during N-API admission, before the returned Future can yield, so
an immediately triggered navigation cannot cross an asynchronous preparation
gap. Local evidence is 219/219 package tests, strict
TypeScript, focused Rust lifecycle/init-script tests, strict Clippy, native
artifact loading, and exported method checks. The next complete run was 23/24:
click and callback dispatch worked, but the published ElementHandle lacked
`innerText`. That method is now a thin facade over the retained Rust Handle
evaluation port, and the third unchanged run passes 24/24. A separate teardown
probe still reports two retained native leases, so lifecycle cleanup remains an
open gate despite the functional pass. The unchanged
`page-add-init-script.spec.ts` file progressed from 8/9 to 9/9 with Page/Frame
script-tag as a derived combinator over Rust-owned Frame evaluation and retained
Handle publication. The complete script-tag file progressed from 9/12 to 10/12
after inline-module publication and CSP rejection were corrected, then reached
12/12 after composing `page.waitForFunction`. The new independent
`wait-for-function` native leaf owns eight resource-accounted operations,
deadline/cancellation and exact truthy Handle publication. Rust performs
one-shot Frame evaluations, probes each returned Handle and disposes falsy
attempts, so timeout/cancellation leaves no continuing browser Promise. The
pinned 26-test wait-for-function file additionally requires `page.$`,
`waitForTimeout`, and `setDefaultTimeout`. The query is a facade over the
existing Rust Frame/Handle owner. Timeout waiting is a new independently
selected, fixed-16-operation Rust leaf. Default timeout inheritance reuses the
existing fixed Context/Page Rust table and resolves an omitted wait timeout in
native code. Four complete unchanged browser runs progressed 22/26, 24/26,
25/26, then 26/26. The final fixes retry one-shot evaluation after navigation
context replacement and implement `ElementHandle.contentFrame` by directly
describing the exact retained element, mapping its bounded frame id through the
existing topology owner, and creating no temporary Handle. Local evidence is
6/6 wait-function Rust AAA, 1/1 timer Rust AAA, 2/2 timeout-settings Rust AAA,
219/219 package tests, strict TypeScript/Clippy, release build and native-load.
The final native artifact SHA-256 is
`46768820b4ae1c16458592d371c20b3dc6a8407d54d70229534a573e6247b830`.

The next complete-file slice moves console events from 9/13 to 13/13 and Page
load from 0/2 through 1/2 to 2/2. The Page-event observer keeps a fixed
16-entry, 128-byte-loader document table, seeds the initial document, deduplicates
each DOM/load phase, and prunes retired Pages. `FrameLocator` retains only a
lazy language recipe: native wait-for-function selects the iframe, exact
`contentFrame` resolves the existing Frame, and the owner Handle is disposed.
Text matching polls the immutable Locator recipe in Rust with one deadline;
Node only decodes the official matcher options. `Locator.all` derives at most
1024 native `nth` recipes, and `div >> p` becomes one admitted Rust `Within`
tree. The resulting complete files are load 2/2 and locator-list 1/1. Handle
evaluation 2/2, asElement 4/4 and jsonValue 3/3 pass unchanged; Handle preview
and sparse-array console formatting remain recorded as 0/6 in
`jshandle-to-string.spec.ts`.

Quality-hardening checkpoint (2026-09-21): native artifacts now export a
numeric addon ABI version, and the package requires exact ABI 1 plus the core
`NativeEnvironment` constructor before validating the selected feature markers.
An empty/default-looking or stale addon therefore fails closed. A real release
artifact smoke runs in an isolated child process so native runtime teardown or
a stalled macOS load cannot hold the package test process open.

The session watch now handles both synchronous `wait()` throws and rejected
wait Promises. It records the primary failure, closes the session, retains a
cleanup failure in an `AggregateError`, and publishes one local disconnect
even if a faulty artifact refuses terminal publication. Page, Frame and
JSHandle expression ingress now matches Locator's runtime stringification for
non-function values. At the transport boundary, a zero connection command
timeout maps to no response timer, removing the old hidden 30-second cap from
`connectOverCDP({ timeout: 0 })`.

The CI definition now covers package unit/type behavior on Node 20/22/24,
dependency and real default-addon loading on Node 20, explicit default addon
unit tests, and terminal retained-payload standalone/combined selections. This
is CI configuration evidence, not a claim that an unpushed workflow run passed.

NODE-191 adds `prepareMutatedTarget` after `prepareImportedTarget` under the
same `progress.race`. `handleMutateSerializedFileInputPayloads` proves the exact
NativeSession and `PayloadImported` ticket before it takes either owner, moves
the Rust graph and validated target into the host mutation, and parks the same
typed FFI admission while the async work runs. No target ID, world, group,
payload graph or policy flag can be replaced by JavaScript at this boundary.

After mutation and native group release settle, the callback rechecks the same
ticket, consumes the typed completion and uses a success-only terminal commit.
The registry entry, zero-root container, graph allowance and 1 MiB FFI permit
therefore disappear immediately; `abort()` correctly returns false afterward.
Failure or revocation removes the exact entry while already-started Rust cleanup
continues through its reserved executor. Numeric serializer IDs arrive through
the N-API JSON boundary as either integers or integral doubles, so the narrow
validator accepts only finite positive integral values within the fixed
64-item ID range.

This adds no generic evaluator or public method. Node still owns JavaScript
argument semantics, the single deadline and error facade; Rust owns target,
transport, browser mutation, settlement and resource lifetime. [Canonical
NODE-191 checkpoint](knowledge/pwright-node-handles.md#node-191-terminal-rust-owned-serialized-payload-mutation).

The differential harness reaches pinned Playwright's in-process server only in
tests to install an `onBeforeInputAction` listener. This gives oracle and
candidate the same deterministic post-prevalidation pause for timeout,
late-type and lifecycle races without adding a production hook. It exposes one
settlement difference: Playwright resolves after main-document navigation at
that pause, while the internal Pwright coordinator rejects fail-closed. The
future public facade must choose that policy and map its error explicitly.

NODE-190 adds `prepareImportedTarget` immediately after
`prepareSerializedTarget`. `handleImportSerializedFileInputPayloads` first
proves the exact session and `Serialized` stage, then asks the host for a
separately budgeted import admission before checking out the phase or reading
the graph. This ordering makes capacity failure retryable and prevents a
foreign session from cancelling the legitimate ticket. After admission, the
addon walks the package-owned graph synchronously, rechecks the ticket, commits
`PayloadImported` and replaces the eleven-root container with an empty one.

The resulting `RetainedSerializedPayload` is Rust-only and move-once. It keeps
the existing validated target, call identity, Node deadline and parked 1 MiB
FFI admission unchanged; the new 96 MiB graph allowance is independent. The
generic importer is transport staging, not a CDP codec: no command, mutation,
event, response or public method crosses this boundary. [Canonical NODE-190
checkpoint](knowledge/pwright-node-handles.md#node-190-bounded-rust-ownership-for-the-serialized-payload-graph).

NODE-189: `prepareSerializedTarget` extends the selected coordinator after the
awaited input-action hook. The exact-session N-API gate invokes the pinned
synchronous Node language adapter on retained root six, rechecks reentrant
revocation, retains the result as root ten and commits `Serialized`. The Rust
browser core receives no bytes and issues no browser command yet; ordinary
base64 leaves remain shared V8 strings. Standalone and combined profiles add
only the addon phase flag. Generic live-handle adoption and the later
non-retargeting mutation/settlement leaf remain explicit gaps. [Canonical
NODE-189 checkpoint](knowledge/pwright-node-handles.md#node-189-late-call-argument-serialization-on-the-retained-owner).

NODE-188: `prepareInstrumentedTarget` extends the selected coordinator after
`preparePrevalidatedTarget`. Rust's begin gate returns the exact original
receiver root and moves only the record stage to `WaitingInputAction`. Node then
awaits the frozen internal `onBeforeInputAction` port through the already-active
progress deadline; the finish gate commits `Instrumented` only if the same
session, ticket and waiting stage still exist.

The port receives a restricted progress view containing `call@N` and `race`,
plus the original receiver and undefined point/box. The production default is
an awaited no-op, not a public listener API. Rejection and timeout follow the
existing abort path; there is no additional native dispatch, permit, root or
target adoption. This matches the pinned position before late serialization and
mutation while leaving those operations deliberately unimplemented.
[Canonical NODE-188 checkpoint](knowledge/pwright-node-handles.md#node-188-awaited-input-action-instrumentation-boundary).

NODE-187: `preparePrevalidatedTarget` composes projection with the new gated
`handlePrevalidateFileInputTarget` method. The method resumes the original
parked FFI preparation, consumes its Rust-only marked owner and runs the
existing DOM retarget/policy preflight under the already-running Node progress.
It cannot accept a replacement target ID or flags from JavaScript. Success
re-parks the same permit with an `OwnedValidatedMarkedFileInput` and commits
`Prevalidated`; its retargeted object ID never becomes an additional Node root.

The timer and timeout Error remain Node-owned, including ordinary event-loop
delivery while native prevalidation is awaited. Failure or abort drops the same
nested owner and reaches pre-admitted cleanup; no second FFI call or target
adoption is allowed. The raw internal error currently maps through the bounded
host backend surface. Instrumentation, late byte serialization, mutation and
the public Playwright error/API facade remain intentionally unavailable.
[Canonical NODE-187 checkpoint](knowledge/pwright-node-handles.md#node-187-same-owner-retained-dom-prevalidation).

NODE-186: The marker FFI completion now retains the concrete Rust marked owner
on success. Node consumes only the ordered mark outcome; the move-only owner
stays inside the same parked result while `projectAfterMarking` reads the raw
carrier through the actual language projection ports. Projection performs no
byte copy or await, rechecks the exact ticket after user-observable operations,
roots the original six values plus four raw projected values and stores two
native booleans before committing `Projected`.

The one Node timer remains scoped around this coordinator, but synchronous
projection still has ordinary event-loop semantics and cannot be preempted by
the timer. Dropping or aborting the ticket submits the already-reserved cleanup;
there is no second FFI call or fresh target adoption. Retained DOM prevalidation
does not yet consume this owner, so the public method remains unavailable.
[Canonical NODE-186 checkpoint](knowledge/pwright-node-handles.md#node-186-retained-marked-owner-and-same-ticket-carrier-projection).

NODE-185: Selected preparation now has a session-scoped public-call identity and
one Node-owned deadline. Server validation completes first; Rust allocates and
stores `call@N`; Node awaits the instrumentation checkpoint; Rust rechecks the
exact source and returns the raw validated timeout; one captured Node timer then
races materialization and native marking. The same FFI ticket is used throughout,
and native marking derives the retained ID instead of accepting one from JS.

The deadline callback uses `abortDeadline` to revoke the exact ticket, but Node
still owns the timeout Error and event-loop timing. Rust does not infer elapsed
time while synchronous JS blocks, start another clock or transport a typed
timeout reason today. The internal composed helper reaches Marked; the raw helper
can still stop at Materialized. Public projection/prevalidation/instrumentation/
serialization/mutation and `ElementHandle.setInputFiles` remain gated. Evidence
and red-run qualifications are canonical.
[Canonical NODE-185 checkpoint](knowledge/pwright-node-handles.md#node-185-session-call-identity-and-one-node-deadline).

NODE-184: The selected addon now carries typed native progress in the initial
preparation admission. At `Materialized`, exact-session checkout installs a weak
running revoker, registers the source, upgrades the original Handle only in the
Rust worker and performs host target marking. The worker's outer FFI result stays
successful so its inner typed outcome and publication fence can be parked on the
same ticket. Node reinstalls that parked owner before lifecycle callbacks,
rechecks after reentry and commits `Marked` only after publication and success.

The new standalone/combined artifact profiles are explicit; earlier profiles
remain feature-identical. This is not yet public `setInputFiles`: connection-wide
call identity, one deadline begun before materialization, Node timer delivery,
projection, prevalidation, instrumentation, late serialization and mutation are
still separate. Exact ownership and real-Chrome evidence are in the
[canonical NODE-184 checkpoint](knowledge/pwright-node-handles.md#node-184-same-admission-node-target-marking-handoff).

NODE-183: An explicit default-off Rust host `retained-target-marking` feature
now composes native original-Handle marking with a persistent exact-world
cache. Same-world calls share private marker state; different utility worlds
retain separate native ownership through retirement. Persistent owners keep
only independently admitted utility snapshots/receipts, not source Handles or
running operation tokens. Temporary adoption and marking keep their own call
ownership until cleanup. Route currentness is not native destruction evidence.

This adds no addon feature, Node artifact selection, preparation phase or public
retained upload. The Rust phase accepts the supplied call ID and starts no new
deadline; eventual Node promotion still needs original-target Weak upgrade,
exact-session revocation and typed same-admission progress. Preserve one deadline
originating before materialization and its Node event-loop delivery, then marking
before carrier projection, DOM prevalidation, instrumentation and late utility
serialization. Selected host/composition tests and strict Clippy pass; the same
assembled Chrome case passes both final host selections with all resources
retired. This is not a Node artifact rebuild or public handoff verification.
Default/older retained regressions, dependency/boundary and formatting gates
also pass. Exact evidence belongs to the
[canonical NODE-183 checkpoint](knowledge/pwright-node-handles.md#node-183-host-owned-utility-world-marker-cache).
Earlier checkpoints retain their historical scope and verification status.

NODE-182: Rust now provides independently selectable persistent exact-world
target marking and cancellation-safe temporary target ownership. Real Chrome
validates its DOM events and cleanup, but no host/addon profile selects this
feature. The actual Node preparation still stops at Materialized with a unit
FFI result. Next integration must bind a bounded world-state owner, upgrade the
original target at its proper boundary and park/resume typed progress on the
same admission while preserving revocation and publication ownership.

The deadline begins before materialization, but pinned Playwright's timer is
delivered by the Node event loop and observed through explicit progress races.
Immediate Rust expiry during blocking synchronous JavaScript can reorder errors.
Preserve that scheduling boundary alongside connection-wide call identity;
neither a new native-entry timeout nor a preparation-only counter is equivalent.
Mark original target before carrier reads, retargeting and late serialization.
Wrapper/storage quotas do not bound DOM retention by persistent native Sets.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-182-persistent-native-target-marking).

NODE-181: Preparation now binds the actual supplied NativeHandle through an
inert Weak reference, carried through all records and snapshots without a strong
pin, upgrade, disposal gate or observable JS read. This closes the original
wrapper capture gap, not native execution. Roots, capacity, features and budgets
are unchanged. The actual 16 KiB registry-layout check includes the added fields;
the same record/snapshot overlap sum is compile-checked against the existing
1 MiB entry allowance and independent 16 KiB synchronous phase scratch, so
reentrant entry revocation cannot invalidate the temporary-storage proof.

The ticket still stores a unit FFI result and stops at Materialized. No native
phase, marker, public upload or later Weak/GC behavior is proved. Final frozen-
source runs pass twenty distinct actual binding cases in each of standalone,
combined and standalone repeat: sixty passes, owned cleanup fully retired and
no failures/cancellations/skips. The exercised boundary is preparation through
Materialized plus abort. Exact offline, Rust, strict Clippy, dependency and
test-only watch-sampling evidence belongs to the canonical checkpoint.
Next design must place the operation deadline before materialization,
use connection-wide protocol IDs and own persistent exact-world remote-marker
state before implementing typed progress and same-admission native handoff.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-181-original-native-wrapper-binding).

NODE-180: HostSession now registers/checks move-only weak FFI receipts against
exact session and source metadata under its admission state lock. Eight native
invocation hooks share an additional selected-only 4 KiB fixed registry lease;
this is not the 32-entry language preparation registry. No package or feature
is added. Disposal/dispatcher/world errors are not moved into this provenance
gate, and registration grants no native-use authority.

Shutdown cancels outside locks, crosses the state admission barrier, and seals
hooks before lock-free revocation and both operation/Handle waits. Handle owner
tracker wakeups likewise occur outside the owner lock. The host registration
and close hooks are complete, but the addon must still attach one registration
at its first native handoff and retain it through park/resume/cleanup. Current
initial preparations remain unregistered, with no typed native progress,
original NativeHandle weak binding, marking or public ElementHandle upload.
Assembled and browser gates remain open; exact checks and pending dependency
verification belong to the canonical record.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-180-session-bound-revocation-before-native-drains).

NODE-179: Selected Rust FFI results can park and resume with their original
typed entry/permit, not a second admission or result retyping. Parking removes
queued counts and the completed future's size lease; base bytes and the native
result remain revocable without a running token. Resume validates exact budget
identity and registers work under the shutdown lock before extraction. Keep the
returned result and execution together through immediate future admission/run;
pure parked inspection must not call NAPI or expose native-use authority.

The raw carrier diagnostic reads filePayloads/localPaths/localDirectory before
the short-circuit multiplicity expression, preserving raw falsy results and
language coercion. It adds no production ticket phase or marking receipt.
`NativeFileInputPreparation` remains at Materialized with the unit-result
preparation; public ElementHandle.setInputFiles and NODE-178 selection are still
unwired. A session-bound weak revoker must run before native owner waits, even
without a Node callback. Persistent utility marker state needs its own ownership
and budget; temporary group release is insufficient. The eventual operation
deadline starts before materialization, and late utility serialization follows
native prevalidation and instrumentation. Offline kernel/ownership tests do not
establish actual-ticket, marking or browser parity.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-179-same-admission-ffi-handoff-and-raw-carrier-projection).

NODE-178: Rust now has a standalone opt-in native precheck owner in the existing
file-input leaves. It preserves the exact retargeted input and settlement-owned
group across cancellation, but is not selected by the host/addon and exposes no
new Node API. Future selection must retain mark original target -> raw carrier
projection -> native precheck -> onBeforeInputAction -> late custom utility
serialization -> browser mutation. Marking is observable; raw falsy projection
and late getter behavior cannot be replaced by eager PreparedFileInput.
Use a single operation deadline and a cancellation-safe parked native owner
without holding an unstarted execution token across arbitrary JS waits.
Offline script comparisons are not native browser conformance.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-178-owned-native-file-input-prevalidation).

NODE-177: Rust gates ChannelReady to ServerDispatchReady and, after server
schema, ServerParamsReady to ServerMaterializationReady using exact owner-ticket
and creation-Page registration. World/Handle/Frame runtime liveness is not used
as a missing-dispatcher substitute. The JS continuation adds only setImmediate
delivery and an empty instrumentation await between checks. It resolves undefined
at internal Materialized; no ticket then assimilation or public upload success
is implied. Foreign direct-gate rejection leaves that ticket alone; the driver
aborts failures it owns. No roots, dependencies, classes, budgets or native pins
are added. Full hooks, public error transport/deadlines and native submission
remain open. In particular, preserve mark target -> carrier projection -> DOM
checks/retarget -> onBeforeInputAction -> late utility serializer -> browser
atob/File; eager PreparedFileInput conversion would reorder observable behavior.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-177-deferred-registration-gates-before-materialization).

NODE-176: Add server-direction schema and raw payload materialization through
primitive language ports. Preserve custom serializer returns, own-data output
properties and outer-only async assimilation. These manual ticket phases do not
implement the dispatcher target/deadline/error boundary or public upload.
[Canonical contract and current evidence](knowledge/pwright-node-handles.md#node-176-server-direction-validation-and-raw-payload-materialization).

NODE-175 (2026-09-17): Add selected synchronous Rust raw-buffer channel
validation after argument assembly, before serializer/materialization or native
execution. Visit payloads (name, optional mimeType, buffer), localDirectory,
directoryStream, localPaths, streams and timeout in schema order, including
optional-field behavior. Preserve observable map and its raw result, boxed
valueOf's raw result, inherited setters from ordinary assignment and original
Buffer identity. Do not introduce early byte copying, serialization or eager
numeric projection. Normal non-isUnderTest validator behavior is the contract.

Frozen JavaScript ports implement only language primitives; Rust selects
validation order and errors. Three no-capture/ZST native callback specializations
use static typed dispatch and receive current Env/path/ports, never retaining
scoped N-API values or preparation/native ownership. This avoids new Rust
callback-Box allocation and its construction-failure leak; N-API/V8 callback
backing and escaped JS graphs remain outside administrative RSS bounds.
Success replaces root six and advances
ArgumentsReady to ChannelReady without another await or increased root,
FFI/scratch, metadata, feature, dependency or class budgets. WritableStream
has no producer/brand: observe the first `_object` read, then fail closed with
the expected-channel error. Valid-stream/whole-protocol parity is not claimed.

Public API error prefix/stack, deferred server bytes/path preparation, float
deadlines, correctly typed move-only FfiPreparation promotion/native authority
and public ElementHandle.setInputFiles remain open. Offline verification is
recorded in the canonical checkpoint; actual-ticket/browser/public gates
remain open. Shared CDP is blocked without restart permission and no new
browser pass is claimed. [Canonical contract and exact verification](knowledge/pwright-node-handles.md)
own evidence; earlier entries retain their historical scope.

NODE-174 (2026-09-17): Production timeout association is now creation-Page
bound, superseding NODE-173's caller-selected settings gap. The selected
session's bounded index issues canonical Context/Page cells from published
membership, with exact Weak<Owners> provenance. The facade forwards creation
Page ID to `frameEvaluateHandleWithPageTimeout`, with no JS policy query/cache.
Rust captures the canonical lookup Result after FFI admission; source/argument
materialization still runs before any lookup error is raised before host work.
This avoids both pre-commit lifecycle queries and closed-Page metadata errors
skipping normal conversion. Rust verifies immutable source
creation-Page proof and propagates the binding through generic and Element
Handle producers, including property batches. Preparation captures bound
settings before callbacks; raw `assembleArguments(ports)` accepts no late
settings argument. Current DOM ownerFrame lookup does not select timeout policy.

The facade freezes raw PWDEBUG, npm_config_pwdebug and npm_package_config_pwdebug
values at module import. Rust performs undefined-only fallback and excludes
empty, `0`, `false` and `console` from inspector mode. Each selected connection
initializes once before Browser publication; failure closes the unpublished
session and preserves primary/cleanup errors. Old artifact forwarding remains.

Cells and fresh scoped bindings share the original 1024-slot pool, with bounded
session indexes bringing metadata to 784 KiB. Metadata owns no JS roots,
HostSession, native-use pin or task; immutable provenance can survive disposal
without authorizing browser work. No feature/profile/dependency/raw class is
added. Public setDefaultTimeout/arbitrary JS defaults remain absent, as do
complete channel/error phases, deferred bytes/paths, native deadline/promotion/
submission integration and public ElementHandle upload. [Canonical contract
and exact evidence](knowledge/pwright-node-handles.md#node-174-canonical-creation-page-timeout-binding)
own verification; no new browser pass is implied. Earlier entries are historical.

NODE-173 (2026-09-17): Rust now constructs arguments by spreading converted
input, spreading options, then resolving timeout. Frozen JavaScript ports
perform only CopyDataProperties and `options.timeout` access. No extra await
separates conversion assimilation from assembly. Raw
`NativeFileInputPreparation.assembleArguments(typedSettings, ports)` advances
Ready to ArgumentsReady, replacing the sixth root with params rather than
adding another root; existing preparation and synchronous-phase limits remain.

The Rust timeout policy follows normal Frame semantics: falsy options become
`{}`; a numeric `typeof` result triggers a second raw read, even if that result
is no longer numeric. Otherwise debug precedes the live local default, then
the parent repeats the options read, with a final 30000 ms fallback. Defaults
are read after getters so reentrant updates remain observable. Context roots
and Page children contain typed optional numeric defaults and inherited,
injected debug classification, not arbitrary JavaScript default state.

The selected environment publishes the no-public-constructor, module-local
`NativeFileInputTimeoutSettings` safely; selected class inventory becomes ten
without changing older profiles. A separate 528 KiB metadata budget owns no
JS roots, native authority, session or task. The caller currently selects the
creation Frame's settings: provenance does not automatically bind or validate
that association. Public setters, facade/default/debug configuration, arbitrary
custom settings trees, channel validation/error phases, deferred Buffer/path
work, native fractional/non-finite timeout mapping, promotion/submission and
public retained upload remain unimplemented. No feature/profile/package or
dependency is added. [Canonical contract and exact evidence](knowledge/pwright-node-handles.md#node-173-rust-owned-argument-assembly-and-live-timeout-settings)
own verification; offline kernels do not establish browser/API parity.
Earlier checkpoints below retain their historical scope.

NODE-172 (2026-09-17): Rust now orchestrates synchronous owner lookup,
context lookup and array/payload-size preflight through one-expression
JavaScript language ports. Observable member calls keep their receiver;
array methods, truthiness, addition and comparison remain JavaScript
operations, not numeric projections. The owner result is awaited first;
`page().context()` executes synchronously outside the separate async converter.
The converter returns the exact ordinary `{ payloads }` shape so inherited
`then` assimilation remains observable.

Weak phase guards reject overlapping/out-of-order phases with no N-API call
inside registry borrows. Pure `RootSnapshot` inspection clones weak source
metadata and `Rc<NodeReference>`; reference lookup occurs afterward and drops
the temporary Rc before user JavaScript. Initial `creation_phase` and later
phases pre-admit separate scratch before N-API allocation, then recheck liveness
after callbacks before allocating more; replacement rejects revocation. The
registry-wide limit is 32 synchronous phases at 16 KiB each, adding 512 KiB of
administrative allowance while leaving the original table, entry and shared
FFI budgets unchanged. Scratch holds no FFI Command/task or native authority;
it survives callback abort/finalization only until the raw synchronous phase
ends, not arbitrary Promise waits. The revocable array grows through 3/4/5/6
values under existing entry admission; closing or aborting can remove roots
while the weak scope/registry guard and scratch survive. Arbitrary V8 graphs,
scoped handles and opaque allocator backing remain outside byte-bound claims.
Selected stateless diagnostics invoke this same kernel
without ticket/session/native authority or persistent roots. Their differential
harness is not actual-ticket N-API, public API or browser verification.

Paths are classified but explicitly unsupported at the parent boundary. File
stat/read, deferred byte materialization, options/channel validation, promotion
and native submission remain open, as does public alignment of raw
closed/cancelled phase ordering. Public `ElementHandle.setInputFiles` stays
unexposed; no feature, dependency or artifact profile changes in this checkpoint.
[Canonical contract, scope and exact evidence](knowledge/pwright-node-handles.md#node-172-rust-owned-preflight-and-reentrant-language-phases).
Earlier checkpoints below retain their historical scope.

NODE-171 (2026-09-17): Internal Node preparation now has a module-local,
weak exact-session `NativeFileInputPreparation` receipt. The registry admits
the existing FFI Command slot/1 MiB allowance before rooting receiver, files
and options in a private three-element array. It invokes no user getters,
Buffer conversion, filesystem or native browser operations. Weak Handle source
metadata validates session/Element provenance without prematurely checking
disposal; final native submission still requires the actual live source.

The new addon `retained-file-input` feature implies `handles`, `file-input`
and the host's retained selection. Only the new `retained-file-input` and
`retained-file-input-combined` artifact profiles enable it; thirteen existing
profiles remain unchanged. Scope close, NativeSession Drop and environment
cleanup revoke rooted entries before FFI drain. Delivered terminal publication
revokes too; a raw session without a watcher may wait for later publication,
explicit close, wrapper Drop or environment shutdown. Public
`ElementHandle.setInputFiles`, owner/context/conversion phases, preparation
promotion and submission remain absent. This is not browser verification or
completed ingress parity. [Canonical contract and evidence](knowledge/pwright-node-handles.md#node-171-session-bound-node-preparation-registry).
Earlier checkpoints below retain their historical scope.

NODE-170 (2026-09-17): The safe host adds a revocable preparation prerequisite before tracked FFI execution. Exact budget origin is environment-wide, not session authority. Public owner/context lookup, conversion microtasks, channel validation and deferred Buffer serialization still need a bounded Node registry and phase-correct language ports; no public upload API or addon artifact is added.
[Canonical contract and evidence](knowledge/pwright-node-handles.md#node-170-revocable-ffi-preparation-before-execution).

NODE-169 (2026-09-17): The safe Rust host now selects retained-file-input independently and accepts already-materialized, installation-bound payloads for an exact ElementHandle. This is not the Node ingress implementation: the offline pinned oracle fixes owner/getter/microtask/error/timeout ordering that the later Node preparation owner must obey. No public ElementHandle.setInputFiles or addon artifact is added.
[Canonical contract and evidence](knowledge/pwright-node-handles.md#node-169-safe-host-retained-upload-and-pinned-ingress-oracle).

NODE-168 (2026-09-17): Retained upload requires more than ownerFrame plus the
existing Locator ingress. Pinned Playwright performs owner lookup, a separate
size/stat phase, options/channel getters, then deferred server-side byte
materialization; getter/microtask mutation is observable. Keep the preparation
state, native ownership and mutation in Rust, with only bounded Node-thread
roots/conversion callbacks in the language boundary. The new opt-in exact-node
group owner is a prerequisite, not public ElementHandle.setInputFiles support.
[Detailed ordering, ticket design and evidence](knowledge/pwright-node-handles.md#node-168-group-owned-retained-adoption-and-deferred-upload-ingress).

NODE-167 (2026-09-17): The handles artifact adds ElementHandle.ownerFrame.
The complete node/document/native Frame lookup and temporary owner lifecycle
remain Rust-owned; the Node facade resolves the returned portable ID to the
existing Context/Page Frame cache and propagates that resolver through Handle
producers. No portable protocol, package or feature change. The retained-upload
preflight/ingress/adoption design and [verification status](knowledge/pwright-node-handles.md)
remain separate from complete ElementHandle actions and Playwright parity.

NODE-166 (2026-09-17): The Handles artifact now exposes JSHandle.getProperties
with ordered Map results and ElementHandle classification, backed by one
Rust command with independently owned siblings. N-API emits own data properties
and compensates partial native conversion; JS wrapper/Map failures await
attempt-all native disposal without GC. The selected session has at most 64
retained result owners, including the source, and eight shared Handle calls;
a batch reserves all currently free result capacity before browser work.
No portable protocol or package/feature selection changes. [Canonical
ownership, memory boundaries and evidence](knowledge/pwright-node-handles.md)
remain separate from full Playwright parity.

NODE-165 (2026-09-17): The existing Rust retained-result path now explicitly
selects raw CDP acquisition/release through settlement, preserving result and
exception IDs before semantic error conversion. Addressable IDs are
deduplicated and compensated on failure under the existing evaluation lease;
unresolved native identity stays Cleanup. Host cancellation still retains
its call allowance, frozen-world snapshot and pre-admitted recovery ownership,
and failed abandoned compensation remains visible to continuation health.
The shared parser stays pure; ordinary CDP methods are unchanged.

This adds no public Node API, portable schema, package or feature. Exact
artifact, verification and pending-gate status belongs to
[the canonical checkpoint](knowledge/pwright-node-handles.md).
Runtime.getProperties late settlement, full property-pipeline admission,
pre-owned cleanup groups for unaddressable IDs, Node getProperties batch
publication and the full compatibility corpus remain separate work.

NODE-164 (2026-09-17): Raw native Handle, Session, Environment, Frame, Context-settings, Locator and Signal classes now use module-local TypeTag identity through the actual N-API receiver/argument checks, addressing the previous priority cross-build implementation gap. Portable single-property reads also preserve snapshot identity with pre-admitted ownership, and abandoned cleanup failures stay visible. Exact old-peer and independently loaded same-build verification, together with remaining gates, is recorded in [the canonical checkpoint](knowledge/pwright-node-handles.md). Node getProperties and broader Playwright compatibility remain incomplete.

NODE-163: Handle-targeted evaluateHandle and single-property creation now share Rust-owned staging, native references, source-world revalidation and publication recovery. Native-class and JS-wrapper publication are separate fallible boundaries with explicit ownership rollback. Property Promise/thenable identity and complete paired evidence are tracked in knowledge/pwright-node-handles.md; getProperties remains pending atomic batch ownership.

NODE-162: The handles artifact now forwards the source-kind flag for JSHandle.evaluate. Rust-native programs preserve expression semantics while validating ignored references, without extending portable wire schemas. A disposed target's error follows ordinary and deferred binary conversion. Final evidence and unresolved broader compatibility gates are recorded in knowledge/pwright-node-handles.md.

NODE-161: The handles profile adds native type-tagged reference arguments with retained Rust owners, nested graph preservation, host-session checks and TypeScript unboxing. It reuses binary-copy timing and preserves ordinary getter traversal before disposed-argument rejection. Frame by-value reference calls use an independent leaf, not create/read/dispose emulation. String-expression reference arguments and cross-Frame adoption remain unsupported.

NODE-160: The independent handles artifact now exposes a narrow public Handle vertical slice, with all native creation/reads/release in Rust. Existing deferred binary conversion is reused, explicit disposal waits for completion, and disposed wrappers release native admission without GC. Unsupported reference arguments, string Handle evaluation, toString and broader ElementHandle methods remain explicit gaps.

NODE-159: Safe-host Frame creation now uses native-creation and driver publication, with cleanup ownership across abandoned receivers and late/closed queue arrivals. A successful reply contains PendingHandle, never an unowned initializer. Public Node wrappers, other Handle producers and their conformance lane remain next.

NODE-158: An optional Rust native-creation interface now supports staged Frame.evaluateHandle publication without rerunning JavaScript on a busy lifecycle gate. It is not selected by the host/addon yet: bounded queue admission, orphan-result compensation, parent-retirement failure handling and shutdown integration are required before using it from Node.

NODE-157: Safe-host pending Handles now use split native release and driver-owned local commit, with evidence-based parent retirement during close. Public N-API Handle producers remain absent. The existing combined addon was rebuilt without its unused native-close control outbox; paired close confirmation passes 6/6 and lifecycle events 7/7. Full shared parity was not rerun. See the canonical Node Handle checkpoint for exact artifact and remaining creation/publication work.

NODE-156: Native Handle release now has an optional acknowledgement receipt and short exact-incarnation local commit, verified against real Chrome. This is not public Node parity: ordinary and combined host/addon graphs still must not select native-release until driver/shutdown integration exists.

NODE-155: Runtime disposal admission now supports ordinary queue-free hosts, with a retained owner on busy commit boundaries. This prerequisite is not yet connected to native Handle release/host scheduling; the public Node Handle gate and existing addon artifacts remain unchanged.

NODE-154: Safe Rust frame_prepare_handle returns an owned pre-publication PendingHandle whose cleanup is revocable by the session. This does not open N-API or JSHandle/ElementHandle APIs; persistent wrapper publication/disposal and broader lifecycle parity remain required. Keep native retained-result shutdown failures separate from unit ownership passes.

NODE-152: Safe-host handles selection now installs the independent route owner and exposes native metrics. Existing addon selections exclude it; there is no Node Handle creation API or rebuilt artifact. Next close pending-result ownership, post-conversion abandonment and retained-wrapper shutdown before exposing creation.

NODE-151: Node Handle producers/lifecycle remain absent; the new 18-case paired lane is red. Pinned oracle behavior requires live-owner preflight before payload getters and exact retained-node identity even after DOM removal. A Rust borrowed payload-target seam is implemented, but N-API publication/adoption/owned continuation remain to integrate.

NODE-150: Page/Frame.setInputFiles are implemented under file-input with default-first/explicit-strict semantics and existing publication/cleanup. ElementHandle/directory and timing gates remain open.
[Page/Frame upload contract and verification](knowledge/pwright-file-input.md#node-150-page-and-frame-selector-taking-upload-apis).

NODE-149 verification: standalone upload passes three consecutive 32-case runs plus strengthened Worker-byte checks; combined upload passes 32/32. Full shared corpus: 53 match, one click-result mismatch, one unsupported download and two upstream history errors. The overall compatibility goal stays open.

NODE-149: The opt-in file-input addon and Locator.setInputFiles wrapper are implemented. Buffer language typing is checked before native backing inspection/copy. Page/Frame/handle methods and full timing/directory parity remain open.
[Selected Node upload boundary and verification](knowledge/pwright-file-input.md#node-149-selected-node-buffer-ingress-and-public-locator-uploads).

NODE-148: Safe Rust HostSession/HostLocator upload APIs now compose the existing leaves under file-input. N-API and the JS package still have no setInputFiles method or file-input artifact. Preserve original public deadlines across ingress and do not reuse evaluation's 1 MiB BinaryView policy for large files.
[Selected safe-host upload contract](knowledge/pwright-file-input.md#node-148-selected-safe-host-upload-integration).

NODE-147: Nearly 50 MiB buffered uploads now have an explicit Rust transport path without enlarging ordinary Node CDP limits. The Node host/addon still do not select file-input; keep FFI ingress and lifecycle integration separate and preserve the ordinary 64 MiB settlement budget.
[Large outbound admission and remaining gates](knowledge/pwright-file-input.md#node-147-independently-admitted-large-outbound-commands).

NODE-146: Rust payload continuations now retain already-started uploads after caller cancellation without reissuing them, and explicitly release the exact group before result publication. The Node host/addon do not select this new leaf yet. Page/Frame/Worker shutdown and public timeout/error wiring remain required; native artifacts are unchanged.
[Upload continuation and remaining gates](knowledge/pwright-file-input.md#node-146-single-use-upload-continuations).

NODE-145: The Rust payload backend now reaches real Chrome and passes 14 browser fixture scenarios, including the full Rust worker/file/MIME/base64/CDP path and setter-exception cleanup via group-owned dispatch. This does not wire public Node setInputFiles, native path/directory completion, operation cancellation or the shared-corpus upload case. The existing native addon is unchanged.
[Low-level browser evidence](knowledge/pwright-file-input.md#node-145-chromium-payload-mutation).

NODE-144: An optional Rust Tokio adapter now runs bounded filesystem preparation off the Node event loop, with dropped-waiter/queued cancellation and close-and-drain semantics. It is not yet wired into Node operations; memory-payload preparation, Chromium upload, message budgets and public setInputFiles remain unimplemented. No addon rebuild or browser parity claim is made.
[Executor checkpoint](knowledge/pwright-file-input.md#node-144-admitted-blocking-executor).

NODE-143: Rust path preparation and pinned MIME lookup now exist in an optional filesystem leaf. Original-path stat ordering, second-directory rejection, separate-browser size stats and post-read mtime are represented. Native Rust MIME lookup matches the pinned oracle on 9,393 cases. No public setInputFiles, production executor, browser transfer or Node artifact is added.
[Filesystem/MIME checkpoint](knowledge/pwright-file-input.md#node-143-filesystem-preparation-and-pinned-mime).

NODE-142: The optional Rust content materializer now preserves bytes and supplied name/MIME/mtime with batch-owned resource admission. This is not Node setInputFiles: path/MIME preparation, file-input backend, admitted executor and FFI integration remain open. Existing native artifacts and NODE-140 non-green differential results are unchanged.
[Content ownership checkpoint](knowledge/pwright-file-input.md#node-142-ledger-admitted-content-ownership).

NODE-141: Begins setInputFiles with a dependency-free Rust preparation model (29 release tests) and 21 pinned upstream browser observations. Default connectOverCDP paths transfer as payloads, directories use native paths, and exact 50 MiB totals reject. The public upload API, materializer and addon feature remain unimplemented.
[File-input policy, evidence and remaining work](knowledge/pwright-file-input.md).

NODE-140: Full Frame navigation remains 27/29; the unchanged 57-case shared corpus yields 51 equal values, one click mismatch, one lifecycle RequestCapacity setup error, two unsupported operations and two oracle history errors. These are not complete compatibility evidence. New bounded controls reproduce upstream close/click failures too, without identifying their cause or changing the addon.
[Expanded verification and retained failures](knowledge/testing-strategy.md#node-140-broadened-node-ffi-verification-remains-non-green).

NODE-139: Frame.goto now keeps its Rust Page lifetime witness through the publication fence and final projection. Actual closure interrupts a pending fence; a live Page retains its native navigation error. The prior transparent ERR_ABORTED/TargetClosed comparison was not phase-equivalent. Overall compatibility remains unfinished.
[Page lifetime publication contract and verification](knowledge/pwright-frame-navigation.md#node-139-page-lifetime-through-navigation-publication).

NODE-138: Adds explicit concurrent-close and concurrent-close-drag-frame-extra-headers artifacts. Owned Context/Page close requests use the Rust native scheduler and committed local retirement; default/external contexts preserve their old service semantics. The default artifact remains unchanged; selected browser evidence is tracked separately.
[Node close-driver integration and final verification](knowledge/pwright-close-cascade.md#node-138-opt-in-node-close-driver-integration).

NODE-137: The missing local half of scheduled close now has an optional composition API returning post-session-lock notifications. Current Node profiles still exclude it; driver request admission/polling, metadata/FFI settlement, addon rebuilds and the controlled ancestor-dispatch browser gate remain next.
[Native local retirement and post-lock notification contract](knowledge/pwright-close-cascade.md#node-137-native-proven-local-retirement-and-post-session-lock-notification).

NODE-136: Rust capacity owners can now be consumed together with borrowed native evidence. Current Node profiles explicitly exclude close-capacity, native scheduling, route retirement and transitive reservation features. These opt-in Rust changes do not fix the controlled Node ancestor-dispatch discrepancy or rebuild the addon.
[Joined close capacity and native completion contract](knowledge/pwright-close-cascade.md#node-136-joined-close-capacity-and-consuming-native-completions).

NODE-135: Runtime now offers guarded local publication plus deferred disposal, with ten new synchronization/failure tests. The default host regression preserves its old serial behavior; this is not Node scheduler wiring or a browser discrepancy fix. No rebuilt addon/browser result is claimed.
[Runtime-locked publication/disposal contract and evidence](knowledge/pwright-close-cascade.md#node-135-runtime-locked-local-publication-and-disposal).

NODE-134: Optional runtime outbox storage now preallocates both long-lived buffers and retains their byte admission across shutdown/handle lifetime. Runtime selected/combined tests pass 168/186, but this capability is not yet selected by the Node close coordinator. No addon rebuild or new browser evidence; NODE-133 artifacts remain installed.
[Preallocated runtime outbox contract and evidence](knowledge/pwright-close-cascade.md#node-134-preallocated-runtime-control-outbox-storage).

NODE-133: Rust retained-world receipts now recognize exact native Page destruction after local route retirement. Selected/combined composition tests pass 397/486, default host 416, and both rebuilt addon profiles pass all 28 focused Chrome regression cases. These do not prove complete compatibility or resolve the controlled concurrent-close discrepancy; artifact hashes and scope are recorded below.
[Native Page world-retirement contract and evidence](knowledge/pwright-close-cascade.md#node-133-retained-worlds-follow-exact-native-page-destruction).

NODE-132: Added exact attachment/native proof to current-topology commit scope. Native-peer tests exercise actual Context/Page topology retirement and parent-first child joins; the Node host does not yet select a full close coordinator, and installed addons/browser discrepancy evidence are unchanged.
[Native close commit-scope contract and evidence](knowledge/pwright-close-cascade.md#node-132-attachment-bound-native-close-commit-scope).

NODE-131: Added selected deferred runtime retirement with retained result/waker admission, required before a short multi-component close commit can safely release outer locks. Node host scheduling and rebuilt-addon/browser discrepancy verification remain unfinished; current addon artifacts are unchanged.
[Deferred runtime close commit contract and evidence](knowledge/pwright-close-cascade.md#node-131-deferred-runtime-disposal-notifications).

NODE-130: Rust gains an independently selected local route-retirement primitive. No route/provider clones or native commands are needed in its prepare/commit; readiness wake is explicit after locks. It is not selected by the Node host/addon and does not itself prove native destruction or close-event publication.
[Deletion-only close route contract and evidence](knowledge/pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-129: Rust now has an opt-in bounded native scheduler that dispatches parent disposal despite a child destruction wait. Evidence remains private-constructor and revalidated before use; Node still selects the serial lifecycle service. No rebuilt addon or browser discrepancy pass is claimed.
[Bounded native scheduler contract and evidence](knowledge/pwright-close-cascade.md#node-129-bounded-native-close-scheduler).

NODE-128: Rust now has attachment-derived native Context receipts and sealed Page ancestry, including epoch, source-domain, generation and current-route checks. Node does not select native-close-authority or use its commands yet; installed artifacts and browser evidence remain unchanged. Coordinator integration is still required.
[Native ancestor authority contract and evidence](knowledge/pwright-close-cascade.md#node-128-exact-native-context-ancestor-authority).

NODE-127: Rust native confirmation now offers prepare_command, a command-only acknowledgement owner, and a prebuilt exact-destruction wait. Node scheduling does not yet call this additive API. Default/combined addon Clippy passes; installed binary hashes and browser evidence remain unchanged.
[Native close phase contract and evidence](knowledge/pwright-close-cascade.md#node-127-native-command-and-destruction-wait-phases).

NODE-126: Adds Rust-only deferred publication capacity with current-view planning. The Node driver still uses the ordinary serial close path; no addon artifact was rebuilt and no browser discrepancy is claimed fixed.
[Deferred publication contract and evidence](knowledge/pwright-close-cascade.md#node-126-deferred-close-publication-capacity).

NODE-125: Rust runtime now optionally reserves future disposal/outbox capacity without blocking other lifecycle work, then prepares current-state disposal. This complements NODE-124 topology reservations but remains unselected by host/addon. Native intent scheduling, exact ancestor outcome authority, publication rebasing and full peak ownership budgets remain next. Existing Node binary hashes and the ancestor-dispatch discrepancy are unchanged.
[Runtime reservation contract and evidence](knowledge/pwright-close-cascade.md#node-125-opt-in-runtime-disposal-and-outbox-reservations).

NODE-124: Adds a feature-gated Rust topology transition-reservation primitive, not a new Node API or selected host runtime. Capacity can cross a native wait without freezing topology; commit preparation uses the current owner/epoch state. Runtime disposal, native coordinator and publication integration remain required before the ancestor-close discrepancy can be fixed. NODE-123 binaries and browser evidence remain unchanged.
[Topology reservation contract and evidence](knowledge/pwright-close-cascade.md#node-124-opt-in-topology-transition-reservations).

NODE-123: Context.close interruption preserves the upstream TargetClosed outcome and terminal emitter identities. Driver reply-producer and request-queue loss now seal admission before close settlement can mistake shutdown for an ordinary closed object. Final profiles each pass 50 related browser cases and two extra six-case repeats. Transparent/controlled probes retain a separate ancestor-dispatch gap despite two admitted FFI/host operations. Next implement bounded native intents with serialized latest-state commits, not a JS driver or queue workaround. The compatibility goal remains active.
[Close cascade design and evidence](knowledge/pwright-close-cascade.md).

NODE-122: Node Events now selects Rust exact-target destruction confirmation instead of the ordinary finite absence-check policy. All admission, native wait, cancellation, transaction and final settlement fencing remain Rust-owned. Both final artifacts pass 48 close/lifecycle/connection tests; the extra Frame-navigation corpus remains 27/29 at the previously documented arrival boundary. Final error formatting is bounded without changing failure classification. Preserve the broader discrepancy backlog and historical natural-failure evidence; the goal is not complete.
[Selected confirmation](knowledge/pwright-page-close.md#node-122-selected-native-destruction-confirmation).

NODE-121: The original listener child can use an explicitly validated loopback diagnostic relay without changing production endpoint selection. Bounded original-scenario traces pass in both existing artifacts. Pinned ordinary-close semantics still differ from the current 20-check Node policy; a native opt-in confirmation plan is recorded.
[Transparent trace and next confirmation contract](knowledge/pwright-browser-close.md#node-121-transparent-original-scenario-tracing).

NODE-120: Browser.close joins the native driver and can surface an earlier topology initialization failure. Private Rust phase labels now distinguish that from retirement and transport failures without changing error codes, resource ownership or deadlines. A bounded relay test verifies this through the actual addon.
[Close attribution contract and evidence](knowledge/pwright-browser-close.md#node-120-attribute-retained-native-driver-failures).

NODE-119: Production native PageReady/PopupLinked effects now drive the NODE-118 precommit wrapper converter. Context.page and Page.popup delivery share Rust-owned identity, visibility and acknowledgement state, including delayed linkage. The Node facade has no readiness/diff policy or driver dependency.
[Native ready-source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-119-native-ready-and-popup-source).

NODE-118: Creation preparation is two-phase: the private JS adapter requests missing identities, Rust binds descriptors from LifecycleNoticeView before commit, then an emit closure installs caches after native publication. Cached Page/Context wrappers request no replacement bindings. Ready/linkage producers remain pending.
[Creation-conversion contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-118-precommit-creation-wrapper-conversion).

NODE-117: Selected Rust driver receipts now produce external Page/Context close notices through the existing Node publication port. Both profiles pass 5/7 lifecycle cases and 14/18 listener cases; newPage and popup notification sources are still incomplete.
[External-close source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-117-pre-admitted-external-close-source).

NODE-116: Node now selects connect_over_cdp_with_lifecycle_events, composing explicit close receipts with terminal delivery. Source admission precedes native close; committed Context disposal expands its native-owned visible Page subtree. The existing FFI dispatcher needs no JS-side event synthesis.
[Explicit-close source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-116-explicit-close-source-integration).

NODE-115: All asynchronous addon publication paths now use the same environment-thread PublicationPort. Node connections select the terminal native source. Page/Context close callbacks from Session termination now have real FFI delivery; ordinary creation, popup and explicit-close sources remain unjoined.
[Terminal dispatcher contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-115-environment-thread-terminal-dispatch).

# Playwright-compatible Node FFI design

NODE-114: An experimental Rust connect_over_cdp_with_terminal_events path now produces groups from actual host teardown. The Node addon keeps ordinary connect_over_cdp until all its fence paths have a bounded foreign consumer. This native live gate does not establish JS lifecycle-event parity.
[Terminal source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-114-pre-admitted-native-terminal-source).

NODE-113: A bounded Rust lifecycle group now joins incremental collection visibility, Page closed metadata and synchronous delivery acknowledgements. Node still uses legacy publication and does not receive these notices. New reference close-listener cases distinguish independent close events from dependent popup callbacks.
[Group contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-113-bounded-dependent-publication-groups).

NODE-112: The explicit close native-input port is available but not selected by the Node
host yet. It pre-admits retention and completes after native ownership commits.
Internal producer failures remain distinct from user-listener exceptions;
foreign dispatch and a bounded dependent journal are still required.
[NODE-112 close receipt contract and verification](knowledge/pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: The Rust driver exposes borrowed committed effects to selected native observers.
The host still uses legacy snapshot projection, so this is source plumbing,
not new Node lifecycle support. Direct close/terminal cause inputs and admitted
foreign notification groups remain required.
[NODE-111 source contract and verification](knowledge/pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-110 fixes synchronous Browser disconnected listener errors incorrectly
surfacing as unhandled Promise rejections. The scoped Rust language port reports
the original value as uncaughtException; close remains resolved. New isolated
listener cases also constrain native cause-group failure and reentry semantics.
This does not implement the missing Page/Context event dispatcher.
[NODE-110 contracts, fix and verification](knowledge/pwright-node-lifecycle-events.md#node-110-listener-failure-boundaries-and-scoped-exception-reporting).

NODE-109 implements the exclusive metadata publication owner in Rust, without
wiring Node callbacks. A raw queue change is not a Playwright event: close state
and collection membership can occupy separate changes. Native cause ordering,
intermediate visibility and listener reentry/throw semantics remain required
before switching Node away from full-fence publication.
[NODE-109 contract and verification](knowledge/pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-108 confirms existing exact native-session retirement in controlled real
Chrome comparisons (default 6/6, combined 10/10, unchanged NODE-107 addons).
Fresh Rust probes include bounded detach/destruction identity receipts and pass
six layouts each without reproducing the startup timeout. No Node logic, deadline
or event dispatch changed; the earlier partial matrix remains historical failure
evidence, not a fixed issue. The admitted Rust event-step owner is still required.
[NODE-108 receipts and verification](knowledge/pwright-chromium-topology-events.md#node-108-bounded-native-lifecycle-identity-receipts).

NODE-107 verification is partial: native tests/strict checks pass, while combined
live attachment times out in multiple lanes. A same-feature Rust-only probe
reproduces four observer-layout timeouts, proving that the observed failure can
occur outside the Node bridge; its cause is not yet fixed. Event callbacks remain
unimplemented. No full compatibility or all-green claim follows from this work.
[Artifact-specific matrix and trace evidence](knowledge/pwright-node-lifecycle-events.md#combined-profile-reproduction-and-incomplete-live-gate).

NODE-107 implements Rust cross-revision visibility rebasing and domain/identity
validation without a union allocation. It is a primitive for the proposed step
owner, not a JS lifecycle engine or an installed callback dispatcher. Existing
NAPI fences still publish full views; full event compatibility remains open.
[NODE-107 contracts and evidence](knowledge/pwright-node-lifecycle-events.md#node-107-validated-cross-revision-visibility-rebasing).

NODE-106 implements and measures the Rust immutable collection visibility layer.
Node getters consume borrowed members from full published views; no JavaScript
lifecycle logic or foreign dispatch was added. Both addon profiles are rebuilt;
the single-revision foundation is not the proposed cross-revision event cursor.
[Native contracts, evidence and remaining work](knowledge/pwright-node-lifecycle-events.md#node-106-immutable-single-revision-visibility-foundation).

NODE-105 records a seven-case lifecycle-event discrepancy: upstream passes;
both unchanged NODE-103 artifacts fail every case while final collections agree.
The proposed Rust step cursor must combine each state transition and typed
notification, preserve fixed fences and cache identity, admit terminal work,
and invoke foreign callbacks outside locks. JS must not infer lifecycle state.
[Evidence, reuse points and implementation gates](knowledge/pwright-node-lifecycle-events.md).

NODE-104 adds seven creation/discovery publication comparisons, passing twice
on each unchanged NODE-103 artifact. Readiness separates native Command and Close
lanes, and concurrent insertion order is not confused with invocation order.
The next concrete Node compatibility gap is lifecycle event delivery: getters
are published, but Context page, Page/Context close and popup emissions are absent.
[Tests, corrections and next event boundary](knowledge/pwright-frame-publication.md#node-104-creation-and-discovery-publication-characterization).

NODE-103 completes the selected Rust collection producer/getter cutover.
The unchanged collection differential gate is now 7/7 on rebuilt default and
combined artifacts; Browser/Page scalar and enumeration regressions also pass.
Native topology/admission and Node-published membership remain separate. The FFI
baseURL TypeError correction lives in Rust and preserves user getter exceptions.
No production JS driver, transport, browser launcher or lifecycle state was added.
[Implementation, budgets and artifact-specific evidence](knowledge/pwright-frame-publication.md#node-103-rust-collection-producers-and-published-node-reads).
Earlier NODE-101/102 red/unwired descriptions are historical; broader parity remains open.

NODE-102 implements bounded Rust collection revision storage and fixed-fence
publication, with eighteen deterministic ownership/capacity tests. Default and
combined host suites pass 263/311 tests. No JS state or new dependency is added.
Native producers and Node collection getters are still unchanged; NODE-101's
six discrepancies remain open and no addon artifact was rebuilt.
[Implementation, budgets and cutover requirements](knowledge/pwright-frame-publication.md#node-102-bounded-rust-collection-revision-storage).

NODE-101 establishes a RED collection-publication gate: upstream 7/7 twice,
default/combined candidates 1/7 each. Native topology disappears before Node
delivery while scalar lifecycle getters correctly retain their published values.
Only test infrastructure and design evidence change; implement bounded membership
publication in Rust before claiming coherent Context/Page lifecycle compatibility.
[Contract, root cause and implementation requirements](knowledge/pwright-frame-publication.md#node-101-context-and-page-collection-publication-discrepancy).

NODE-100 corrects Browser.isConnected at the Node event-loop boundary. The
Rust-owned event port transitions once immediately before disconnected emission;
native admission/metrics stay native. The new paired lane improves from 2/5
to 5/5 on both rebuilt profiles, with Page.isClosed 8/8 each. Legacy artifacts
retain earlier timing and must be rebuilt. Full Node API parity remains open.
[Contract and evidence](knowledge/pwright-browser-close.md#node-100-rust-owned-browser-disconnect-publication).

NODE-099 exposes synchronous Page.isClosed through the existing Rust Page-tree
binding. The Node layer only forwards and maps errors. Each Page pre-admits a
close notification in the existing metadata queue; native retirement remains
invisible until its fixed Node publication fence. Terminal teardown works under
full admission and retains no browser/host in published metadata. Older artifacts
require rebuilding. Page events and broad Node API parity remain unfinished.
[Contract, resource proof and verification](knowledge/pwright-page-close.md#node-099-rust-owned-page-close-publication).

NODE-098 adds a failing Page.isClosed differential lane and a pinned eight-case
reference contract. Crash is not close; borrowed disconnect closes the local
wrapper without destroying the native Page. A confirmed native close during a
blocked Node turn remains unpublished until Node yields. Keep the getter's state
in Rust and preserve fixed publication fences, including saturated teardown.
No production API is added yet; default/combined baseline is 0/8 (missing method).
[Design gate and evidence](knowledge/pwright-page-close.md#node-098-pageisclosed-reference-contract-and-failing-candidate-baseline).

NODE-097 expands actual upstream/Rust comparison to connection during paused
iframe initialization. Both exhibit native retirement and timeout when a second
client enables Page-scoped auto-attachment, then recover on an explicit second
navigation. Combined/default matrices pass 11/11; production JS/Rust are unchanged.
This classifies one stress signature without hiding prior failures or claiming
full compatibility. Next close the explicit Page.isClosed API gap in Rust.
[Checkpoint](knowledge/pwright-chromium-topology-events.md#node-097-page-auto-attachment-from-a-second-client).

NODE-096 is a test/evidence checkpoint, not a new production API. The four-way
owner/borrowing-peer disconnect control passes on the combined artifact, while
churn failures remain. FFI admission metrics are Node-environment-wide, unlike
Browser-local host leases; assert zero only after both candidate connections and
their operations drain. Rust navigation and its deadline remain unchanged.
[Checkpoint](knowledge/pwright-chromium-topology-events.md#node-096-navigation-timing-and-borrowing-peer-control).

NODE-095 selects Rust pending-call retirement on exact native session detach.
Node remains a language/FFI boundary. Two new public cases fail on the old addon
because connect waits for a response after detach. Current combined/default
attachment cases pass 10/10 and 6/6; Worker ordering controls pass 3/3 each.
Overlap still fails two child cases per run, despite headers passing 35/35.
[Contract and evidence](knowledge/pwright-chromium-topology-events.md#node-095-native-detach-before-an-initialization-response).

NODE-094 fixes selected Rust Context attribution for queued attachments from
already disposed foreign Contexts. No Node browser logic or protocol-error
suppression is added. The new queued-context-retirement lane compares disposed
and live borrowed Contexts against pinned Playwright 1.58.2.
[Checkpoint](knowledge/pwright-chromium-topology-events.md#node-094-queued-attachments-from-disposed-contexts).

NODE-093 keeps native Network.enable errors typed across the selected Rust
header initializer, reusing the topology driver's exact-session retirement
policy. No Node automation logic or new dependency is added. Expanded attachment
tests distinguish lifecycle/header occurrences and verify a headers-only build.
[Evidence and remaining discrepancies](knowledge/pwright-extra-http-headers.md#node-093-typed-header-initialization-retirement).

NODE-092 reproduces and fixes a child disappearing between resume and detach:
connection initialization now accepts the exact already-detached receipt in Rust.
The new test:detach-retirement lane checks dead and live browser Workers plus
borrowed Page preservation. [Contract and evidence](knowledge/pwright-chromium-topology-events.md#node-092-child-disappears-between-resume-and-detach).

NODE-092 final: default/combined targeted tests 2/2 each plus a combined repeat;
serial child headers 8/8 and attachment 7/7. Native regressions pass 79/373/274
CDP/composition/host tests. The overlapping run still fails (35/35 headers,
4/8 child headers), so broader connection/disconnect parity remains unfinished.

NODE-091 moves child-session-close behavior into bounded Rust request watches.
Live protocol errors still reject; actual child closure settles before a native
command receipt. Main-session and replacement-child comparisons are included in
the new public lane. Full Node compatibility, including Page.isClosed, remains
unfinished. [Contract and evidence](knowledge/pwright-extra-http-headers.md#node-091-native-session-closure-during-header-fanout).

Final NODE-091 public gate passes 8/8 on both combined and headers-only artifacts,
plus a combined 8/8 repeat. Headers pass 35/35, attachment 7/7, host 274. An
overlapping run still exposed disconnect/attachment races; full connection and
API compatibility remain unproven.

NODE-090 handles exact missing-session receipts in discovered/routed native
initialization and child cleanup in Rust, including lifecycle-owner retirement.
Public attachment tests pass 7/7 plus three 7/7 repeats; headers pass 35/35.
Host tests pass 269. Other attachment and Node compatibility work
remains open. [Contract and evidence](knowledge/pwright-chromium-topology-events.md#node-090-missing-session-retirement-before-page-publication).

NODE-089 final verification: 35/35 public header tests, three 4/4 receipt-gate
repetitions, 269 Rust host tests and an isolated zero-owner native live test pass.
An earlier 34/35 run and a concurrent native test failed during attachment with
a missing CDP session; that reliability defect remains open.

NODE-089 fixes three observed native header discrepancies entirely in Rust:
Context Pages advance sequentially, later Pages read then-current desired values,
and native protocol errors preserve the public method name. Four real-Chrome
receipt-gated public comparisons pass, including a newer setter while old ACKs
are held. No production Node driver or CDP transport is added.
[Evidence and exclusions](knowledge/pwright-extra-http-headers.md#node-089-native-receipt-gates-and-context-page-sequencing).

NODE-088 polls bounded Rust header jobs alongside existing topology/lifecycle
awaits. First failure settles the Node completion while primary siblings and
compensation remain owned and polled. Combined public tests pass 31/31; controlled
Rust gates prove early result/drain separation, not full real-CDP timing parity.
[Implementation and remaining scope](knowledge/pwright-extra-http-headers.md#node-088-caller-polled-native-jobs-and-early-primary-rejection).

NODE-087 invokes Rust header publication and Page/Frame policy capture before
N-API schedules completion. Queue admission precedes mutation, and each native
request retains a frozen plan. The combined public header lane passes 30/30;
concurrent native fanout and early rejection with tracked compensation remain open.
[Boundary and evidence](knowledge/pwright-extra-http-headers.md#node-087-synchronous-rust-preparation-and-frozen-dispatch).

NODE-086 reproduces navigation reading stale headers when a preceding setter is
not awaited. The Rust model now optionally supports bounded overlapping writers,
matching 128 controlled upstream traces; host/FFI ordering is not fixed. Required
next work separates ordered preparation, native progress and public settlement.
[Evidence and integration requirements](knowledge/pwright-extra-http-headers.md#node-086-pending-setter-race-and-bounded-overlapping-model).

NODE-085 reconciles public Page/Frame navigation referer options in Rust against
the first original Page header. No JS policy or dependency is added. Combined
header/drag/frame tests pass 24/24; pending mutation races and full native failure
semantics remain open.
[Policy and evidence](knowledge/pwright-extra-http-headers.md#node-085-navigation-referrer-reconciliation).

NODE-084 adds selected creation-time headers through N-API. JS only copies options,
selects the language port and wraps the result; Rust owns admitted strings, Context
creation, settings installation and compensation. Header-only public differential
coverage passes 21/21. This remains an experimental subset, not full parity.
[Creation boundary](knowledge/pwright-extra-http-headers.md#node-084-context-creation-headers).

NODE-083 corrects header validation timing in Rust: original desired strings
are bounded but HTTP syntax is checked only after merge at installation.
Expected Page-initialization syntax failure is local; its exact created Page is
closed before rejection without terminating the shared attachment. Context
creation options remain unimplemented pending their admitted transaction design.
[Observed discrepancies and recovery](knowledge/pwright-extra-http-headers.md#node-083-deferred-syntax-and-page-local-initialization-failure).

NODE-082 adds separately selected public Page/Context header setters. Rust owns
header policy and phase ordering; synchronous Node language ports preserve value
and exception behavior. The header-only release addon passes ten live tests
(nine pinned paired groups plus admission); native failure timing, Context
creation options, referer reconciliation and worker/OOPIF coverage remain open.
[Contracts and evidence](knowledge/pwright-extra-http-headers.md#node-082-public-selected-setters-and-language-phase-differential).

NODE-081 installs `pwright-node-host/extra-headers` behind an independent feature.
Provisional native Rust setters, session initialization and obsolete-session
clearing are real and live-tested. Node exports and packaged addon profiles are
unchanged. Sequential native fanout / awaited compensation are not the final
public timing contract; no compatibility claim is made for those paths.
[Scope, evidence and next FFI steps](knowledge/pwright-extra-http-headers.md#node-081-selected-rust-host-and-native-session-reconciliation).

NODE-080 adds a Rust-only desired-header registry with bounded update owners.
Failed/cancelled writes restore metadata and separately record native uncertainty;
ordinary reads remain compatible with Playwright. The host/addon does not
install it yet; no JavaScript setter or native fanout is implied by 54 internal
setter matches. [Contract](knowledge/pwright-extra-http-headers.md#node-080-bounded-desired-settings-transactions).

NODE-079 gives the topology initializer portable Page/Context ownership before
resume, including a reserved new-Page ID. Its default method preserves the
existing Node host initializer. Node API/addon artifacts remain unchanged;
header storage, native installation/rollback and public setters are still next.
[Scoped port](knowledge/pwright-extra-http-headers.md#node-079-identity-aware-native-initialization).

NODE-078 implements a default-off Rust extra-header value/policy module:
25 crate tests and 484 internal Playwright comparisons pass. Two public upstream
runs reproduce 23 observations. Host/session fanout, rollback, creation/setter
FFI and actual paired API tests remain open; addon artifacts are unchanged.
[Header integration contract](knowledge/pwright-extra-http-headers.md).

NODE-077 keeps both addon binaries unchanged and verifies public timeout
continuation, post-execution close and three Worker boundaries per profile.
Owned pending/queued environments remove their native Context/target; borrowed
navigation survives Worker termination and finishes its HTTP body. Both public
lanes remain nonzero at arrival-close, including an intermittent Rust
TargetStillOpen result. No unconditional stopLoading/retry workaround is added.
[Causal and environment evidence](knowledge/pwright-frame-navigation.md#node-077-causal-close-timeout-continuation-and-worker-teardown).

NODE-076 exposes Frame.goto through the selected Rust executor and a bounded
N-API conversion owner. BaseURL resolution reuses the Context port, and
Response.frame returns the navigated wrapper. New independent frame-navigation
and combined drag-frame-navigation artifacts leave default/drag files intact.
Both final profiles pass 23 paired Frame cases and an independent Rust-close
check, but fail upstream held-response Page.close before candidate comparison;
neither full gate is green. Combined drag/Keyboard/Locator additionally passes
10/10. Rust units (239), Node units (47), types, boundaries and strict Clippy pass.
Header reconciliation, exact errors, Worker parity and remote service installation
remain open. The full objective is unchanged.
[Current FFI boundary and evidence](knowledge/pwright-frame-navigation.md#node-076-public-node-frame-navigation).

NODE-075's Rust `frame_goto` accepts selected typed params and returns a charged
`FrameGotoOutput`, with response parsing/wait-error mapping wholly in Rust.
Its result remains readable after host shutdown and releases its own lease on
drop. It accepts resolved URLs; it does not install the generated command
service or expose an addon/Node method. Context option reconciliation, foreign
allocation budgets, final Node error text and Response.frame mapping remain.
[Host boundary](knowledge/pwright-frame-navigation.md#node-075-typed-rust-host-result-ownership).

NODE-074 provides the default-off navigation 1.2 FrameGoto contract, typed
client/codec/server bindings and nonzero portable Frame response identity.
The selected Rust host executor is not wired to it yet. Node automation logic
and retained addon binaries are unchanged. BaseURL/header reconciliation,
budgeted response/error conversion and correct selected-Frame Node wrappers
must precede the public Frame.goto differential gate.
[Wire boundary](knowledge/pwright-frame-navigation.md#node-074-opt-in-typed-navigation-12-contract).

NODE-073 adds the selected Rust `navigate_frame_native` executor and host
publication fence. It accepts resolved URLs and returns a charged core result,
without foreign-value copies. No public Node method/addon feature is exposed;
generated command adapters, baseURL/header reconciliation, Node error/response
ownership and public pinned gates remain required.
[Execution boundary](knowledge/pwright-frame-navigation.md#node-073-rust-host-dispatch-scheduling-and-cancellation).

NODE-072 adds optional Rust host navigation registration, consuming native
notices and readiness under the Page lock and waking one bounded consumer.
The distinct `frame-navigation` host feature does not add a public Node method
or addon feature. Host dispatch/cancellation, generated Frame command, final
publication fencing and foreign result ownership remain required.
[Integration contract](knowledge/pwright-frame-navigation.md#node-072-ordered-operation-registration-and-actual-rust-host-observation).

NODE-071 implements Rust-only bounded navigation event matching and response
retention. The result borrows admitted JSON and retains the selected portable
Frame ID. The Node facade/addon is unchanged: an admitted production executor,
FFI value allocation, generated Frame command, response wrapper identity and
public differential gates are still required. Offline oracle JS is test-only.
[Core contract](knowledge/pwright-frame-navigation.md#node-071-bounded-navigation-matching-and-response-retention).

NODE-070 adds default-off lifecycle selection in the actual Rust host, using
the existing event loop and explicit multi-Page/retained-owner budgets. The
current selected sink provides readiness only; it cannot fabricate a navigation
Response. The default addon and Node facade are unchanged. Actual operation
subscriptions, generated Frame commands, response/waiter/cancellation state and
N-API exposure remain required.
[Boundary and verification](knowledge/pwright-frame-navigation.md#node-070-bounded-multi-page-registry-and-opt-in-rust-host-selection).

NODE-069 adds opt-in ordered production lifecycle observation, entirely in Rust.
No Node API, generated command or addon selection has changed. The one-Page
owner's clock/notice sink are explicit ports; JS must not run inside its locked
callbacks. Multi-Page host composition and the actual navigation waiter/response
protocol remain required before exposing Frame.goto.
[Implementation and evidence](knowledge/pwright-frame-navigation.md#node-069-opt-in-production-ordered-lifecycle-observation).

NODE-068 fixes native document descendant retirement in the production topology
driver. It adds no Node API or JavaScript algorithm. Live tests distinguish a
portable Frame's survival from its renderer snapshot's retirement; the future
Rust navigation operation must handle that document/session handoff before
publishing its FFI result. The lifecycle adapter is still not selected by the host.
[Evidence and remaining gates](knowledge/pwright-frame-navigation.md#node-068-production-document-commit-topology-retirement).

NODE-067 adds Rust-native lifecycle event normalization with borrowed document,
response, redirect and failure notices. It is default-off and not yet installed
by the Node host. The facade must not implement lifecycle/network matching in
JavaScript; remaining Rust operation, topology initialization, cancellation and
FFI publication gates precede a public Frame.goto compatibility claim.
[Native evidence and integration gaps](knowledge/pwright-frame-navigation.md#node-067-bounded-native-lifecycle-normalization).

NODE-066 adds Rust-only retained Frame navigation access, optional at installation
and transaction-coupled to topology publication. The host/addon does not install
it yet; no public Frame.goto or cancellation guarantee is added. The facade must
ultimately compose this owner with normalized lifecycle, response identity and
native/FFI admission, not reimplement those algorithms in JavaScript.
[Ownership and pending integration](knowledge/pwright-frame-navigation.md#node-066-retained-routes-in-the-production-topology-transaction).

NODE-065 keeps Frame lifecycle snapshots, request accounting and subtree idle
policy in an optional Rust reducer. Pinned internal comparison passes 93/93.
Node only hosts the development oracle; no timer or driver is added to the
runtime facade. Native normalization, retained dispatch and FFI remain pending;
neither addon is rebuilt or selects the new feature yet.
[Boundary and evidence](knowledge/pwright-frame-navigation.md#node-065-optional-normalized-lifecycle-reducer).

NODE-064 starts Frame navigation with exact Rust-native dispatch and borrowed
topology routes that do not need a world. Six local/nested/OOPIF final-state
observations match pinned Playwright; this is not yet public Frame.goto support.
The FFI design requires a distinct Frame command, Rust-owned lifecycle/cancel
state and the actual selected Frame in Response.frame. Addons are unchanged.
[Design, evidence and integration gates](knowledge/pwright-frame-navigation.md).

NODE-063 adds public page.frame and typed matcher forms. Rust owns selection,
glob policy and pinned ancestry; Node supplies actual RegExp/URL/callback values
and weak-wrapper identity only. Original thrown values bypass HostError mapping.
The same addon passes 41/41 lookup observations, three resource groups, Page 7/7,
Frame 18/18 and glob 1647/1647 plus seven checks. No new driver or dependency.
[Contract and evidence](knowledge/pwright-frame-lookup.md#node-063-public-lookup-over-the-rust-kernel).

NODE-062 now passes Context baseURL/navigation and retained-ownership verification:
16/16 public observations, four boundary groups, Page 7/7 and glob 1647/1647 plus
seven checks on the final addon. Node 43 units and strict types pass. BaseURL
remains Rust-owned with scoped Node URL values; no JS driver or launch was added.
Frame.goto/public page.frame and intermittent attachment reliability remain open.
[Contract and evidence](knowledge/pwright-context-creation.md#verification-status-and-remaining-gates).

NODE-061 fixes diagnostic response double-counting and freezes the existing
attachment collector before cleanup, with 12 offline checks. A new asynchronous
wrapper reuses its prebuilt test binary across observer layouts. Original-layout
glob 1647/1647 and Page 7/7 now pass on the unchanged addon; no production fix for
intermittent attachment is claimed. Context baseURL and public lookup remain open.
[Evidence and reproduction](knowledge/pwright-chromium-topology-events.md#node-061-reuse-the-bounded-attachment-collector).

NODE-060 connects Rust glob policy to scoped Node WHATWG URL values through
N-API, with admitted native text copies and no runtime JS driver. The selected
addon was rebuilt; a labelled experiment passes 1647 patterns plus seven boundary
checks, but default attachment/Page regressions still fail. Context baseURL,
public lookup and whole-process parser memory accounting remain separate gates.
[Contract and evidence](knowledge/pwright-frame-lookup.md#node-060-actual-n-api-url-value-conversion-port).

NODE-059 moves URL-glob preprocessing/restoration into admitted Rust helpers.
URL resolution is an explicit borrowed port, not a retained JS driver. The
1647-case offline comparison uses a diagnostic Rust parser; production parser
budgeting, Context baseURL and public N-API lookup remain unfinished. Native
artifacts are unchanged; this does not add a public Node method.
[Contract and evidence](knowledge/pwright-frame-lookup.md#node-059-bounded-glob-preprocessing-and-borrowed-url-resolution).

NODE-058 adds Rust-owned first-match traversal, typed callback-error preservation,
borrowed URL reads and bounded resolved-glob planning. No Node lookup facade is
exposed yet; full matcher conversion, baseURL and identity return remain gates.
The selected/default native artifacts are unchanged in this offline checkpoint.
[Core implementation and evidence](knowledge/pwright-frame-lookup.md#node-058-rust-traversal-and-bounded-resolved-glob-compiler).

NODE-057 records [full Frame lookup semantics and Rust/FFI gates](knowledge/pwright-frame-lookup.md).
The pinned oracle preserves RegExp state and synchronous callback truthiness;
JS callbacks and exceptions must cross N-API faithfully while Rust owns traversal,
glob planning and identity selection. Fourteen oracle-only tests pass; this is
not yet candidate lookup support or whole-package parity.

NODE-056 adds `Frame.name()` through borrowed N-API conversion of a Rust-owned,
bounded navigation snapshot. Name and URL publish atomically; no DOM query,
JS driver or additional background task is involved. Eighteen local/OOPIF
public comparisons pass; lookup and Frame events remain open.
[Contract and evidence](knowledge/pwright-frame-publication.md#node-056-bounded-navigation-names-and-public-framename).

NODE-055 admits synchronous descriptor conversion through actual N-API delivery
and retains explicit Page-tree wrapper leases. Cache-aware enumeration avoids
repeated native boxing; all four creation metadata states are reserved before
native Page creation. A shared 7-case gate also exposed and fixed missing
Page/Frame evaluation user activation through an explicit Rust profile option.
[Current boundary and evidence](knowledge/pwright-frame-publication.md#node-055-admitted-page-descriptors-and-cache-aware-enumeration).

NODE-054 fixes a reproduced native Page leak when the main-Frame wrapper quota
is exhausted. Rust now reserves the root binding before native creation and
moves it through the FFI descriptor; JS performs no second admission or cleanup.
The public stress gate proves zero unexpected native targets and exact resource
rollback. Frame, Mouse and drag comparisons pass on the rebuilt addon; prior
intermittent connection timeouts remain undiagnosed rather than declared fixed.
[Current admission contract and evidence](knowledge/pwright-frame-publication.md#node-054-creation-time-root-binding-admission).

NODE-053 exposes public Frame collections, parent/child identity and detach
metadata through admitted synchronous N-API reads and weak JS wrapper caches.
The real local/OOPIF paired gate passes 10/10; Frame events, names and lookup
remain open. Older checkpoints below describe their historical scope.
[Current API, resource contract and evidence](knowledge/pwright-frame-publication.md#node-053-public-node-frame-collections-and-retained-wrappers).

NODE-052 connects bounded Frame trees to real host topology and pre-reserves
new Page metadata. Native local/OOPIF lifetime observations match upstream;
parent-scoped child detach and paused-OOPIF initialization fixes also repair a
public Node owned-Page navigation failure reproduced on the old addon. The new
selected artifact passes Mouse 44/44, Keyboard 24/24 and drag/Locator 10/10.
Public child-Frame wrappers remain pending.
[Current contract and evidence](knowledge/pwright-frame-publication.md#node-052-live-host-projection-and-paused-oopif-attachment).

NODE-051 implements bounded retained Frame-tree metadata and stages tree changes
on the existing URL publication watermark. There are 27 new unit tests and a
local/sandbox upstream identity probe. Host topology projection and public Node
child-Frame APIs remain unconnected; neither addon was rebuilt.
[Contract, evidence and next gates](knowledge/pwright-frame-publication.md).

NODE-050 adds explicit `drag-runtime` addon selection and binary capability
validation, without changing the default artifact. The same Rust core now passes
44/44 public Mouse, 24/24 Keyboard and 10/10 native drag/Locator Node comparisons.
Frame lifecycle tests pass 18/18 internally; public child-Frame APIs remain open.
[Selection, evidence and remaining work](knowledge/pwright-chromium-drag.md#node-050-native-lifetime-gates-and-selected-node-addon).

NODE-049 implements a Rust-only, default-off runtime lifetime handoff selected
by host `drag-runtime`. Frame focus retains original command admission; Page
owns the subsequent input cancellation lifetime. All 12 native Frame cases match.
No Node facade/addon rebuild or new public child-Frame coverage is claimed.
[Contract, evidence and remaining gates](knowledge/pwright-chromium-drag.md#node-049-optional-frame-to-page-lifetime-handoff).

NODE-048 adds a ten-case native Frame discrepancy gate: 8 match; local/OOPIF
detach during the Page key delay fail (Rust TargetClosed versus oracle timeout).
The underlying command still has Frame cancellation ownership after focus.
Public child-Frame wrappers and addon drag selection remain unimplemented.
[Evidence and required phase transition](knowledge/pwright-chromium-drag.md#node-048-native-frame-gate-exposes-a-lifetime-discrepancy).

NODE-047 adds Rust-host Locator keyboard selection and retained cancellation
binding. Ten host/oracle cases match before close; the Node addon was not rebuilt
and its public 39/44 Mouse baseline remains historical and incomplete.
[Exact scope and remaining gates](knowledge/pwright-chromium-drag.md#node-047-locator-keyboard-selects-the-owning-page).

NODE-046 wires optional Rust Keyboard Escape interception through the existing
Page drag controller and exact cancellation handler. All five native oracle
snapshots match before host close. Locator input and native Page destruction
remain open; addon and public 39/44 evidence are unchanged.
[Details](knowledge/pwright-chromium-drag.md#node-046-optional-rust-keyboard-escape-interception).

NODE-045 rebuilds the default addon with derived Mouse/CDP command capacities:
39/44 public Mouse comparisons, including all 25 raw cases. Drag remains opt-in
on the Rust host; the same five same-turn cases still differ. Logical resource
allowances are not RSS measurements.
[Artifact and budgets](knowledge/pwright-chromium-drag.md#node-045-maximum-click-batches-and-matching-host-admission).

NODE-044 adds host-only `drag-runtime`, reusing native world access and retaining
Page/session cleanup drivers through close. The addon does not select it yet;
no JS driver or new public conformance claim is introduced.
[Ownership and remaining gates](knowledge/pwright-chromium-drag.md#node-044-opt-in-hostsession-mouse-integration).

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

## Status

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

NODE-039 composes Rust drag preparation/capture/native movement and cleanup
(**57** unit tests; **9** native cases). It returns optional admitted data; Page
drag state, a selected Mouse dispatcher and host-owned cleanup quarantine are
still required. No addon rebuild or public **36/41** change.
[Composition contract](knowledge/pwright-chromium-drag.md#node-039-composed-intercepted-movement).

NODE-038 adds retained capture lifecycle to the optional Rust leaf (**44** unit
tests; **6** real Chrome cases). Subscription admission precedes enable; native
receipts survive borrowed-future cancellation and failed disable remains dirty.
No host/FFI selection or addon rebuild occurs. The Page-scoped provider must
compose this owner with Mouse dispatch and teardown before public **36/41** can
be reevaluated. [Design and evidence](knowledge/pwright-chromium-drag.md#node-038-retained-capture-lifecycle).

NODE-037 implements typed intercepted-data admission and shared lease ownership
in the optional Rust drag leaf (**27** unit tests). The native listener suite
still passes **6** cases; only NativeDrag exercises the new captured-data flow.
The Node host does not select this leaf and the addon/public **36/41** evidence
is unchanged. Capture lifecycle and Page integration remain required.
[Current boundary](knowledge/pwright-chromium-drag.md#node-037-admitted-intercepted-data).

NODE-036's optional Rust-owned utility-world drag observer now passes **13**
unit and **6** real Chrome cases. Neither the Node host nor addon selects it.
At that checkpoint intercepted-data admission, per-Page drag state, multi-Frame composition
and teardown remain required before rebuilding/verifying the public API.
[Listener boundary](knowledge/pwright-chromium-drag.md).

NODE-035 adds native factory/Page-route dispatcher selection and retained
standalone button owners. Host integration still requires a shared Page-scoped
provider across per-call routes and explicit persistent-state teardown; it must
not instantiate isolated drag state per command. Default and test-only
passthrough native lanes both pass **22/22**; no Node addon or public discrepancy
improvement is claimed. [Integration boundary](knowledge/pwright-raw-mouse.md#page-scoped-dispatcher-selection-node-035).

NODE-034 adds a caller-polled optional Mouse dispatcher to the input scheduler,
not to the public FFI installation. It separates action results from cleanup,
retains escaped Page readers' admission, and stops later pointer actions after
cleanup failure. Input **81**, native **13**, route/runtime **33** and host
**115 / 6 ignored** pass. The addon is not rebuilt; drag provider, Page selection
and the existing public five-case discrepancy remain open.
[Dispatch ownership](knowledge/pwright-raw-mouse.md#optional-mouse-dispatch-ownership-node-034).

NODE-033 exposes a larger failing Mouse contract: **33/41 -> 36/41** after an
explicit Rust first-poll ordering boundary, with five left-held moves still red.
Mouse/Keyboard reserve their bounded session-local ticket after conversion and
before foreign task spawning. The order covers only the first poll, not whole
operations or later continuations. Shutdown/owner drop wakes queued bodies;
native Mouse retirement retains its independent workers. Host **115 / 6 ignored**
and strict host/addon Clippy pass. [Current evidence](knowledge/pwright-raw-mouse.md#same-turn-input-ordering-node-033).

The final ordering artifact `e65e6d85...17b246` remains **36/41** for Mouse;
Worker/profile **3/3**, Keyboard **24/24**, screenshot **12/12** and lifecycle
**45/45** pass. Typing **30/38** and actions **24/27** fail at attachment, with
5 candidate topology-init and 6 oracle connect timeouts in total. Keep these as
NODE-013 evidence rather than substituting the earlier intermediate passes.

Earlier Mouse addon `e9d9e543...109535` regression: Keyboard **24/24**, typing **38/38**,
actions **27/27**, screenshot **12/12**, lifecycle **45/45**, reload **16/18**.
Shared corpus **53 match / 2 unsupported / 1 oracle-error / 1 setup-error**.
NODE-013 recurs as a 5 s candidate topology-init timeout before the goBack case's
operations; retain the failed gate separately from the upstream history error.
Overall compatibility is red. [Exact artifact outcomes](knowledge/pwright-raw-mouse.md#same-artifact-regression).

NODE-032 Mouse primitives and click/dblclick scheduling match **22/22** pinned
event/state cases. Rust separates operation results from retained-call retirement.
The new addon exposes all six public Mouse methods: **26/26** paired cases and
**3/3** profile/Worker tests pass. Concurrent first-error ordering,
HTML drag interception and final nested allocation/artifact verification
remain explicit gates. JS must not own pointer state or synthesize wheel scrolling.
The optional Mouse 1.0 capability/client/codec/server layers now pass **24**
release tests and independent **3/6/7/10** dependency proofs. The generated wire
chain now connects to a bounded Page route: **18** tests and **22/22** fresh
full-wire browser matches, with a **16-package** closure. Separately,
the new admitted native factory passes **13** tests and **22/22** browser cases.
Its starters return retained execution owners with borrowed result/retirement
methods, not unit futures. The Page route retains them outside cancellable
response futures and returns separate caller-polled retirement receipts. The
new optional runtime adapter waits for actual consumed receipts, not cancellation
request acknowledgement: **15** additional tests and **22/22** controlled browser
cases pass, with default/runtime closures **16/17**. It does not start a pump or
change the plain wire handlers. The safe host now installs up to **16** retained
invocations/independent retirement workers and **8** native execution owners.
Host-session close joins those workers before transport teardown; **9** new
host tests, **22/22** host browser cases and a real close-during-delay probe pass.
The host source closure is **234** and the new addon is `e9d9e543...109535`.
Foreign conversion uses fixed numeric options, admission before owned strings,
and measured future/settlement tickets; final nested allocation audit remains open.
Runtime idle/transport closure alone is not Mouse retirement proof.
The `touchscreen` shared suite currently uses mouseClick and is not Touchscreen
evidence. [Mouse contract and integration tasks](knowledge/pwright-raw-mouse.md).

Raw-CDP-only isolation now reproduces the held-response close failure without
host, navigation service, runtime, addon or reload cancellation. The native
target survives 200/1000 ms after command acknowledgement in both response-boundary
runs. This is browser-boundary evidence, not complete cause attribution or a
production fix; public reload remains **16/18**. The independent reproducer
allows that investigation to continue without blocking unrelated API coverage.
[Raw recipe and evidence](knowledge/pwright-page-reload.md#minimal-raw-cdp-isolation).

The reload close-boundary investigation now has a bounded, loader-correlated
native control: direct Target.closeTarget can acknowledge while the target
remains after reload timeout, bypassing host Page-close disposal. This is a
test-only diagnostic, not a production workaround or parity pass. Host **99**
offline tests and strict release Clippy pass; addon and original red gates are
unchanged. [Evidence and next decision](knowledge/pwright-page-reload.md#causal-native-close-diagnostic).

Latest same-artifact regression on `1319fa02...ded25dc1`: **38/38 typing**,
**27/27 actions**, **12/12 screenshot**, **45/45 lifecycle**, **16/18 reload**.
Eight previously filtered held-modifier typing comparisons are now exercised.
Reload's original arrival-boundary close failures remain; older intermittent
attachment/window failures did not reproduce but are not declared fixed.
[Full evidence](knowledge/pwright-raw-keyboard.md#same-artifact-post-keyboard-regression).

NODE-031 adds selector-free Rust raw Keyboard primitives, verified by **14/14**
real-Chrome event/state cases plus **42** input and **103** Locator unit tests.
Keyboard 1.0 capability/client/codec/server now pass **19** contract tests and
independent **3/6/7/10** package closure proofs. The optional typed native backend
adds eager bounded admission and shared Page-state injection, passing **18** tests
and **14/14** Chrome cases with an **8**-package closure. The separate Page wire
composition now adds **11** tests and **14/14** full wire-to-Chrome comparisons
with a **16**-package closure. Aggregate Page-input selection is now independent
of Locator, with clone/transaction-retained state admission. Host source opts into
32 retained Interleaved states; **89** host tests and a real attach/close resource
gate pass. The opt-in controlled adapter now passes **18** composition tests,
including 7 exact-route lifecycle/deadline tests, with default/runtime dependency
closures **16/17**. A fresh regenerated-wire comparison matches **14/14** cases.
Host routing now snapshots and budgets the exact retained Page client/input.
All five public `page.keyboard` methods pass **24/24** paired cases on addon
`1319fa02...ded25dc1`. Host **95**, root **295**, Node contracts **31** pass;
host closure is **229**. Shared coverage is **52/3/2**, no other errors.
[Integration gates](knowledge/pwright-raw-keyboard.md).

NODE-030 now exposes Rust-backed `page.reload()` through navigation 1.1 and FFI.
The new addon `8d973a18...e53930b` moves both original reload cases to match:
shared **48 match / 7 unsupported / 2 oracle-error**, no other errors. Ordinary
reload, POST method/body and timeout cases pass; the full reload gate is **16/18**.
Page close at held-response arrival fails on both oracle and candidate, while
a separate post-commit close case passes on both with the body still held.
The new artifact's complete lifecycle gate is **44/45** on window geometry;
its screenshot gate is **0/12**, failing at attachment before image comparison.
Earlier milestones below remain historical evidence, not fresh green results.
[Reload contract and remaining gates](knowledge/pwright-page-reload.md).

Working implementation design for the user-authorized Node compatibility and
differential-verification goal; not a claim of separate design sign-off.
The first local attach/topology implementation and differential tests now exist.
See [the implementation checkpoint](#implementation-checkpoint-2026-09-06) for
verified behavior and remaining gates. Full Playwright compatibility is not claimed.

Current milestone: Node `Locator.pressSequentially` and `type` invoke typed
Rust 1.23/2.6 Locator commands. The new opt-in interleaved Page input artifact
passes **30/30 typing + 27/27 actions**, including the earlier overlap and
NODE-022 Shift examples. Its own shared gate completes 57 cases at **46 match /
9 unsupported / 2 oracle-error**, with no other errors. Broader native input
cancellation and Frame/timeout coverage remain open. The latest full lifecycle
rerun passes **45/45 in 192.715 seconds** after separating the aggregate suite
budget from a new independent per-case watchdog; no API timeout or native code
changed. Earlier lifecycle failures and intermittent screenshot attachment
remain evidence, not resolved bugs. See [supervision evidence](#node-013-lifecycle-supervision-evidence-2026-09-07).
The full goal is not complete. Public
Locator.evaluate and Page screenshot are also already exposed; the paragraphs
below preserve historical milestone evidence, not current availability.
[Typing contract and discrepancy](knowledge/pwright-sequential-typing.md).

Earlier milestone: the safe host now selects bounded native-destruction receipts for its
single-command Locator evaluation. The formerly failing Context-close gate
passes in two complete 5/5 real-Chrome runs, plus an isolated repeat. The final
epoch-guarded run is 4/5 because its queue case fails during 5 s attach setup;
Context cleanup still passes, and an isolated queue diagnostic passes. Public
Node Locator.evaluate remains unavailable pending Node reference/conversion and
settlement integration. See the
[native-retirement checkpoint](#native-destruction-receipt-checkpoint-2026-09-07).

Final public regressions on the epoch-guarded addon remain non-green: Locator
29/32, evaluation 17/38, lifecycle watchdog after 11 passes / 5 failures,
actions 25/26 (NODE-022), shared 41 matches / 14 unsupported / 2 oracle errors.
Initialization timeouts affect both candidate and oracle; their cause is still
unresolved. No increased timeout, skipped case or browser restart hides them.

The Node package is a conformance instrument for Pwright Rust v2. It is not a
second browser engine, a copy of Playwright's private protocol, or permission to
move browser behavior into JavaScript.

## Objective

Provide a Node.js package whose supported public surface can be used like
Playwright:

```ts
import { chromium } from "@shuozeli/pwright";

const browser = await chromium.connectOverCDP(endpointURL);
const context = browser.contexts()[0];
const page = context.pages()[0];
await page.goto(url);
console.log(await page.title());
await browser.close();
```

Every browser command, object transition, timeout, cancellation decision,
resource admission check, protocol mapping, and CDP interaction in that flow
must execute in Rust. JavaScript owns only the public ergonomic facade,
`EventEmitter` integration, argument-shape forwarding, call-site stack capture,
and loading the correct native artifact.

The package must make it cheap to execute the same fixture against upstream
Playwright and Pwright and report semantic discrepancies.

## Compatibility contract

Compatibility is measured in levels. A method is never marked compatible from
its name alone.

| Level | Requirement | Evidence |
| --- | --- | --- |
| C0 import | Equivalent ESM and CommonJS exports exist | import tests on every supported Node version |
| C1 shape | Public object, method, overload, option, and TypeScript shapes match the declared upstream version | type fixtures and runtime argument tests |
| C2 value | Success values and normalized errors agree | paired golden assertions |
| C3 lifecycle | object identity, event order, cancellation, timeout, close, and disconnect behavior agree | deterministic state-machine fixtures |
| C4 browser | the same real-browser fixture agrees against the pinned upstream Playwright oracle | differential Chromium lane |

The first package release may contain a documented subset. Declared unsupported
async methods return rejected Promises with `UnsupportedOperationError`;
unsupported synchronous methods throw that error. They must not silently
approximate behavior. Methods outside the documented surface may be absent.

### Explicit non-goals

- Playwright's private client/server wire protocol is not an API target.
- Playwright's internal `ChannelOwner`, dispatcher, validator, or GUID layout is
  not copied.
- The package does not import, spawn, or embed the Playwright Node driver.
- The initial package does not download or bundle Chromium.
- Firefox and WebKit are not claimed until their Rust backends pass the same
  compatibility levels.
- Test-runner fixtures, reporters, and Playwright Test configuration are a
  separate product surface from the browser library.

## Upstream evidence

The pinned local upstream checkout is
`microsoft/playwright@de214f440b7e34937fe4886f046b78b757136087`
(`1.63.0-next`). The package-level baseline used by the existing golden suite is
Playwright `1.58.2`; that remains the first differential oracle until the
goldens are deliberately re-recorded.

Relevant upstream boundaries are:

- `packages/playwright-core/src/client/playwright.ts`: root browser types;
- `packages/playwright-core/src/client/browserType.ts`: overload normalization
  and `connectOverCDP`;
- `packages/playwright-core/src/client/browser.ts`: contexts, connection state,
  close, and disconnected event;
- `packages/playwright-core/src/client/channelOwner.ts`: object identity,
  listener-driven subscriptions, API-call errors, and recursive disposal;
- `packages/playwright-core/src/client/connection.ts`: pending calls and event
  delivery;
- `packages/playwright-core/src/server/dispatchers/`: server object boundary;
- `packages/playwright-core/src/server/chromium/chromium.ts`: CDP attachment and
  externally owned browser cleanup.

Playwright normally keeps its public client and browser server behind a channel
even when they live in one Node process. Pwright keeps the same conceptual
separation but uses its typed Rust v2 runtime and capability services instead
of Playwright's Node dispatcher protocol.

## Product and package topology

```text
application code
    |
    v
@shuozeli/pwright              small ESM/CJS Playwright-shaped facade
    |                          EventEmitter, overloads, call-site stack only
    v
pwright-node-addon             Node-API exports and JS/Rust value conversion
    |
    v
pwright-node-host              safe Rust embedded-session composition root
    |
    +--> selected pwright-client-* facades
    +--> selected Chromium composition leaves
    +--> pwright-runtime-tokio executor adapter
    +--> pwright-provider-cdp attach-only provider
    +--> pwright-core resource ledger
```

The existing `pwright-js` crate retains its current meaning: audited JavaScript
snippets injected into pages. It is not the Node binding.

### Why two Rust crates

`pwright-node-host` must compile and test without Node headers or a JavaScript
runtime. It owns the embedded session, typed object registry, and calls into the
selected Pwright v2 services. `pwright-node-addon` is deliberately thin and may
depend on `napi`/`napi-derive`; no other Pwright crate may depend on it.

This split lets Rust tests prove lifecycle and resource behavior before N-API
conversion, and prevents Node-specific types from leaking into reusable core
logic.

### Install-time composition

The differential package selects a broad, explicit Chromium compatibility
profile because its purpose is to exercise cross-capability parity. That must
not become the only distribution shape.

Later packages may compile narrower addons, for example navigation-only or
locator-only profiles, from the same `pwright-node-host` features. Omitted Rust
features must be absent from their dependency trees. Separate packages must not
attempt to share Rust objects across independent native-addon dynamic
libraries.

## Native boundary

Node-API is selected over V8 APIs, `node-gyp` C++ wrappers, process RPC, and a
JavaScript CDP implementation. Node-API provides a stable ABI across supported
Node major versions; `napi-rs` supplies Rust bindings and Promise conversion.

The first supported runtime range is Node 20 and newer, matching the upstream
Playwright package. Prebuilt artifacts are per operating-system, architecture,
libc where applicable, and Node-API level, not per V8 version.

### Export model

The addon exports native session and object references, not Playwright-facing
classes:

```text
NativeSession
  connectOverCdp(options) -> NativeBrowserDescriptor
  invoke(objectId, operation, typedOptions) -> Promise<typed result>
  subscribe(objectId, event) -> subscription token
  unsubscribe(subscription token)
  close(reason) -> Promise<void>
  metrics() -> bounded diagnostic snapshot
```

The concrete implementation may expose dedicated typed methods rather than a
single stringly `invoke`, but method selection and payload validation must end
in typed Rust commands. JavaScript must never construct raw CDP messages.

Every descriptor contains the portable object ID, kind, parent ID, and only the
bounded initializer fields needed by the public facade. JavaScript caches one
facade per `(session, object ID)` so repeated events preserve object identity.

### Async execution

- N-API entrypoints return Promises and never block the JavaScript thread on
  browser I/O.
- A session owns its Rust driver task, cancellation source, and terminal state.
- The task is created only by `connectOverCDP`, is stored in the session, and
  is joined during explicit close.
- The napi-rs Tokio runtime is an execution adapter; browser state and resource
  ownership never live in a global JavaScript or Rust singleton.
- A queued Promise that is abandoned by JavaScript does not imply protocol
  cancellation. Timeout or `AbortSignal` forwarding must invoke an explicit
  Rust cancellation path with the exact call identity.

### Events and backpressure

Playwright objects are `EventEmitter`s, so the JS facade owns listener methods.
The first listener enables the corresponding Rust subscription; the last
listener disables it.

Rust owns the authoritative bounded event queues. Delivery into JavaScript uses
a Node-API thread-safe function with a finite nonblocking queue. An enqueue
failure is observable and terminates the affected subscription or session
according to the selected loss policy; it is never converted to an unbounded
buffer. No Rust worker thread calls a JavaScript value directly.

### Object lifetime

- `Browser -> BrowserContext -> Page -> Frame -> Handle` remains the portable
  Rust ownership tree.
- Native and JS wrappers contain stable object IDs, not borrowed references to
  other wrappers.
- JavaScript garbage collection releases only local wrapper memory. It must not
  send browser protocol traffic from a finalizer.
- `close()` and `dispose()` are explicit, idempotent lifecycle operations.
- Parent disposal invalidates descendants in Rust before a later FFI call can
  reach CDP.
- A `FinalizationRegistry` may remove facade-cache entries, but is never a
  correctness or cleanup mechanism.

### Browser ownership

The first vertical slice implements only `chromium.connectOverCDP`. The browser
process and binary are externally owned. `browser.close()` closes the Pwright
connection and emits `disconnected`; it must not terminate that Chromium
process. Fresh launch, executable discovery, managed download, and browser
process close remain independent future capabilities.

### Errors

Rust errors cross the boundary as a structured record:

```text
name, code, message, operation, objectKind, objectId,
failureKind, retryable, optional bounded details
```

The JS facade maps this record to stable exported error subclasses and prefixes
the call-site stack in the same general location as Playwright. Differential
tests compare error class/code and normalized message fields, never unstable
native stack addresses or timing text.

Panics are defects, not expected error flow. Public Rust calls return `Result`;
the addon boundary converts them to rejected Promises. A caught panic may be
reported as `PwrightInternalError`, but panic catching is not treated as a
process-safety boundary.

### Binary values

Screenshots, traces, and downloaded artifacts cross as Node `Buffer` values.
The first implementation transfers a bounded owned allocation with one explicit
copy if required by Node-API lifetime rules. Borrowed browser buffers and
unbounded base64 strings are forbidden. Zero-copy external buffers are a later
measured optimization and must have a teardown proof before adoption.

## Public surface roadmap

### VS0: binding and package contract

- ESM and CommonJS import parity;
- native artifact loader with clear platform error;
- `chromium.name()` and exported compatibility metadata;
- safe Rust host tests and addon smoke test;
- no browser operation yet.

### VS1: attach and topology

- `chromium.connectOverCDP(endpointURL, options?)`;
- `Browser.isConnected`, `Browser.version`, `Browser.contexts`, and
  `Browser.close`;
- default attached `BrowserContext.pages`;
- Page identity and main Frame identity;
- external Chrome survives close.

### VS2: read and navigate

- `Page.url`, `Page.title`, `Page.content`, `Page.goto`;
- timeout and error normalization;
- same-document and response-null behavior;
- differential fixtures against Playwright 1.58.2.

### VS3: locators and input

- `Page.locator` and supported locator composition;
- actionability, click, fill, press, check, uncheck, selectOption;
- strictness and timeout discrepancies become blocking test failures.

### VS4: evaluation and handles

- by-value `evaluate` first;
- retained JSHandle/ElementHandle only with the handle lifecycle feature;
- explicit disposal, argument adoption, and cross-context errors.

### VS5: events and artifacts

- Page/popup/network events with listener-driven subscriptions;
- screenshot and trace Buffers;
- bounded event and binary pressure tests.

## Differential verification

The existing `tests/` harness becomes the single fixture source with three
adapters:

```text
spec fixture
  +--> playwright-adapter      upstream oracle
  +--> pwright-node-adapter    new FFI package under test
  +--> pwright-adapter         legacy gRPC migration signal
```

Each run records:

- upstream package and browser versions;
- Pwright commit and compiled capability manifest;
- normalized values, errors, and ordered events;
- elapsed phase boundaries without asserting wall-clock equality;
- Rust resource metrics before attach, steady state, and teardown;
- Pwright process RSS separately from Chromium process RSS.

The comparator classifies differences as `value`, `error`, `ordering`,
`lifecycle`, `timeout`, `resource`, or `unsupported`. A known discrepancy needs
an issue ID, exact expected/actual evidence, affected upstream version, and an
expiry or review condition. No blanket snapshot update can convert a regression
into compatibility.

### Determinism rules

- The same dedicated externally owned Chromium instance is used serially for
  oracle and candidate runs.
- Each case begins with a clean context/page state and ends with zero leaked
  Pwright-owned objects, calls, subscriptions, and event bytes.
- URLs, ports, object IDs, timestamps, user-agent version fragments, and stack
  paths are normalized only by named field-specific rules.
- Arbitrary recursive string replacement is forbidden.
- Tests follow Arrange-Act-Assert and contain one public action under
  comparison.

## Security and resource gates

- CDP endpoints are accepted only from explicit caller input. The package does
  not discover or expose a debugging port.
- Loopback CDP remains loopback-only. No addon option binds or proxies it to
  Tailscale, LAN, or public interfaces.
- Endpoint strings, headers, error details, selectors, evaluation source,
  attachments, event queues, live objects, pending calls, and concurrent
  operations all have configured bounds before attachment.
- Diagnostic output redacts authorization headers and never includes browser
  cookies or response bodies by default.
- `metrics()` returns counts and byte totals, not retained payload contents.

## Rejected alternatives

### Spawn Playwright's Node driver

Rejected because it preserves the memory/process behavior this project is
intended to remove and would not verify the Rust core.

### Implement CDP in TypeScript

Rejected because browser semantics, pending calls, and object lifetime would be
duplicated outside Rust and discrepancies could hide in the second engine.

### Copy Playwright's private wire protocol

Rejected because it couples Pwright to undocumented internals without helping
public API parity. Pwright's typed capability protocol remains independently
versioned.

### Use synchronous FFI calls for async browser commands

Rejected because it blocks Node's event loop and creates deadlock risk when
events or Promise continuations require the JavaScript thread.

Environment teardown is a distinct barrier: after Node has disabled JavaScript,
the environment cleanup hook waits only for Rust operations and native I/O,
never for a JS callback. It runs before napi's earlier-registered runtime
teardown hook. This is not a synchronous implementation of a browser API.

### Rely on JavaScript GC for remote cleanup

Rejected because finalization timing is nondeterministic and cannot preserve
ordered disposal or await failure.

## Acceptance gates

Before VS1 is called complete:

1. `pwright-node-host` has no dependency on `napi`, JavaScript packages, the
   legacy gRPC server, or upstream Playwright.
2. `pwright-node-addon` contains conversion and delegation only; browser/CDP
   command implementations are rejected in review.
3. The package contains no postinstall browser download and no dependency on
   `playwright` or `playwright-core` outside dev-only differential tests.
4. Rust release tests, strict Clippy for the new crates, Node type tests, ESM
   smoke tests, and CommonJS smoke tests pass.
5. Three consecutive dedicated-Chrome differential runs pass.
6. Closing Pwright leaves the externally owned Chrome alive and leaves no tabs,
   objects, pending calls, subscriptions, or addon driver tasks attributable to
   the case.
7. An unsupported overload or option fails explicitly and is recorded as an
   unsupported compatibility cell.

## Open decisions after VS1

- exact package names before registry publication;
- supported prebuilt target matrix and minimum Node-API level;
- whether narrower compatibility packages are published or are build features
  only;
- stable serialization form for future dynamic capability plugins;
- whether Playwright Test compatibility is a separate package or remains a
  consumer of the browser-library compatibility layer.

## Implementation checkpoint: 2026-09-06

The completed Rust foundation was committed locally as `f76e3f2`; it was not
pushed. Node binding work is a subsequent, uncommitted implementation slice.

### Current code and executable evidence

- `crates/pwright-node-host`: safe Rust composition, bounded endpoint discovery,
  connection timeout, topology snapshots, explicit session teardown and actual
  resource-ledger metrics. It has no N-API or legacy gRPC-server dependency.
- `crates/pwright-node-addon`: Node-API 8 conversion layer, pinned `napi` 3.12.2,
  `napi-derive` 3.6.3, `napi-build` 2.4.1. Async methods clone Rust-owned state
  before creating futures; they never borrow JS-owned native objects across
  an await. Browser behavior remains in the safe host and existing v2 services.
- `packages/pwright`: private, local-build-only `@shuozeli/pwright` ESM/CJS
  facade with TypeScript declarations. No runtime package dependencies,
  postinstall script, browser download or browser-process launcher.
- `ChromiumTopologyEventDriver::initialize_snapshot`: initializes an empty
  default Context without awaiting an unrelated event; drains the initial
  target announcements with a caller-supplied finite work budget.
- `CdpConnection::io_finished`: distinguishes cancellation notification from
  completion of the actual reader/writer cleanup. Explicit close waits for
  this barrier before resolving; it never sends `Browser.close` to Chrome.

The exposed subset is `chromium.name/connectOverCDP`, Browser
`browserType/version/isConnected/contexts/close`, BrowserContext
`browser/pages`, Page `context/mainFrame`, Frame `page/parentFrame`, and
`Browser.disconnected`. `browser.pwrightMetrics()` is a Pwright-only diagnostic
extension, not a Playwright method. Both endpoint overloads and HTTP-discovered
and direct browser WebSocket endpoints are covered. Only the main Frame is
exposed; child-frame traversal is not implemented in this facade yet.

The package test lane uses the already pinned `tests/` Playwright 1.58.2 as its
oracle. On this Mac (Node 26.7.0, Chrome 152.0.7977.76, darwin-arm64), three
consecutive six-case real-browser runs passed. They compare initial topology
and wrapper identity, concurrent close/event-once behavior, WebSocket attach,
malformed-URL error class, repeated attach/close ledger release, and forced
Worker environment teardown. Existing unrelated Chrome pages were preserved.
The runner treats a background Rust panic on stderr as failure even if all Node
assertions passed. TypeScript, four import/argument contract tests, and the
safe-host dependency boundary check passed. Rust release suites passed for the
CDP connection, Chromium composition, and safe host; strict release Clippy
passed for those crates and the addon. Fourteen preexisting opt-in Chromium
composition tests were not enabled by that unit-suite command.

Every explicit close in the repeated-session test measured zero live objects,
pending calls, topology drivers, active resource leases, live resource scopes,
and retained event bytes. This is lifecycle evidence, not a claim that process
RSS is lower than Playwright: allocator/RSS attribution remains unmeasured.

### Discrepancies and unfinished gates

These are tracked local issue IDs, not blanket golden exceptions. All target
Playwright 1.58.2 and must be reviewed before expanding the compatibility claim.

| ID | Current boundary | Required next evidence / review condition |
| --- | --- | --- |
| NODE-001 (resolved) | `timeout: 0` disables both the overall attach deadline and ordinary CDP response timers. Positive timeouts remain unsigned integer milliseconds and bound both layers. | Keep the zero-to-`None` transport regression and positive-timeout validation mandatory. Operation-specific cancellation and lifecycle shutdown remain independent contracts. |
| NODE-002 | Headers, proxy/redirect behavior, slow motion, custom logger, credentials/URL fragments, close reasons and other options are unsupported. HTTP redirects are refused. | Add one option only with paired behavior/error tests; no silently ignored keys. |
| NODE-003 | Errors currently cross as bounded `PWRIGHT_*: message` envelopes. Malformed URLs map to TypeError; full upstream messages/stacks and structured details are not matched. | Implement the structured error boundary and named normalizations; compare complete supported error fields. |
| NODE-004 | The host still selects the aggregate Chromium composition, not a feature-minimal native dependency graph. The first three slices moved `ChromiumSessionInitializer`, synchronous Frame event/URL/ownership projection, and bounded Frame lifecycle/navigation-wait observation into independent leaves. Session initialization and observation have exact five-package closures; `pwright-plugin-chromium-frame-lifecycle` has an exact ten-package closure. The aggregate compatibility-reexports them while host adapters import the leaves directly; aggregate-coupled host files fall from 55 to 39. The transitional boundary remains 236 packages for the default host and 251 for the default addon, and the aggregate has 69 direct normal dependencies with `default = []`; no size reduction or narrow distribution is claimed yet. | Continue the inventory by moving contracts and implementations behind capability-owned leaves, add a narrow Node assembly crate, and only then remove the aggregate. Each slice must preserve ownership/lifecycle tests and make the boundary gate prove both a smaller exact package set and absence of unselected server/event capabilities before any narrow package claim or publication. |
| NODE-005 | All 19 suites / 57 unchanged cases use fresh owned oracle and candidate Contexts, without golden rewrites or JS automation fallback. Semantic Locator artifact `edd8ba96...b0c9b7` completes 41 matches / 14 unsupported / 2 oracle errors with no value/candidate/setup/cleanup errors. Public getByText/getByRole add five matches over the world-readiness artifact's 36/19/2. The prior `e5076407...65d48` report remains 31/19/2 plus 5 setup errors. | Expose remaining APIs through budgeted Rust bindings. Preserve artifact-specific evidence. Shared successes do not prove all semantics: NODE-020/021/022/023/024 require separate concurrency, navigation and cleanup comparisons. Every unsupported/error classification remains non-passing. |
| NODE-006 | Node >=20 is the intended package range. CI now defines unit/type lanes for Node 20/22/24 and a Linux Node 20 native build/load lane; local native verification is Node 26.7.0 / macOS arm64. There are no prebuilt releases. | Require green workflow evidence and add the intended prebuilt platform matrix before distribution claims; unsupported artifacts must keep failing explicitly. |
| NODE-007 | One bounded session signal now publishes native URL watermarks and disconnect notification on the JS loop. General listener-driven subscriptions and public Page/Context/navigation events are not exposed. | Extend the ordered Rust publication mechanism with explicit event payloads, listener admission and overflow/ordering fixtures in VS5; a metadata watermark is not a general event implementation. |
| NODE-008 | Empty-default-Context initialization is covered with the fake CDP backend, not a zero-tab live Chrome run. Forced Worker exit proves process survival and reattachment, not post-exit access to that worker's ledger. | Isolated dedicated-browser and environment-cleanup instrumentation before closing every VS1 lifecycle gate. |
| NODE-009 | Synchronous `Page.url()` and main `Frame.url()` now read the Rust-owned published view. The paired blocked-JS regression first reproduced early native visibility, then passed with fixed-watermark publication on Node callbacks. | Keep blocked-loop and command-return regressions mandatory. Ordering relative to future public navigation/close events remains NODE-007 work; this is not an all-orderings or child-Frame parity claim. |
| NODE-010 (resolved) | `title()` now dispatches the fixed `FrameTitle` DOM-read 1.1 command through the exact utility world. Main-world prototype changes no longer affect its result. | The paired prototype-interference assertion passed on real Chrome and its TODO marker was removed. Keep it as a mandatory regression; this does not claim every title/navigation race has been verified. |
| NODE-011 | The current async exports now have environment-owned admission, boxed-future and native result/error reservations through JS conversion or environment reclamation. Blocked-JS and forced-Worker tests cover this boundary. | Keep the pressure regressions mandatory; every added async export must use this boundary and declare payload/future limits. General events, Buffers, historic retained metadata, user-owned V8 values and process RSS need their own evidence; zero counters are not a whole-process memory proof. |
| NODE-012 | Context creation/close, `BrowserContext.newPage()`, ordinary `Page.close()` and NODE-415's no-options / `baseURL` `Browser.newPage()` use Rust services. The hidden Browser.newPage Context has an exclusive bounded Rust owner and failure compensation. The Node lifecycle-events attachment selects exact native target-destruction confirmation; the configured 20-check polling policy is only the non-observed fallback. Remaining Browser.newPage Context options, before-unload, close reasons, `Page.isClosed()` and public Page/Context close events remain unsupported or absent. | Keep native-confirmation, lifecycle, implicit-Context compensation and resource-admission regressions mandatory. Run the official real-Chrome Browser.newPage selector rows after the dedicated browser is empty. Public event ordering and shutdown during partially completed creation need dedicated evidence; ready-owned-Context Worker teardown is not proof of every initialization race. |
| NODE-013 | Earlier topology-setup timeouts and the bounded reproduction outside Node-API remain recorded. Paired-prerequisite and concurrency artifacts passed full lifecycle/evaluation lanes, but action-stage `eae32ce9...41af9e` again hit the lifecycle 180-second supervisor and had two oracle attach timeouts in evaluation (36/38). A filtered Worker diagnostic passed only after removing the exact fixture left by the killed lane; it does not establish the original failure's cause. | Intermittent root cause remains open. Preserve artifact-specific full-gate outcomes; shorter timings and filtered passes are not fixes. TCP_NODELAY did not demonstrate improvement and remains reverted. Keep original deadlines and readiness/resource/cancellation fences; no silent retries or shared Chrome reset. |
| NODE-014 | The owned-Context profile maps default viewport, screen, scale, native window bounds, focus and media. A Locator-stage full regression captured initial outer dimensions 0x0 versus Playwright's 1282x800; a subsequent full run passed, without a window-initialization fix. Download defaults, headless fonts and the complete option matrix remain incomplete. Borrowed Context settings are not overwritten. | Preserve the failed first-document snapshot and diagnose window-readiness ordering on exact owned targets; do not call this intermittent discrepancy fixed because a later run passes. An independent Playwright observer can itself change settings and cannot be the sole evidence. Context options still reject before mutation. |
| NODE-015 | Official Playwright 1.58.2 times out in both shared history cases on dedicated Chrome 152. The diagnostic records `BackForwardCacheRestore`, the expected restored title and `readyState=complete`, with no new restored-document `load`. The installed oracle ignores the frame-navigation type and clears lifecycle state on commit; its usual launcher disables BFCache, but this externally owned Chrome profile does not. | Keep these cases classified as oracle errors with the candidate's `goBack` unsupported evidence alongside them. Review the reference-browser profile and desired BFCache contract before implementing history parity. Do not restart/reconfigure shared Chrome, silently change `waitUntil`, extend timeouts or mark the oracle failure as a match. |
| NODE-016 | Repeated-attach measurement is not yet isolated from utility-world creation on preexisting Pages. Installed Playwright names utility worlds using each new Page GUID and creates them while attaching existing Frames; the Rust profile derives names from connection-local Page IDs. Test teardown verifies owned targets and native counters, not surviving external-page worlds or renderer RSS. | Measure world inventory and renderer memory across attach/detach cycles on an exclusively test-owned stable Page before an RSS comparison. Evaluate a persistent oracle connection with fresh per-case Contexts if repeated oracle attachment perturbs the reference workload; do not reload/close unrelated Pages or assume zero Rust counters proves no browser-side retention. |
| NODE-017 (timing fix implemented) | The five reproduced eager-copy failures now match with bounded Node-thread staging: immediate/microtask/later-getter mutation, detachment and resizable-buffer shrink. Later-immediate mutation, growth and fixed-subarray behavior also match. Ordinary structure remains a synchronous snapshot; an injected `setImmediate` materializes owned binary bytes before Rust execution. A 1 MiB aggregate backing-capacity ceiling complements the existing payload budget. | Keep all paired assertions and prepared/running/queued Worker checks mandatory. The Locator-stage rebuilt addon passes evaluation 38/38; older setup failures remain NODE-013 evidence. Never retain a borrowed pointer or JS reference across a Rust await. This scoped timing fix does not close NODE-018 or the overall compatibility gate. |
| NODE-018 | The bounded Node value profile is not complete serializer parity. Unpaired UTF-16 strings, SharedArrayBuffer-backed arguments and invalid Date arguments explicitly reject; the portable value model is UTF-8. Source normalization failures and complete browser exception stacks remain incomplete. Browser DOM references, cross-realm/prototype interference, throwing result getters and `toJSON` behavior still need paired coverage. | Extend the portable model and conversion contracts only with lossless encoding, admission and teardown proofs. A supported scalar/typed-array round trip is not full `evaluate` compatibility. Keep NODE-003's structured error gate open and do not silently replace Unicode or coerce unsupported values. |
| NODE-019 | Public Page/main-Frame Locators support ordinary CSS, composition, string/Locator/visible filters, seven semantic `getBy*` constructors with string/RegExp matchers, fourteen reads and thirteen action entry points. Queries/actions reuse existing typed Rust services. RegExp filters, `toString`, Locator evaluation/Handles, child-Frame APIs, mutable selector configuration and Playwright's extended selector DSL remain unimplemented. Passing `css=`, XPath or extended syntax to the raw CSS leaf is not selector-language compatibility. | Preserve the expanded 32-case Locator comparison and budget-before-copy proofs. Accessible-name precedence and global role-name regex now have scoped fixes, not complete ARIA/selector parity. NODE-021 has an entry-readiness fix; The NODE-022 sample now passes under Node's opt-in interleaved policy; broader input cancellation remains open. Do not translate browser behavior into the JavaScript facade. |
| NODE-020 (scoped fix) | The original serial Frame lane blocked title/evaluate behind a waiting Locator. The Node profile now explicitly selects eight controlled-runtime target permits and eight DOM reads per Frame inside unchanged connection/pending/byte bounds. The original mandatory comparison and new evaluate/read/close-pressure regressions pass in the expanded 22/22 Locator lane. Default Rust consumers remain serial. | Preserve the ordinary completion-order assertion, capacity/teardown proofs and all four live lanes. Arbitrary navigation/input interleavings are not verified by these read/evaluation tests. Do not remove the scheduler globally, normalize timing, or infer complete operation-ordering parity. |
| NODE-021 (entry precondition implemented) | Enter/form navigation returned after commit while the replacement utility-world route was still absent. The optional Rust composition now waits for the current required world inside the controlled command, with bounded admission, unchanged deadlines and lifecycle cancellation. The first readiness artifact passes the original Enter comparison plus concurrent post-Enter reads and three successive commit-navigation/action comparisons. | See the bounded-readiness contract below. This is not an atomic document lease after dispatch, complete navigation parity, or permission to retry committed input/arbitrary evaluation. No sleeps or browser behavior are added to the JS facade. |
| NODE-022 | Historical default-exclusive mismatch: timed-out delayed `press('Shift+Z')` clears Shift in Rust, while pinned Playwright retains it. Node now opts into Page-owned interleaved state; the alias-corrected artifact passes this unchanged assertion, including TimeoutError, text and the next native key event. | Scoped fix verified by the 27/27 action gate. Default Rust cleanup remains unchanged. Keep broader native enqueue/ack cancellation, physical-key modifier changes and lifecycle retirement gates open; do not infer complete keyboard parity from this one example. |
| NODE-023 (scoped fix verified) | The action-stage Locator lane passed 21/22: one of ten reads cancelled by Page close returned ResourceExhausted instead of TargetClosed. A separately retained pressure probe reproduced the exact source: `topology/lifecycle request capacity exceeded (8)`. Every cancelled command was trying to enqueue another completion fence. | The host now skips extra publication only for authoritative TargetClosed results or an already disconnected host. Five offline tests retain full-queue closure, success waiting, nonterminal-error publication and unsuppressed live fence failures. Rebuilt Locator 22/22, evaluation 38/38 and lifecycle 44/44 pass without capacity growth or generic error normalization. Broader gates remain non-green. |
| NODE-024 | Final world-readiness lifecycle regression fails during the candidate half of retained-URL/disconnect comparison: the independent Playwright controller's next navigation reports interruption by the previous `second.html#last` navigation after Rust goto and disconnect returned. A three-pair isolated event-order probe passes, but observes delayed controller load delivery after Rust disconnect. The new caller-held HTTP body comparison passes: goto load remains pending while independent reads complete, then resolves after body release. | Preserve the full failing gate. The new comparison proves scoped load waiting, not the cause of cross-connection ordering. Diagnose navigation publication/loader ordering before assigning cause; delayed observer delivery is not proof that Rust returned before Chromium loaded. Do not insert sleeps, observer fences or silent navigation retries to turn this comparison green. |

NODE-027 (scoped fix verified): running Locator Worker teardown now uses sealed,
acknowledged owned-session retirement. The final 23-case lane and supervisor
pass, including borrowed default-Context Page/Promise survival. Keep this gate,
epoch/partial-failure tests and the separate release-ack fence mandatory. This
does not resolve NODE-013/014 or claim full lifecycle/Playwright parity; see the
[host integration checkpoint](#node-027-host-integration-and-worker-verification-2026-09-07).

### Initial VS2 command-path checkpoint (before DOM-read 1.1)

The host now uses `ControlledDispatcher`, `TokioDeadlineScheduler`, and a
16-slot `TokioCleanupExecutor` with a five-second cleanup-job limit. Every
native command receives a distinct checked call ID and a finite public deadline
unless the caller requests zero. Active operations are tracked before leaving
session admission; close cancels them, waits for them and accepted cleanup, and
then waits for native I/O completion. Addon futures own Arc clones; native-class
finalization separately requests owner cancellation. One environment cleanup
hook owns a bounded weak registry of up to sixteen live/draining sessions and
pending attaches. It cancels pending attaches, waits for their admission tokens,
then cancels and joins all published sessions before napi tears down Tokio.
Each record also retains only a completion token, allowing the hook to wait for
a driver whose last session Arc was already finalized without retaining that
session. A finalized-but-draining session still consumes its admission slot.
Waiting here involves no JS callbacks. Cleanup errors or nonzero native teardown
counters emit a diagnostic that fails the differential runner.

The hook uses Node's documented reverse registration order; see
[Node-API cleanup and finalization](https://nodejs.org/api/n-api.html#cleanup-on-exit-of-the-current-nodejs-environment).
Object finalization alone cannot provide this guarantee when Promises retain
Rust-owned Arc references.

Implemented typed Rust paths are `Goto`, `FrameContent`, and value evaluation
for title. DOM content uses the explicit utility-world composition. The Node
facade forwards supported navigation options and wraps Rust response metadata;
it never constructs CDP traffic or implements navigation waits in JavaScript.
Response bodies/request identity and `response.finished()` remain unsupported.

The extended sixteen-case real-Chrome lane passed: original attachment gates,
byte-for-byte Page/Frame title and content, all four navigation waits and
response metadata, main-Frame reuse after document replacement, null responses
for same-document/about:blank, retained-wrapper rejection, a stalled-response
TimeoutError and close during an active navigation. Stalled-response endpoints
are dynamic test servers bound only to the explicit Tailscale IP; all static
documents use `serve-lib`. The extended lane now contains eighteen cases:
seventeen passing assertions and one deliberately failing, executable NODE-010
TODO. The TODO observes `page-world-intercepted-title` instead of the oracle's
original document title; it is not a passed compatibility cell.

The new Worker case destroys the only Node environment loading the addon while
its native `goto` awaits an HTTP response. It verifies that the dynamic server
observes request cancellation before fixture cleanup closes that server. The
case passed both alone and after the main environment had loaded the addon.
The environment hook also checked all recorded native counters at zero before
returning. This does not inspect all allocator memory or queued FFI values.

The shared Chrome also exposed a fixture-discovery race: an unrelated Page
navigated while the oracle was probing all page titles, failing setup before
the retained-wrapper assertion ran. Discovery now settles all probes and still
requires exactly one successful fixture marker. A failed actual-fixture read
or a stale duplicate remains fatal; no browser-operation assertion is retried
or normalized. Three deterministic helper tests cover those distinctions.
The package now has seven passing offline contract/helper tests; TypeScript,
strict release Clippy on the host/addon, and the 221-package safe-host normal
dependency check also pass. Fixture cleanup closes its exact owned Page even
if native close reports an error.

After that discovery fix, three successive real-browser runs of the current
native profile completed in 44.62 s, 61.61 s, and 46.75 s, each with seventeen
passing tests plus the same one known-discrepancy TODO. No unexpected failures,
native panic diagnostics, or environment-cleanup failures occurred in those
runs. The Worker-in-flight case passed again in a separate process with no
main-environment addon loaded, and the opt-in safe-Rust live close test passed.
Final CDP discovery contained no fixture-route pages or either recorded
preflight target. No unrelated browser targets were closed by this cleanup.

Three diagnostics changed the implementation rather than being hidden by test
normalization:

- This shared Chrome had more than 257 initial native events while target
  initialization awaited CDP replies. The original finite 256-item profile
  exhausted its ledger. The explicit startup budget is now 1024 events with
  256 KiB per-event and 4 MiB per-WebSocket-message limits; exhaustion still
  fails closed. These are admission ceilings, not allocated RSS or proof of
  lower process memory. Adversarial burst/steady-state pressure remains a gate.
- macOS kernel logs recorded `cs_invalid_page` and SIGKILL after in-place addon
  replacement. The release build now copies to a fresh staging inode and
  atomically renames it, preserving any currently mapped native library.
  Browser test lanes run serially; the runner streams evidence and bounds the
  entire lane to three minutes, with no nested Node test child left behind.
- Owner-requested close initially used the runtime's transport-lost path,
  which deliberately skips backend cancellation because the peer is gone.
  The stronger Worker fixture exposed that Chrome could keep loading even
  when all local counters were zero. Graceful host close now stops admission,
  abandons controlled operations while CDP is still live, drains their
  cancellation jobs (including `Page.stopLoading`), and only then closes the
  transport. Actual peer loss retains the no-peer-cleanup path.

VS2 remains incomplete because of NODE-009 and NODE-011, broader error/option
coverage, and the shared fixture adapter. Locators, public evaluation, handles,
artifacts, process/RSS attribution, narrow distribution and other full-goal
gates remain unfinished. No completed foundation work was pushed.

### DOM-read 1.1 checkpoint: NODE-010 resolved

Title is now `FrameTitle`, a fixed, portable command in `pwright.dom_content`
1.1, with distinct command/result wire tags and the same budgeted UTF-8 result
ownership as content. Its backend uses the existing Frame utility route and
never accepts public JavaScript source. `PageTitleExt` delegates to the main
Frame without changing the existing content-only extension trait bounds.
The Node host delegates to this typed command and no longer installs its former
title-only evaluation profile or declares those three direct evaluation
dependencies. The aggregate composition still links a broad dependency graph.

The pinned Playwright 1.58.2 prototype-interference case now passes and its TODO
was removed only after the paired assertion succeeded. A new same-Frame
concurrent title/content fixture passes too. The complete Node differential
lane currently has nineteen passing cases and no skips or TODOs.
Three successive complete runs passed (50.67 s / 8.28 s / 45.22 s). These
durations come from a shared browser/workstation and are not a performance
comparison against upstream.

The canonical 1.0 schema baseline remains untouched; generation and minor-
compatibility drift checks pass. Leaf/backend/client/codec tests verify fixed
source, exact utility context, main-Frame dispatch, UTF-8 budget ownership and
release, oversized result rejection, and distinct title/content result tags.
Negotiated server tests exercise both the preserved content command and the
new title command through generated handlers and the shared native route.
The Chromium composition suite passes 188 tests with 14 preexisting live tests
left explicitly opt-in. Workspace release check, the full
`CARGO_INCREMENTAL=0 cargo test --workspace --release --quiet` regression, and
strict Clippy for changed Rust components pass. The workspace test command
does not enable opt-in browser/Docker tests; real-browser evidence above comes
from the separately executed Node lane. Final CDP discovery confirmed that
this lane's preflight target and case-marked pages were gone; unrelated targets
were not closed. The independent DOM-read proof remains exactly fifteen
Pwright packages and excludes evaluation, navigation, aggregate composition,
provider, and runtime selection; the safe host's normal graph remains 221.

Isolated example lockfiles were synchronized for the earlier CDP task-tracking
feature: each of 26 files only adds `futures-util` to the existing `tokio-util`
dependency list, with no version changes. The schema reference generator also
now emits one final newline so newly added type sections pass whitespace checks.
Full compatibility, URL metadata, FFI result/admission budgets, Locators,
handles, events, shared-fixture integration and distribution/RSS gates remain
part of the active goal.

### Native URL metadata checkpoint

`Page.url()` delegates to its main Frame's native snapshot. The JS facade stores
no URL cache and sends no query to Chrome. The optional
`ChromiumFrameUrlObserver` shares the existing topology driver's cursor; initial
frame trees, `Page.frameNavigated` (including `urlFragment`) and
`Page.navigatedWithinDocument` feed it. Reads check the authoritative renderer
session, including a local Frame's nearest OOPIF ancestor, without requiring a
live execution world. Retired sessions and swap gaps cannot overwrite a current
snapshot. The port adds no extra CDP subscription or task and is absent by
default. Its observer owns and declares its retention budget.

Topology commits, active-metadata retirement and URL projection occur under the
same topology lock before the Node host can enumerate Page descriptors.
Observer failure is fatal to this host: it drains the attachment rather than
reporting that its metadata is fresh. The observer runs after topology commit;
it is not a transactional rollback participant for other embedders.

`goto` and topology use distinct bounded cursors over the native stream. An
eight-entry caller-owned fence queue makes command return wait for already
queued topology events, on success and on errors while still connected. The
driver's signal can interrupt only an idle receive, not partial target setup.
It then drains at most 4096 events using the existing snapshot budget. This is
an ordering fence, not an inferred URL assignment or a new CDP query. Shutdown
cancels the fence without waiting for a JS callback.

Each host has a separate production metadata ledger: at most 128 live or
retained Frame records, 16 KiB per UTF-8 URL, and 2 MiB aggregate URL bytes.
Replacement reserves the new value before releasing the old one, so transient
old/new overlap counts. Empty URLs need an entry but no byte lease. The active
table releases entries on Frame retirement or session teardown; existing
native Frame wrappers retain their last URL and leases until dropped. They
retain no transport, runtime or browser route. `pwrightMetadataMetrics()` reports
`entries`, `urlBytes` and `activeFrames` separately from `pwrightMetrics()`;
zero live-runtime counters do not mean user-retained metadata is gone. These
are per-host bounds, not an environment-wide bound over all historic sessions,
all V8 wrappers or allocator/RSS usage. NODE-011 remains open.

Three successive expanded live lanes passed all 26 cases with no skips or
TODOs (73.77 s / 138.47 s / 67.17 s); the latter two used the final rebuild
including renderer-session filtering and error-path return fences. These
shared-browser timings are not a performance comparison. New
paired cases cover an initial fragment, immediate URL reads after all four
`goto` wait policies, same-document/about:blank results, external document
navigation, history push/replace, hash changes, browser disconnect and external
Page close. Retained snapshots remain readable after close while live native
resources drain. Metadata unit tests separately check byte/entry admission,
UTF-8 rejection without truncation, old/new overlap and final-lease release.

Current release verification passes 37 topology tests, ten safe-host tests and
194 Chromium-composition tests, with fourteen preexisting opt-in composition
tests not enabled. Six new driver tests cover initial fragments, stable main-
Frame identity, idle wakeup versus bounded drain, retired-session rejection,
retirement and observer failure propagation. Workspace release check, strict
Clippy for all changed Rust components, formatting, seven offline Node tests
and TypeScript checks pass. The standalone topology proof stays at exactly
five Pwright packages and the safe-host normal graph remains 221 packages.

The Rust driver may advance metadata while the JS event loop is busy, whereas
upstream's client Frame updates its cached URL on its JS event delivery. This
architecture difference still needs a paired blocked-event-loop fixture and a
JS-visible publication boundary alongside NODE-007; the ordinary polling cases
above do not prove within-turn/event-order equivalence. Child Frame traversal,
general events, complete FFI budgets, broader API coverage, shared-fixture
integration, narrow distribution and RSS attribution remain active goal work.

### FFI admission and queued-result checkpoint

The current async surface (`connectOverCDP`, title/content, `goto`, close and
disconnect wait) now shares one explicitly constructed budget per native
environment, not a process-global budget or one independent budget per Browser.
`pwright-node-host::ffi_budget` contains the safe, foreign-runtime-independent
ownership mechanism. The addon only admits, bounds/converts arguments, schedules
owned futures and materializes JavaScript results.

| Lane | Pending settlements | Native payload allowance per call | Maximum inline future body |
| --- | ---: | ---: | ---: |
| Connect | 16 | 32 KiB | 1 MiB |
| Command: title/content | 32 shared command slots | 1 MiB + 4 KiB | 128 KiB |
| Command: goto | same 32 command slots | 256 KiB | 128 KiB |
| Close | 16 | 32 KiB | 16 KiB |
| Session signal (metadata/disconnect) | 16 | 32 KiB | 16 KiB |

The profile has 80 total slots and 64 MiB of reserved native payload/future
bytes. It admits before copying JS arguments or allocating/spawning an async
task. `JsString::utf8_len` rejects oversized strings before the owned UTF-8
copy: endpoint 8192 bytes, navigation URL/referrer 16 KiB, object IDs 20 bytes,
and wait policy 32 bytes. Browser version metadata is capped at 4096 bytes.
The future's actual inline size is additionally charged before boxing; its
per-lane ceiling prevents large command futures consuming control headroom.
The selected lane maxima and allowances leave byte capacity for close even
when command slots are full. Exhaustion is explicit, not a lossy queue policy.

A ticket keeps its pending-call and byte leases while Rust executes, while its
success **or error** waits for Node, and during JS conversion. Backend DOM data
keeps its existing ledger while the separate FFI allowance covers the owned
String copy; source/conversion overlap cannot release that copy's lease early.
These native reservations are not V8 heap accounting: returned JS strings,
wrappers and user-retained Promises have separate foreign-runtime lifetimes.
Fixed ticket/map/task metadata is count-bounded, not measured as payload bytes.

Environment shutdown cancels and joins Rust execution, then empties a bounded
registry of queued outcomes without invoking JavaScript. Undelivered callbacks
retain only empty ticket cells. Host shutdown follows while CDP is still alive,
so cancellation cleanup can finish before Node tears down its runtime. Cleanup
requires all FFI counters to be zero and reports violations to the fatal-test
diagnostic channel. `browser.pwrightFfiMetrics()` exposes environment-wide
`pending`, `queued`, `queuedCommands`, `running`, `reservedBytes`, `connects`, `commands`, `closes`
and `watches`; closing one Browser does not promise zero if another is active.

An initial real-Node run failed with V8 `IsOnCentralStack` at Promise creation:
the large embedded attach future was carried by value through generic N-API
layers on the JS callback stack. Boxing the charged future before those layers,
then passing a small boxed execution wrapper, removed the failure. No Node
stack-size override, unsafe converter or swallowed diagnostic was introduced.

The release host suite now has 20 passing tests. The ten FFI cases cover queued
values/errors, conversion failure, payload/future admission failure, independent
close slots, cancellation, and reclamation while foreign tickets remain alive.
Seven offline Node tests, TypeScript and the 221-package safe-host dependency
boundary pass. Four new browser stress cases verify 32 queued native outcomes
while JS is deliberately blocked, close under saturated command admission,
oversized UTF-8 rejection without navigation, and forced Worker teardown with
known queued outcomes. Each candidate fixture also checks all FFI counters
after its close/disconnect callbacks settle.

Two successive complete lanes with the final native byte/future profile passed
30/30 with zero skips or TODOs (117.64 s and 11.55 s). The second strengthens
the blocked-JS fixture to return more than 800,000 UTF-8 bytes, below the 1 MiB
DOM ceiling, rather than exercising only a tiny HTML result. Timing varies on
this shared automation browser and is not performance evidence. Workspace
release check, strict host/addon Clippy, formatting and diff checks also pass.

This is completion of the current async-export admission/result slice, not the
whole compatibility goal. The following checkpoint addresses NODE-009's JS-turn
publication issue. NODE-007 general events, new payload types, shared-fixture
integration, narrow packaging and separate Node/Chromium RSS measurements remain
open.

### JS-loop URL publication checkpoint

A paired fixture now performs external `history.replaceState` from an
independent Worker while the main Node loop is blocked with `Atomics.wait`.
Before the change, Playwright retained the old Page/Frame URL within that same
JS turn, but Pwright exposed the new URL. Both updated after yielding. This was
a demonstrated value/ordering discrepancy, not an inferred difference in RSS.
The strengthened candidate fixture checks that this exact Frame reports
`pwrightMetadataMetrics().unpublishedUrl`, plus host `pendingPublications > 0`,
before reading the old public URL. This proves Rust received the target update
without a fixed delay or relying on another Page's events in shared Chrome.

`FrameMetadata` now owns native and published URL views. Both are safe Rust
state; the JS facade does not cache, infer or assign URLs. New URL allocations
are shared by `Arc` between the native view and ordered publication record,
while the old published value remains leased. The existing 128 record / 16 KiB
per URL / 2 MiB aggregate limits cover all retained native, published and queued
URL values together. An additional 256 `EventItem` limit bounds unpublished
changes; reservation fails before metadata mutation, with no silent coalescing
or loss. Observer failure terminates the host attachment as before.

Every record has a monotonic sequence. A publication fence captures an exact
watermark in Rust. On a Node callback, the safe host applies only ordered
records through that watermark; a delayed older fence cannot regress an
already-published view. One environment-budgeted session-signal wait carries
metadata readiness or disconnect. The facade only rearms that wait through a
WeakRef; it contains no URL or navigation implementation. The existing 16 watch
slots cover this signal instead of adding an unbudgeted task or polling timer.

Attach publishes initialization before exposing the Browser wrapper. Async
command settlements capture a publication fence after native execution and
apply it before JS conversion, on both success and error; `goto` still uses its
native topology-return fence. Close and disconnect publish preceding metadata
before their public completion. Synchronous URL getters never drain the queue
or observe whatever the native thread most recently wrote.

Retired Frame records can remain in the publication queue without keeping a
browser route or transport alive. Environment ownership keeps retired session
records until their publication queue drains. On forced teardown it explicitly
discards pending changes and disables further enqueueing before native shutdown
finishes; no JS callback is needed. `pwrightMetadataMetrics()` now includes
`pendingPublications` in addition to entry/URL bytes and active Frames. General
event subscriptions, listener lifecycle and event-by-event public ordering are
still unfinished, separate from this metadata-publication mechanism.
The Frame-specific `pwrightMetadataMetrics().unpublishedUrl` reports whether
its native URL revision has not yet been published, even if a navigation back
produces the same URL text. It is a Pwright diagnostic extension,
not a Playwright method or a general event subscription.

Eight new safe-host tests cover fixed-watermark visibility, stale-callback
non-regression, item/byte admission, retired-Frame delivery, discard without
publication, updates during teardown and notification registration. The first
expanded live lane passes 31/31 with no skips/TODOs, including the formerly
failing blocked-JS oracle comparison and all existing command-return cases.
The next two lanes pass 32/32 (127.26 s / 129.21 s), adding forced Worker exit
with a known unpublished target-Frame update. The latter verifies target-local
revision state instead of relying on unrelated Page events. These shared-browser
durations are not performance comparisons.

`pwrightFfiMetrics().queuedCommands` counts only queued command settlements;
`queued` also includes session notifications and control results. Pressure tests
use the former so shared-browser URL notifications cannot masquerade as a
completed command or make an exact command-count assertion flaky. A mixed-lane
unit test checks this distinction and cleanup. The release host suite now
passes 29 tests; workspace release check, strict host/addon Clippy, seven offline
Node tests, TypeScript and the unchanged 221-package dependency boundary pass.
The final rebuilt addon also passes 32/32 with zero skips/TODOs (145.41 s),
including the target-local publication probes and separate queued-command
counter. No native panic or environment-cleanup failure was emitted.

### Explicit Page creation and ordinary close checkpoint

The Node host now explicitly installs the existing independent Page creation
and Page lifecycle profiles. `BrowserContext.newPage()` calls the typed Rust
`BrowserContextNewPage` service and returns a fully initialized Page/main Frame.
`Page.close()` and `{ runBeforeUnload: false }` call typed `PageClose`. Only a
target-closed failure is suppressed for ordinary close; capacity, invalid-ID
and other backend failures are not converted to success. Unsupported options
are rejected before FFI admission or browser mutation. There is still no Node
implementation of target creation, routing, closing, or cancellation.

The host's existing eight-entry topology fence queue now carries typed
creation, close and fence requests. A single caller-owned driver serializes
these explicit transactions with native topology updates, then reconciles the
bounded snapshot before replying. This prevents event ingestion and explicit
creation from publishing competing Page identities. The socket I/O task can
continue receiving bounded CDP events during the transaction. Lifecycle
services release their connection references before the I/O-cleanup barrier;
shutdown still drains the bounded existing continuation executor.

Creation reserves the new main Frame's metadata entry and `about:blank` URL
bytes before the native service can create a target. Failed or abandoned
preparation drops those leases. Success activates the same snapshot before
event reconciliation, without charging the URL twice, and returns that
snapshot in the descriptor. This avoids looking up a Frame after a racing
external close has already retired it. An activation failure attempts the
existing ordinary-close service as compensation and reports cleanup failure.
Retained closed wrappers continue consuming metadata admission by design.

Two differences were discovered in the live paired fixture:

1. The pinned Playwright 1.58.2 Chromium implementation attributes Pages from
   contexts unknown to that CDP connection to its default Context. Rust's
   general topology driver deliberately excludes them. The new explicit
   `with_playwright_context_fallback()` option matches the Node view while
   preserving strict exclusion as the reusable driver's default. This is Page
   attribution, not ownership or acquisition of the foreign Context. The
   fixture creates a marker in an independently owned Context and compares
   both connections' public view; it never treats that fallback as permission
   to dispose the original Context.
2. The default Context's internal empty key was sent as an empty native
   `browserContextId` to `Target.createTarget`; Chrome rejects that ID. The
   CDP adapter now omits the field for the default key, while preserving an
   explicit non-default ID exactly. Two serialization tests freeze both cases.

The new exports use the existing environment-wide FFI budget: creation uses a
command slot and close uses its independently admitted close lane, both with
the small-result allowance. Their native completion and rejection cross the
same fixed-watermark JS publication boundary as navigation/reads. No extra
event loop, browser process, driver subprocess or unbounded work queue is added.
The safe-host normal dependency closure remains 221 packages; this is still
an aggregate prototype, not a feature-minimal distribution.

Safe-host release tests now include metadata entry/byte pre-admission, unused
reservation release, exact snapshot activation and ordinary-close error
classification. Public close-event ordering and shutdown during an already
started creation remain explicit follow-up gates, not inferred from these
unit tests or ordinary success-path results.

The first rebuilt 36-case real-Chrome lane passes with zero skips/TODOs
(121.33 s). The expanded 37-case lane passes all new lifecycle cases, including
Page-close cancellation of its own stalled navigation while Browser stays
connected, but fails once at candidate attachment during another case's setup
(36 pass, 1 fail; 147.73 s). NODE-013 preserves the exact failure boundary.
A targeted sequence of Worker URL-publication teardown, close admission under
pressure and Page-close navigation cancellation then passes 3/3 (10.89 s).
No retry was added to the test runner and these results do not establish the
cause of the intermittent attachment timeout.

The release suites pass 67 CDP, 195 aggregate-composition and 35 safe-host tests
at the lifecycle checkpoint; 14 existing explicitly opt-in composition tests
remain disabled in this unit-only invocation. Strict release Clippy, workspace
release check, seven offline Node tests, TypeScript and the normal-dependency
boundary pass. The independent Page creation, Page creation composition, Page
lifecycle, Page lifecycle composition and topology dependency proofs retain
their exact closures of 8, 22, 6, 17 and 5 Pwright packages, respectively.
An additional host test verifies that connect-phase diagnostics preserve the
timeout error code; this changes diagnostics, not deadlines or retry behavior.

The final addon rebuilt with phase-aware attach diagnostics passes 37/37 with
zero skips/TODOs (110.64 s). The safe-host suite now passes 36 tests; strict
host/addon Clippy, workspace release check, offline tests, TypeScript, formatting,
diff checks and the unchanged 221-package dependency boundary pass again. There
was no native panic or cleanup failure. NODE-013 remains open because this
successful rerun did not reproduce or explain the earlier attachment timeout.
These shared-browser durations are not performance or RSS comparisons.

After all lanes ended, the exact Pwright-owned preflight Page was closed from
its task-local state directory. CDP discovery verified no remaining fixture
Pages, the unrelated XLQY Page was preserved, and the external dedicated Chrome
process remained healthy. The foundation commit is unchanged; no Node work was
committed or pushed during this checkpoint.

### Owned Context lifecycle and initialization checkpoint

`Browser.newContext()` / `newContext({})` and ordinary `BrowserContext.close()`
now forward through Node-API to the existing typed Rust Context creation and
lifecycle services. They share the serial eight-entry lifecycle/topology driver
with Page operations, rather than introducing a parallel topology writer. Both
capabilities are explicitly installed together; the reusable creation capability
still requires a close path. Non-empty Context options and close reasons fail
before native admission. Creation uses the FFI command lane; close uses the
independent close lane, with existing small-result/future allowances.

Owned Contexts are actual `Target.createBrowserContext` instances using
`disposeOnDetach: true`. Their ordinary close performs exact native disposal,
then child-first Page/Frame/runtime retirement before returning. The Node host
retains stable Context wrapper identity and invalidates its caches after native
completion. Repeated and concurrent close remain idempotent. Closing a borrowed
default Context is a separate host-level decision: Playwright's CDP behavior is
to disconnect the attachment. Rust selects that path without holding an active
operation token while awaiting its own shutdown drain. It never converts
default Context close into `Target.disposeBrowserContext` or `Browser.close`
against externally owned Chrome; the reusable owned-only Context service's
default/external rejection rule is unchanged.

Paired fixtures verify two Contexts and their Page identities, localStorage
isolation, exact sibling preservation, rejection of creation after close,
default-Context disconnect, descendant navigation cancellation, and browser
survival. Additional checks resolve the exact test-owned native target and
Context ID before browser disconnect or forced Worker termination, then require
both to disappear from Chrome's native inventories. They do not rely solely on
zero Rust counters to infer browser-side cleanup. Ready Context teardown passes;
abandonment during creation still needs separate evidence.

The first default-value comparison exposed inherited native viewport/screen
dimensions (1728x907 / 2560x1440) instead of Playwright's 1280x720. A strengthened
comparison then exposed outer-window dimensions (1728x994 instead of 1282x800
on this Mac). Both are now configured through an explicitly selected Rust
`ChromiumSessionInitializer` port. The driver awaits it before debugger resume
and Page-ready publication, for discovered and explicitly created session
routes. The port defaults to absent and owns no task, event subscription or
queue. Initializer failure stops publication rather than exposing a partly
configured Page; unit tests check before-resume ordering and failure gating.

The host's owned-Context initializer sends typed bounded settings for a
1280x720 desktop viewport/screen, scale 1, focus emulation and light /
no-preference / none media defaults. Native window bounds use browser headful
mode plus an injected driver-platform inset policy, matching the pinned
Playwright source; they are applied only to owned Page sessions. OOPIF sessions
receive media settings without resizing a window. Borrowed default or foreign
Context routes are left unchanged. These fixed requests use the existing
bounded CDP command path; no settings table, new task or JavaScript automation
implementation is added.

The canonical defaults fixture records values in its first document script and
is read using each implementation's own `Page.content()`. It opens no separate
Playwright observation connection that could overwrite focus/media before the
assertion. The final targeted snapshot passes, including viewport, screen,
scale, outer window, focus and media values. This does not close NODE-014's
download, headless-font, options or cross-platform gates.

The final rebuilt addon passes the complete 44-case lane with zero failures,
skips or TODOs (117.87 s). Release tests pass 39 safe-host and 197 aggregate
composition cases; 14 existing opt-in composition tests remain disabled in the
unit-only run. Strict composition/host/addon Clippy, workspace release check,
seven offline Node tests, TypeScript and the unchanged 221-package normal host
dependency boundary pass. Context creation, creation composition, lifecycle and
lifecycle composition proofs retain exact closures of 6, 22, 8 and 17 Pwright
packages. This is evidence for the Context checkpoint, not full compatibility,
an RSS comparison or resolution of the earlier NODE-013 attach timeout.

After the lane ended, the exact task-owned preflight Page was closed through
its Pwright state directory. Native discovery verified no remaining fixture
Pages and preserved the unrelated XLQY target; dedicated external Chrome stayed
ready. The foundation commit remains `f76e3f2`; this Node checkpoint was neither
committed nor pushed.

### Shared API corpus checkpoint (2026-09-06)

The shared API lane now imports the original `tests/specs/` functions through
one catalog used by both legacy API golden runners and the new Node-FFI runner.
It includes every non-CLI spec exactly once: 19 suites / 57 cases. Each case
runs serially on independent owned Contexts, first on the pinned official oracle
then through the public Node package. `goto`, `url`, `title` and `content` forward
to the existing Rust implementation; remaining `RunContext` operations reject
explicitly. No browser behavior is reimplemented in the candidate adapter.

The new lane never starts the legacy static HTTP server. It requires an explicit
`serve-lib` MagicDNS fixture prefix and verifies every HTML file's status, MIME
and SHA-256 before connecting. The upload fixture's hardcoded alias is mapped
only to a newly created owned file. Original specs, fixture HTML and stored
goldens are unchanged; the lane neither reads expected goldens nor overwrites
them. Existing corpus limitations (download placeholder, mouse-backed touch,
simple selector/filter substitutes and screenshot header checks) are documented
instead of being promoted to full API claims.

The adapters attempt all Page/Context/Browser cleanups, including partial setup
failure. Candidate teardown checks both safe-host and environment-wide FFI
counters at zero. The observer records an unsupported operation even if the
fixture catches its rejection; cleanup failure takes precedence over equal data.
Oracle failure is distinct from candidate failure, and neither is a match.
Original values use strict deep comparison before bounded diagnostic formatting;
no normalization is added to make differences disappear. A complete matrix is
required for exit 0, while the supervisor separately rejects native panics,
environment-cleanup diagnostics, timeouts, log overflow and process failures.

Evidence is stored in exclusive per-run directories: metadata/source/fixture
hashes, incremental case JSONL, a complete final report and supervisor status.
The runner records the installed release binary's hash independently of git
state; it does not infer that a dirty checkout proves binary/source equivalence.
Per-side observation limits are 256 operations and 64 KiB formatted values;
the corpus is capped at 256 cases. The external supervisor has a ten-minute
deadline and a 2 MiB combined log cap. Native read command budgets remain
distinct from the explicit 5-second connect/goto and oracle action timeouts.
These are test infrastructure limits, not process-memory measurements.

Three complete 57-case runs produce the same 6 matches / 49 unsupported /
2 oracle errors, with no value mismatch, candidate error, setup error, cleanup
error or native panic diagnostic. The two oracle errors also retain the
candidate's unsupported `goBack`; status precedence does not erase that evidence.
All 30 new runner unit tests, strict TypeScript checks, 67 legacy CLI unit tests,
seven package contract tests and the unchanged 221-package host boundary pass.
The earlier package-local 44-case lifecycle/resource suite remains separate.

The first shared run's two oracle errors were investigated without changing its
fixture or reference profile. An independent owned-page probe records a
`BackForwardCacheRestore` for the previous document. The preceding `load` belongs
to the document being left; no new `load` for the restored loader follows.
`page.goBack()` times out although title/URL identify the correct restored page
and `document.readyState` is `complete`. In the installed 1.58.2 source,
`crPage` forwards only `event.frame`, ignoring the navigation type;
`frameCommittedNewDocumentNavigation` clears lifecycle state. This code path
explains the observed missing-load wait. Playwright's own launch switches include
`--disable-back-forward-cache`; the read-only inspection of the dedicated Chrome
LaunchAgent shows no such switch. NODE-015 preserves this oracle/profile
boundary; the probe is not imported by the paired gate and is not a parity pass.

Reproduction commands, report classifications and remaining corpus limitations
are in [the shared comparison guide](../tests/README.md). The next implementation
step is to expose Rust-backed value evaluation, then locator/input capabilities,
using these unsupported cases as evidence targets. Adapter integration closes
the setup gap, not the overall Node compatibility or memory goals.

The final run (20:51:53–20:52:59 UTC, 66.66 s) confirms the same matrix with the
final explicit timeout metadata and supervisor checks. The task-owned preflight
target `348AA8A509C376B80C7D6584D39BD390` was then closed through its original
Pwright working directory; native inventory showed only the unrelated XLQY Page
remaining, and dedicated Chrome stayed ready. No additional commit or push was
performed. Local ignored evidence is under `.work/shared-node-ffi.DxJiZL/`:
`run-final/report.json`, `run-final/supervisor.json` and `history-final.json`.
NODE-016 records the unmeasured utility-world/renderer side of repeated attachment;
successful target cleanup is not an assertion that every Chrome allocation was
reclaimed or that this instrumentation is suitable for unbiased memory comparison.

### Node by-value evaluation checkpoint (2026-09-06)

Public `Page.evaluate` and main `Frame.evaluate` now dispatch the existing typed
`EvaluateValue` capability through the controlled Rust host. The shared adapter
forwards `RunContext.evaluate` to that public API; it contains no evaluator or
Playwright fallback. The JS facade handles overload shape and function source
stringification only. Node-API converts JS values on the Node thread into owned
`EvaluationValue` trees; only those trees and Rust session ownership cross native
awaits. Normal functions, method shorthand and async methods are parsed during
source conversion but never executed in Node. Browser execution and its bounded
serializer remain in the reusable Rust CDP evaluation backend.

The profile explicitly installs evaluation with 32 KiB source, 2 MiB generated
script/wire frames, 4 KiB error fields, depth 64, 4,096 nodes/entries, and separate
256 KiB aggregate string and binary limits. CDP command bytes increase from
64 KiB to 4 MiB to carry the bounded generated program; response/WebSocket limits
remain 4 MiB. Profile-derived backend reservations grow accordingly. The FFI
boundary reserves 8 MiB per evaluation plus the actual bounded future allocation
before input conversion. Input, intermediate trees, reference/key tables and
undelivered output remain covered until Node conversion or environment teardown.
The environment still caps all retained work at 64 MiB; close has its independent
lane. This is accounting admission, not an assertion about allocator or V8 RSS.

Conversion covers special numbers, undefined/null/booleans/strings/BigInt,
ordinary objects and arrays with cycles/shared references, Date/URL/RegExp/Error,
and all eleven standard typed-array kinds. Typed results copy into fresh
Node-owned ArrayBuffers. Ordinary Node Buffers follow the pinned oracle's plain
object argument behavior. Null characters are preserved: napi-rs 3.12.2's
`into_utf16()` view includes a terminator, so conversion uses the actual UTF-16
length, not that extra unit. Unpaired surrogates reject instead of replacement.
Shared memory is rejected before taking a Rust byte slice.

The first working paired run exposed two existing Rust evaluator discrepancies:
direct eval captured private `c`/`handles`, lost global `var`, and bound an arrow's
`this` to the wrapper; returned Error stacks omitted the official name/message
prefix. Both were reproduced and fixed in the Rust-owned runtime, using indirect
global evaluation and the bounded Error prefix. The original 16-case evaluation
lane then passed completely. This also corrects users of the same reusable
by-value runtime outside Node; it adds no capability to consumers that omit it.

The expanded 24-case lane passes 23 cases and fails the genuine TypedArray copy-
timing comparison recorded as NODE-017. It deliberately exits nonzero. Passing
cases additionally prove pre-execution depth/node/string/binary/source admission,
NUL/Unicode preservation, input-getter exception cleanup, bounded output failure,
URL publication on evaluation success/rejection, replacement main-world routing,
Page-close cancellation, close under blocked-JS binary result pressure, and
forced Worker destruction with three running or three queued evaluations. The
Worker tests verify removal of the exact owned native Context/Page and external
Chrome survival; they cannot query a destroyed worker's ledger or establish RSS.
Full error stacks and the remaining serializer contract stay open, not implied
by matching exception name/message prefixes.

Release tests pass: host 41, evaluation backend 9, handle-evaluation backend 17,
evaluation capability 6 and codec 5. Strict Clippy for host/addon and both affected
backends, package/shared TypeScript, seven offline Node tests, 30 shared-runner
tests and the 221-package safe-host boundary pass. The original 44-case real-
Chrome lifecycle/resource lane also passes after this evaluation installation.
Local ignored failed-baseline and expanded evidence is in
`.work/node-evaluation.pXFWNo/`. This is an additional experimental capability,
not completion of the Node compatibility or memory goal.

The full unchanged shared corpus was then rerun (21:25:57–21:27:22 UTC, 84.77 s):
12 matches, 43 unsupported, 2 oracle history errors, and no value mismatch,
candidate/setup/cleanup error or native panic. The six new matches are the five
evaluation cases and the historical filter case implemented by an evaluation
expression; this does not claim public Locator support. The supervisor exits 1
for the incomplete matrix, with no timeout/overflow/native-failure flag. Evidence:
`.work/node-evaluation.pXFWNo/shared-final/{report,supervisor}.json`.

The final 24-case rerun confirms 23 passes and the same NODE-017 failure, with
zero skips/TODOs; `evaluation-final.log` and `lifecycle.log` preserve both lanes.
Workspace release check, formatting and diff whitespace checks pass. The tested
native SHA-256 is `09574dd74a494692dff8df032bd005f4343be86024a752889a7ea8753502b94b`.
The exact preflight target `67F6C3AF17282BD9B258B77E251D384A` was closed through
its original Pwright directory. Native inventory then contained only unrelated
XLQY Pages, and the same dedicated Chrome browser remained ready. Original
shared specs, fixtures and goldens are unchanged. No new commit or push was made.

### NODE-017 implementation contract: deferred Node-thread binary conversion

The expanded timing baseline has 30 cases, with five genuine failures: immediate
and microtask mutation, a later argument getter mutating earlier bytes, detachment,
and shrinking a resizable backing buffer. A later `setImmediate` mutation and
growth beyond the initial view already agree. The pinned oracle throws TypeError
on detachment and serializes an out-of-bounds fixed view as an empty typed array.
Local evidence: `.work/node-binary-timing.ZIwG2D/timing-baseline.log`.

The binding now splits conversion into two Node-thread phases. Admission and the
ordinary value graph snapshot remain synchronous. Binary leaves capture fixed
byte-range views, not borrowed Rust slices, and the next injected Node
`setImmediate` callback materializes their bytes before scheduling any Rust
browser operation. This stage belongs only to the addon, never to the safe host
or portable protocol. Function execution/CDP remain in existing Rust services.

One environment-owned, non-Send registry owns the prepared trees, byte-view
references, promise settlements and existing FFI execution reservations. A single
native callback per environment carries only a weak registry plus checked call
IDs; no per-call finalizer owns unreclaimed argument trees. Environment cleanup
must clear this registry and delete Node references before waiting for FFI task
drain, because prepared executions have not yet entered the Rust executor.
Metrics and a blocked-before-dispatch Worker regression must prove this phase.

The 8 MiB FFI allowance must cover the prepared graph, reference metadata and
retained backing capacity as well as later input/output overlap. Count the entire
backing buffer's maximum capacity, not only a small subarray, with an explicit
1 MiB per-evaluation ceiling. Shared buffers remain unsupported until their
concurrent byte access has its own safe contract. Re-read native view metadata
on dispatch, handle detachment explicitly, and never cross an await with a raw
pointer or callback-scoped handle. Error and close paths must reclaim admission
even when no scheduled Node callback is delivered.

### Deferred-binary implementation and final regression record (2026-09-06)

`evaluation_value/input.rs` now prepares the synchronous graph and fixed views;
`evaluation_value/binary.rs` admits each full backing maximum and materializes
the bytes later. `node_reference.rs` is deliberately non-Send. The new
`deferred_conversion.rs` owns at most 32 prepared calls, constrained further by
the existing 8 MiB per-call / 64 MiB environment allowance. Materialization drops
all view references on the Node thread before constructing the Rust browser
future. Source evaluation, browser serialization, cancellation and CDP remain in
the existing Rust host/backend, with no new core protocol or browser task.

The environment retains only three constant scheduler/callback/intrinsic
references plus admitted binary views. The scheduler and native ArrayBuffer
`maxByteLength` getter are captured at package import, before later prototype
changes. This is not a defense against pre-import intrinsic replacement or a
whole-V8 memory limit. The backing charge conservatively counts aliases per
view, including potential RAB growth. Admission tests reject a tiny view over
an oversized backing, oversized growth capacity and excessive aggregate
backings before browser work. A later throwing getter drops earlier captures.

`browser.pwrightConversionMetrics()` reports prepared calls, binary references
and retained backing capacity independently from active Rust browser operations
and undelivered output. These calls already own FFI tickets: `ffi.running` can
include a prepared conversion and must not be interpreted as a browser operation
count. The prepared-pressure test observes nonzero reservations with zero host
active operations/queued commands. The single environment hook clears prepared
calls and deletes references before draining the FFI tracker, so their unstarted
task tokens cannot keep that barrier waiting on a disabled JS callback.

Verification preserves this sequence rather than selecting only passing runs:

- The eager-copy 30-case baseline failed five timing comparisons
  (`.work/node-binary-timing.ZIwG2D/timing-baseline.log`).
- The first deferred 31-case run passed 30; its prepared Worker failed to attach
  within 5 seconds during topology initialization, before state assertions
  (`deferred-first.log`). All reproduced timing differences matched.
- The expanded 38-case run passed all cases in 52.69 seconds, including forced
  Worker destruction with three prepared, three running or three queued calls.
  It also checked fixed byte ranges after growth/constructor replacement,
  backing admission, late-getter failure and close while input is prepared.
- After removing an unused field and validating the scheduler cast before
  transferring pending ownership, the final clean release build and strict
  Clippy passed. Its evaluation rerun reported **35 pass / 3 fail** in 59.80
  seconds. All three Worker cases failed at `worker-evaluation.mjs:6` with
  `Timeout 5000ms exceeded during topology initialization`; none reached its
  state assertions. Timing comparisons still passed. No skips/TODOs were added.
- The lifecycle rerun reported 17 passes and one Worker failure before the
  180-second supervisor terminated the incomplete lane with SIGKILL. Individual
  failure detail was not emitted before termination; do not infer its cause
  from the evaluation failure. Evidence: `lifecycle-final.log`. The historical
  44/44 pass is not current validation of this artifact.

Release host tests (41), strict host/addon Clippy, workspace release check,
package/shared TypeScript, seven offline package tests, 30 shared-runner tests,
formatting and whitespace checks pass. The host's normal dependency closure is
still 221 packages with no Node/driver/process-launch dependency. The final
native SHA-256 is
`95ffb8684f6797e44b82ecf5b968b5da3203e67bcb0e00c615c074bea798084c`.
The unchanged 57-case shared matrix was not rerun after these regression
failures; its prior 12/43/2 report belongs to the previous artifact. Shared
specs, static fixtures and golden snapshots remain untouched.

The exact preflight target `83097BDCA490D03B5861DD456C3298AE` and the interrupted
lifecycle fixture `F9039DAA32E3BA42026E02C8BABCBFC8` (exact fixture URL and
`__pwright_native_case__` title) were closed through Pwright. For the latter,
the task-owned Pwright state was pointed at the verified target for reattachment;
no unrelated state/profile was reused. Native inventory then showed only
unrelated XLQY Pages, and browser identity
`33b77c01-a5a4-42e4-b93b-ab822942d338` remained ready. Shared Chrome was not
restarted or reconfigured. No new commit or push was made.

Next gate is a bounded, attributable initialization-step diagnostic and a
reproducible NODE-013 case, then full evaluation/lifecycle/shared reruns.
Timeouts and unrelated-page activity are observations, not a proven cause.
NODE-016 still requires browser-world/RSS attribution; NODE-018 and the wider
API/distribution/compatibility goal remain open.

### NODE-013 attachment diagnostic checkpoint (2026-09-06)

An explicit, test-only Rust probe now subscribes to the existing command trace
events. It keeps at most 32 unanswered commands and the latest 128 completed
commands, each with bounded method/session identity and preparation/response
timestamps. The only production trace addition is the opaque session ID; no
collector, global subscriber, task or diagnostic retention is installed by the
Node package. Three offline tests verify pending capacity, completed-history
eviction and oversized-identity rejection. The collector records no response
body, script source, page content or console payload.

The Node diagnostic first attaches on its main thread alone, then keeps one
pinned upstream connection open while attaching from its main thread, a Worker,
and a separately compiled safe-host executable. Each attach retains the original
five-second deadline and performs one attempt. Rust cleanup has a ten-second
diagnostic deadline; auxiliary Node CDP/cleanup calls also have explicit
deadlines. Subprocess output is capped at 256 KiB. Process and target inventories
are capped at 128 and 256 entries respectively. This probe is separate from the
paired conformance gates and its failures remain nonzero.

Initial evidence, all under ignored `.work/node-attach-probe.qw5xeG/`:

- `host-single.log`: standalone safe-host attachment completes 35 commands in
  64 ms, then drains its host/transport.
- `runtime-matrix.log`: Node alone attaches in 45 ms. Holding one oracle
  connection causes main-thread, Worker and independent Rust host attempts to
  hit five seconds. The Rust trace has 29 prepared / 28 answered commands;
  the final unanswered method is `Runtime.enable`, prepared at 4,840 ms.
- `runtime-resources.log`: another instrumented run reproduces the same three
  failures, with `Page.getFrameTree` unanswered at the deadline instead. The
  preceding trace contains multiple slow commands, not one invariant stall.
  During one five-second window Node uses about 77 ms of process CPU, its event
  loop is about 97% idle, and maximum observed loop delay is about 26 ms.
  These CPU samples do not measure GPU wait or prove a renderer scheduling cause.

Source inspection found the Rust WebSocket left Nagle enabled while the pinned
upstream `ws` implementation calls `setNoDelay()`. A temporary production-profile
opt-in tested this hypothesis without changing legacy callers. Its first passing
run coincided with a changed Page count, so it was not accepted as a fix.
Two saved release executables then ran under one continuously open oracle in
off/on/on/off order. Target-ID/type inventories were unchanged before/after all
four attempts (`tcp-abba.log`):

| TCP_NODELAY | Trial durations |
| --- | --- |
| Off | 2.046 s, 2.132 s |
| On | 2.200 s, 2.116 s |

This did not demonstrate consistent improvement. The experimental socket option
and its Node selection were removed; production transport behavior is unchanged.
The ABBA helper accepts separately built artifacts and records their SHA-256;
it is not an exported runtime option or a normal compatibility gate. The saved
off/on artifacts identify this experiment only. Unchanged target IDs also do
not prove unchanged page content, CPU load or browser scheduling.

The final restored release diagnostic (`final-probe.log`) passes with four Pages:
Node alone 727 ms, main-thread with oracle 3,163 ms, Worker 3,172 ms, and independent
Rust host 3,436 ms. Its completed trace now attributes each command to a session.
Browser activity and timings changed during this shared-browser investigation;
do not call this a resolved NODE-013 or replace failed regression evidence with
the passing probe. The full 38/44/57-case lanes were not rerun in this diagnostic
checkpoint.

Verification passes: 41 safe-host unit tests, 67 CDP unit tests, three probe-bound
tests, strict Clippy for host/addon/CDP, workspace release check, seven package
tests, Node/shared types, script syntax checks, formatting and whitespace checks.
The normal safe-host dependency closure remains 221 packages; `tracing` is added
only as a host test dependency. The restored addon SHA-256 is
`dc21781c241e74ac5167110eb7063e81cd5dcfa6e6dfaeea51ad2a8e12b81c30`.

Next work is a controlled session-local latency fixture and review of bounded
initialization batching. Playwright initializes many commands concurrently;
our current driver waits serially. Any change must preserve Context defaults
before resume, utility-world/readiness fences, target identity, cancellation and
the production resource plan. Separately, fixture discovery should avoid title
probes on unrelated Pages using known ownership evidence, while still failing
on unreadable or duplicate actual fixtures. Neither design is implemented or
claimed verified in this checkpoint. No new commit or push was performed.

After verification, only the exact task-owned preflight target
`17EDF8A9119B87ADB1CF7DF33D4B04E9` was closed through its original Pwright state.
Inventory showed only unrelated XLQY Pages afterward; dedicated Chrome kept
browser identity `33b77c01-a5a4-42e4-b93b-ab822942d338`. The expired Node fixture
registration was renewed for the same path/route/Tailscale bind and verified
with HTTP 200. No unrelated registration or browser profile was changed.

### NODE-013 paired-prerequisite checkpoint (2026-09-06)

Following the diagnostic checkpoint, fixture selection now first filters by the
creator's exact current URL, passed explicitly through Worker data, then requires
one readable title marker. Pages at other URLs receive no title probes. Wrong/missing
URLs, duplicate markers and unreadable fixtures still fail. Package offline
tests increase from seven to nine; no shared specs, fixtures or goldens change.

Rust topology initialization gains explicit production-profile selection of
paired `Page.getFrameTree` / `Runtime.enable` prerequisites. Node selects it;
other consumers remain serial. Page enable and Context defaults remain before
the pair, and utility worlds, recursive attachment, debugger resume and atomic
PageReady remain after successful completion of both. Pinned upstream
`crPage.js::_initialize` independently schedules these commands too; this is not
adoption of its unawaited utility-world calls. The pair uses one caller-polled
join, no task, timer, retry, hidden driver or concurrent session loop.

Selection requires two configured pending CDP slots and adds exactly
`4 * max_response_bytes` to the event driver's fixed backend reservation. In
the Node profile that is another 16 MiB declared ceiling, not preallocation or
an RSS measurement. Both calls remain within existing connection admission.
Failure or caller cancellation drops sibling pending registrations; queued wire
buffers retain their own writer leases. Ten deterministic tests cover both
polls, both pending orders, both failures, owner cancellation, successful result,
serial default, two-slot rejection, exact accounting and idempotent selection.

Verification completed in `.work/node-paired-init.nHmvUx/` using native SHA-256
`384f05939a4fce614c649e3cf2212daebf294a8ac0de3c4d5d104ea2d3c25a4c`:

- Composition release tests: 207 passed, 14 preexisting opt-in browser tests not
  enabled. Host release tests: 41 passed. The ten new prerequisite tests are
  included in the composition count.
- Strict release Clippy for composition/host/addon (all targets), workspace
  release check, formatting and diff checks passed. Node package tests: 9/9;
  shared runner unit tests: 30/30. Both TypeScript checks passed. Normal safe-host
  dependency closure remains 221 packages; no driver/process launcher in JS.
- Attachment probe: main alone 737 ms; main with oracle 1737 ms; Worker 1853 ms;
  independent Rust host with oracle 1719 ms, 28/28 replies, no pending commands
  or recording overflow. In one session the Frame Tree and Runtime requests
  both began at 97 ms and replied at 352/353 ms, proving real overlap. Three
  Pages were reported. Browser activity differs from previous probes, so these
  timings are not a controlled performance attribution or root-cause closure.
- Lifecycle: **44/44**, 133.89 seconds. Evaluation: **38/38**, 44.83 seconds,
  including prepared/running/queued Worker destruction. Both complete with no
  skips, cancellations, background native panic or cleanup diagnostic.
- Unchanged shared corpus: **57 cases / 19 suites**, **12 matches / 43
  unsupported / 2 oracle history-navigation errors**, no value mismatch,
  candidate, setup or cleanup error. Its supervisor exits 1 as required, without
  a signal, timeout, log overflow or native failure. Metadata identifies the same
  addon hash; no fixtures, goldens or observations were rewritten.

The five-second attach, 180-second package-lane and 600-second shared-lane
deadlines are unchanged. Earlier failed/incomplete runs remain evidence;
NODE-013 is not declared resolved, and full API/serialization/RSS claims remain
open. Next implement the public Locator path over typed Rust capabilities and
expand its paired coverage, keeping these regression lanes mandatory.

Chrome was reused without a restart. The Node fixture and renewed shared
registration were verified with HTTP 200. Only task-owned preflight target
`FD11C75F77F34979E95FCB19BE0A2469` was closed using its original Pwright state;
unrelated XLQY Pages were preserved. No new commit or push was performed;
foundation HEAD remains `f76e3f2701a8ccde0474b5accf40dc328aaa511c`.

### VS3 Locator binding contract (in progress)

Locator values now own bounded, immutable Rust selector recipes, not DOM
handles, CDP sessions or cached matches. Creation and composition perform no
browser work; each operation resolves again through the existing typed Locator
capability. JavaScript supplies API-shaped arguments and wraps native values.
Query evaluation, strictness, retries, actionability and input remain Rust code.

The Node environment explicitly owns a separate recipe budget: at most
1,024 retained recipes/copies and 8 MiB of declared selector storage. Each recipe
retains at most 64 KiB of strings, 256 nodes and depth 32, matching the existing
selector compiler. Admission precedes copying selector strings or composing
trees; failed construction releases its reservation. Command-owned selector
copies need the same explicit admission and release after the operation.
This budget is independent of asynchronous FFI settlement admission. Closing
a browser does not dispose a pure Locator value; it must not keep browser I/O
alive. Node finalization releases retained recipes, while Worker shutdown first
drains native operations without waiting for a JS callback.

Native bound Locators carry a weak host identity and portable Frame ID.
Cross-Frame or cross-connection combinations fail in Rust before query execution.
The facade must preserve lazy selector errors: for example, upstream accepts
`page.locator('')` as a value and rejects its later operation. A deferred typed
query issue is preferable to substituting a fake valid selector. CSS engine
strings and Playwright's extended selector language need explicit Rust parsing;
ordinary CSS support is not evidence of complete selector-language support.

The implementation sequence is query ownership/composition, bound typed reads,
Node wrappers and paired strictness/re-resolution/error/lifecycle tests, then
actions using the same value and admission path. VS3 is not complete until its
action/input and discrepancy gates are also satisfied. Existing lifecycle,
evaluation and the full unchanged shared corpus remain mandatory regressions.

The public value path includes `Page.locator`, main `Frame.locator`,
`locator.page`, `first`, `last`, `nth`, `and`, `or`, nested `locator` and
`filter({hasText, hasNotText, has, hasNot, visible})`. Text filters currently
accept strings only. `filter()` returns a distinct value, and empty string
text filters preserve the upstream no-op behavior. Cross-Frame and even
same-native-Page/cross-connection combinations reject in Rust.

The fourteen read operations are count, allTextContents, allInnerTexts,
textContent, innerText, innerHTML, inputValue, getAttribute, isVisible,
isHidden, isEnabled, isDisabled, isEditable and isChecked. Scalar reads use
strict fresh resolution; collections/count do not. Visibility is immediate,
and an absent attribute returns null rather than an empty string. Ordinary
CSS pierces open shadow roots through the existing Rust selector runtime.
Declared action methods explicitly reject; no action support is claimed.

The host installs the existing Locator 1.x backend with eight operation slots,
1 MiB codec strings and 2 MiB command / 6 MiB result frame limits. A read admits
a command-owned selector clone and conservatively reserves 6 MiB of FFI
payload allowance through Node conversion. No JS handle or borrowed pointer
crosses a Rust await. Recipe count/byte metrics are independent of browser
teardown metrics because pure values may remain held by user code.

The first 15-case real-Chrome lane passed, but expansion found a real close
race: `Session with given id not found.` was classified as generic Backend by
the Locator leaf. It now reuses `pwright-backend-cdp-common`'s existing native
error classifier, as DOM reads and evaluation already do. Two public-service
tests cover closure and the negative navigation-context-loss case. The helper
is command-free; exact standalone/Locator-composition proof closures increase
from 9/17 to 10/18 packages, without importing another capability. All 30
umbrella profiles still pass and the safe Node host closure remains 221.

The first expanded lane timed out because a close test awaited `title()`
behind its own pending selector operation, leaving two query reservations
alive long enough to fail the following GC check. The close test now observes
native admission without another browser command. NODE-020 separately retains
the actual concurrency mismatch as a mandatory comparison; it is not erased
by changing that synchronization fence. Subsequent 17/18 runs captured the
closed-session error above before the Rust classification fix.

Shared adapter reads use the public Locator API. The original oracle adapter
uses legacy Page selector reads, whose default is first-match, so the candidate
adapter composes `.first()` before scalar/state reads. Count stays non-strict.
This composition is a Rust query, not a JavaScript evaluation substitute.
No existing shared spec, fixture or golden was rewritten. In particular, the
historical filter test still exercises its original evaluation expression;
new package tests are the evidence for actual Locator composition.

### NODE-020 bounded concurrency contract

Default per-target FIFO serialization is preserved. The explicit immutable
`ControlledRuntimeConfig` selects a nonzero per-target permit count;
the Node reference profile selects eight. Keep target identity unchanged:
different methods must not fabricate targets or use separate connections to
evade admission. Each target has a bounded active set and FIFO waiting queue;
dropping a queued or granted future releases exactly its call and promotes the
oldest waiter. No new executor, worker, timer or detached cleanup is added.

Connection execution remains capped at eight and runtime pending calls at
sixteen. Target permits may wait for connection admission but consume no
ActiveCommand lease until that admission grants. Existing per-call leases,
absolute operation lifetime, deadline/cancellation races, subtree retirement,
and bounded abandoned-cleanup ownership remain authoritative. More per-target
permits do not increase the connection-wide payload or command budgets.

The Node DOM-read backend also explicitly selects up to eight reads per Frame,
within its existing connection-wide eight-read/byte budget; the leaf's default
stays one. Every read reserves independently and releases both counters on
failure or drop. Other capabilities retain their own declared operation bounds.
Input transactions remain Page-owned and serialized by the input coordinator,
not by preventing unrelated reads while waiting for an element.

Scheduler tests cover FIFO/capacity/drop/identity; controlled dispatcher tests
cover connection admission, deadlines, subtree close, resource drain and
independent abandoned-cleanup jobs. DOM tests cover per-Frame/global bounds and
default compatibility. The unchanged NODE-020 comparison passes alongside
same-Frame Locator/evaluation overlap and close with ten pending reads, including
two beyond the connection's active limit. All four complete browser lanes
remain required. This contract does not claim arbitrary navigation/input
ordering parity before those tests exist.

### Locator-stage verification checkpoint (2026-09-06)

The rebuilt release addon SHA-256 is
`748c976fd940d213f8ef7d105a59c3668855315fd05385a8a926f99175e07838`.
Evidence is retained under `.work/node-locator.51x1V1/` at repo root. The same
dedicated Chrome 152.0.7977.76 browser identity was used without restart,
profile changes, unrelated-Page mutation or deadline increases.

- `rust-unit.log`: 98 Locator-backend and 54 safe-host release tests pass.
- `clippy-final.log` / `workspace-check.log`: strict all-targets Clippy for
  Locator/host/addon and the release workspace check pass.
- Locator backend/composition proofs pass at 10/18 Pwright packages, including
  their consumers; all 30 umbrella profiles pass. Node normal closure is 221.
- Nine package tests, 30 shared-runner unit tests, both TypeScript checks,
  formatting and whitespace checks pass.
- `locator-final.log`: 18/19, 29.87 seconds. Page-close error, cross-connection
  identity, separate count/byte pressure, finalization and Worker cleanup pass.
  NODE-020 remains a normal failing concurrency comparison; no cancelled or
  skipped tests and no native-panic/cleanup diagnostic.
- `lifecycle-final.log`: 44/44, 22.79 seconds.
- `evaluation-final.log`: 38/38, 35.64 seconds.
- `shared/report.json`: complete 19-suite / 57-case matrix, 22 matches,
  33 unsupported and 2 oracle history errors, with zero value mismatches or
  candidate/setup/cleanup errors. `shared/supervisor.json` records exit 1,
  no signal/timeout/overflow/native failure. Start/end: 23:55:31–23:56:57 UTC.
  This remains a failing full-compatibility gate, not 57 passes.

Earlier evidence remains alongside the final artifact. The first 15-case
Locator pass and lifecycle 42/44 ran addon
`a4126bf1966edc0d4b5dcb579ca45926cb0732964c351d6adbffedcc88d36a1f`.
That lifecycle failure included the preflight-path assumption, now corrected
using an exact owned Page, and NODE-014's initial outer-window discrepancy,
which remains unresolved. `locator-expanded.log` records the test-fence timeout
and collateral query-retention failure; `locator-cancellation.log` and
`locator-error-evidence.log` record 17/18 before the closed-session classifier
fix. No failed log was overwritten or reclassified as a pass.

After these runs, only the exact preflight target
`95542738B3E01D60350E5F82991F57A2` was closed through its original Pwright
working directory, and CDP discovery confirmed it absent. Other Pages and the
external Chrome process were preserved. Foundation HEAD remains `f76e3f2`;
no new commit or push was made. The goal remains active: next review the
NODE-020 opt-in command-concurrency contract and implement remaining Rust
selector/action paths, with all four live gates retained.

### Bounded target-concurrency verification checkpoint (2026-09-07 UTC)

The release addon SHA-256 is
`9b2415d7e5b4a9541ad9e74290a4bc66e0fa05a39a47a1b6f6be01da49e77452`.
Evidence is retained under `.work/node-concurrency.TLMFjf/` at repo root.
Chrome 152.0.7977.76, Node 26.7.0 and pinned Playwright 1.58.2 use the same
dedicated browser identity, without a Chrome reset. The static package route
was registered as `/pwright-node-concurrency` through serve-lib on the existing
Tailscale listener; the shared corpus reused its verified `/pwright-shared`
registration. Browser lanes ran sequentially, against independently owned
oracle/candidate Contexts.

- `locator.log`: 22/22, 33.92 seconds. The original NODE-020 completion-order
  comparison now passes. Added cases cover evaluate creating a waited-for
  element, seven independent same-Frame reads, and Page close with ten admitted
  reads (eight active-command slots plus queued work), all settling TargetClosed
  and returning operation/FFI admission to zero.
- `lifecycle.log`: 44/44, 95.17 seconds.
- `evaluation.log`: 38/38, 48.80 seconds.
- `shared/report.json`: complete 57-case / 19-suite matrix, from
  00:19:57 to 00:21:33 UTC; 22 matches, 33 unsupported, 2 oracle history errors,
  and zero value/candidate/setup/cleanup errors. `shared/supervisor.json`
  records status 1, no signal, no timeout, no overflow and no native failure.
  The complete shared gate is still non-green.
- `rust-unit-final.log`: runtime 104, DOM-content backend 13, safe Node host
  54 and Chromium composition 208 pass; 14 pre-existing explicit real-browser
  composition tests remain ignored in that unit-only invocation.
- `clippy-final.log`: release all-target strict Clippy passes for runtime,
  DOM backend, host, addon and Chromium composition; no lint suppression.
  `workspace.log`: release workspace check passes.
- Package offline tests 9, shared runner unit tests 30, both TypeScript checks,
  and the Node boundary check pass. The safe host closure remains 221 normal
  dependency packages. The narrow DOM proof remains 15 Pwright packages and
  all 30 umbrella dependency profiles pass; concurrency adds no capability
  dependency. Formatting and whitespace checks pass.

The first shared invocation supplied a relative output directory and failed
before spawning a runner. Its error remains in `shared.log`; the corrected
absolute, newly owned output directory was used for the actual run recorded
in `shared-live.log`. No conformance outcome was silently retried.
An earlier new runtime test incorrectly expected backend cleanup after a
transport had already closed; the existing contract correctly skips native
I/O then. Its failure remains in `.work/node-concurrency-unit.log`; the corrected
subtree-close test instead proves both started operations cancel, a queued
operation never starts, and all admission drains. A separate concurrent
future-drop test proves both independent cleanup jobs survive until polled.

The original Locator 18/19 failure and earlier NODE-013/NODE-014 evidence remain
historical records, not deleted or converted into passes. Longer or shorter
lane durations are not a performance attribution or RSS measurement.

After validation, only this task's preflight target
`960DF3CE1D7A6325A6BEF3837316A2AE` was closed using its original Pwright working
directory; CDP discovery confirmed it absent. The external browser was not
restarted or terminated. HEAD remains `f76e3f2`; no new commit or push was made.
The goal remains active. Next extend Rust-backed Locator selector/action
coverage, then expose it through thin Node-API bindings and run all four gates.
Node distribution profiles, handles/events/artifacts, full error/value/ordering
parity and process-memory comparisons remain open.

### Rust-backed Node Locator actions

The Node API now exposes `click`, `dblclick`, `fill`, `clear`, `press`,
`check`, `uncheck`, `setChecked`, `selectOption`, `hover`, `focus`, `blur`
and `scrollIntoViewIfNeeded`. Query ownership is unchanged. Each call derives a
separately admitted command selector and invokes an existing typed Rust service;
the addon only converts Node values, and the facade only validates public shapes.
Click uses the existing 2.x optional-field contract in process, without encoding
a wire request. Shared canonical action parameter types serve the other calls.
No protocol schema or default Rust dependency profile changes.

Double-click is one Rust Click request with count two and navigation waiting
disabled, matching the pinned oracle's double-click path. Clear is one Fill
request with an empty value. Check/uncheck are one SetChecked request each.
String selection uses value-or-label; descriptor fields are conjunctive.
Node selection arrays must be homogeneous strings or descriptors, and null/empty
arrays deselect. ElementHandle selection is still unsupported. Deprecated
`noWaitAfter` is accepted as ineffective on operations where Playwright ignores
it; Click and Press retain their existing Rust navigation barriers.

Actions reserve 6 MiB of the existing environment-wide 64 MiB FFI allowance
before owned string/vector conversion. Fill is bounded at 1 MiB UTF-8; Press at
128 bytes; selection at 1,024 descriptors, 64 KiB per string and 1 MiB aggregate.
Each conversion validates UTF-16 and rejects unpaired surrogates rather than
silently replacing input. Safe-host validation independently checks retained
capacities. Native pointers and callback-scoped Node objects never cross an await.
Backend CDP/script ceilings remain independently authoritative; these are
resource bounds, not a promise that every possible maximum-size input fits all
downstream encoding budgets or a claim about V8 heap/RSS.

Inspection exposed an existing Fill gap: focus/selection and insertion bypassed
the Page input coordinator, and empty Fill sent Delete without tracked key
cleanup. Fill now takes that shared gate for each attempt, releases it before
retry delay/runtime reinstall, and keeps focus/selection plus insertion atomic
with respect to other coordinated input. Clear records acknowledged Delete
down/up using the browser's platform marker and repairs a failed key-up before
return. Both Locator-major cancellation hooks drain the shared input state.
Two new backend tests prove no focus/insert under an already-held Page gate and
exact Delete repair after key-up failure. The command adds no retained queue,
DOM handle, task, or dependency. Focus/Blur and arbitrary external script
mutations do not gain a global DOM transaction guarantee.

The independent `test:actions` lane is now required alongside lifecycle,
evaluation, Locator reads and the shared corpus. It compares native events,
modifiers, forced/trial input, strictness, all fill target kinds, selection
descriptors/events, focus, hover, scrolling, navigation and Page close. Worker
coverage terminates only after observing an acknowledged keydown, then requires
the exact owned native Context/Page to disappear. A normal deadline comparison
retains NODE-022 as a discrepancy, not a passing cleanup-equivalence claim.

The shared adapter now routes click, double-click, fill, hover, focus, blur,
check and uncheck through the public Node Locator API. Its `.first()` preserves
the original legacy Page-selector first-match behavior. Shared `press` is a
Page Keyboard operation, not Locator.press, and remains unsupported alongside
sequential typing, mouse coordinates, selectors, downloads and other missing
APIs. No shared fixture, spec or golden was rewritten.

### Action-stage regression and terminal completion evidence (2026-09-07)

Evidence is retained under `.work/node-actions.5JbLoA/`. The initial action
artifact `7bff4a4f...48b6` passed 17/21; an expanded `27a8cdc7...75fe` passed
22/24. The homogeneous selection rejection then changed from TypeError to the
oracle's plain Error in the addon. These earlier failures remain recorded.

The action artifact
`eae32ce938217fa3246bdff5d2b0eef2b3840271c285b958fc27030aa841af9e`
produced these full-lane outcomes before the NODE-023 fix:

- Shared: 36 matches / 19 unsupported / 2 oracle errors, all 57 cases complete;
  supervisor exit 1, no timeout, signal, output overflow or native failure.
- Actions: 22/24 in 30.94 s, retaining NODE-021 and NODE-022.
- Evaluation: 36/38 in 45.40 s; both errors are pinned Playwright's 5-second
  `connectOverCDP` setup timeout before the candidate observation.
- Locator reads: 21/22 in 36.44 s; NODE-023 is an actual candidate error.
- Lifecycle: 36 reported passes and one reported failure before the unchanged
  180-second supervisor killed the child; seven cases did not report completion.
  The failing Worker test took 62.46 s, but the former spec reporter deferred its
  error details until exit, so this log cannot attribute it to teardown itself.

The supervisor now explicitly selects TAP to emit each test's failure details
immediately, including when a later deadline kills the process. Its limits and
failure policy are unchanged. A tiny local failing-test sentinel verified this
reporting behavior. Locator close diagnostics retain original native messages
without changing the expected public result.

The killed lifecycle process left target `9F64EE7A7A1C2AB58612F96153D69C16`
at this task's exact fixture URL/title. The first filtered Worker diagnostic
correctly failed on duplicate fixture markers. That exact stale target was
closed through a separate Pwright state directory; the second filtered run
passed 1/1 in 3.32 s. This is not a repaired full lifecycle gate or evidence
that the original failure was caused by the later duplicate.

The first filtered Locator-close diagnostic passed. Four explicit probes then
passed/passed/failed/passed; probe 3 retains the exact eight-entry fence queue
error. The shared completion policy now preserves authoritative TargetClosed
without attempting a redundant fence, across Locator reads/actions, navigation
and evaluation. Successful commands and nonterminal errors still publish;
live fence errors still propagate. This is not retry, queue growth, suppression
of ResourceExhausted, or a solution for missing execution worlds.

The replacement addon is
`e50764079d4aafcb3ba898257c4726b385db2a73194e735028896c3684265d48`.
Safe-host release tests pass 62/62, including five new completion tests. Strict
host/addon Clippy passes. The first unit attempt referenced an unavailable test
helper crate; its retained compile failure was fixed using standard Future
polling and existing Tokio channels, without adding a dependency.

Full package lanes on this replacement artifact complete without skips or
cancellations: Locator **22/22** (22.69 s), evaluation **38/38** (24.53 s),
lifecycle **44/44** (68.25 s), and actions **22/24** (20.02 s). Actions still
fail the ordinary NODE-021 and NODE-022 assertions. Passing regressions after
the terminal fix do not establish an attach-latency fix or explain the earlier
Worker failure. No native panic or environment-cleanup diagnostic was reported.

Static checks pass: backend Locator **100** release tests, host **62**, package
offline **9**, shared runner unit **30**, both TypeScript checks, strict release
host/addon Clippy, workspace release check, formatting and whitespace checks.
The safe-host normal dependency closure is still **221**; narrow Locator and
composition proofs remain **10** and **18** packages and the umbrella proof
passes all **30** profiles. Shared specs, fixtures and goldens remain unchanged.

The first shared invocation on this artifact exited 2 before running cases:
the `/pwright-shared` serve-lib registration had expired and fixture byte
verification received HTTP 404. `shared-completion/` retains that infrastructure
failure. The same exact fixture path/route/bind was renewed for two hours and
verified with `serve-lib list` plus HTTP 200 before starting a separately owned
`shared-completion-live/` report. No observations were overwritten or retried
inside a comparison case.

That final shared report completed at `2026-09-07T01:08:14.978Z`, after 101.48 s:
**31 matches / 19 unsupported / 2 oracle errors / 5 setup errors**, with zero
value mismatch, candidate-operation error or cleanup error. Four setup cases
time out in both pinned Playwright and Rust attachment; one times out only in
Rust topology initialization. Its supervisor exits 1 with no signal, timeout,
overflow or native failure. The earlier artifact's 36/19/2 result is preserved,
not substituted for these five unobserved current comparisons. NODE-013 remains
open despite the completed package regression passes.

At cleanup, the original preflight target `6472FA591FD647EFFF18A8DEC24C2449`
was already absent: Pwright reported no such target when reattaching its original
state, and CDP discovery confirmed neither it nor the cleaned stale fixture
remained. Only an unrelated XLQY Page, browser UI and extension worker were
visible. The browser identity stayed `33b77c01-a5a4-42e4-b93b-ab822942d338`;
we did not restart Chrome or close unrelated Pages. Changing target activity
is recorded, not assigned as the timeout cause. HEAD remains `f76e3f2`, with no
new commit or push. The Node compatibility goal remains active.

Next design task: NODE-021 execution-world readiness must be a caller-polled,
explicitly selected Rust precondition inside the controlled operation, after
pending-call admission and under its original deadline. Do not put an unbounded
host waiter before runtime admission. Readiness must distinguish an installed
capability waiting for a new document's world from a genuinely absent capability,
and use a world-generation check plus lossless notification registration to
avoid missed wakeups. Page/Context retirement, transport loss and Worker teardown
must cancel that wait without input replay. The existing topology driver remains
the only event owner. Required proofs include commit-before-world, consecutive
document replacements, no timeout reset, close during readiness, full admission,
and preserved dependency/resource budgets. This precondition is not implemented
by the terminal-completion fix.

### NODE-021 bounded execution-world readiness

The separately implemented precondition now addresses that next design task.
`ChromiumExecutionWorldReadinessProfile` is explicitly selected by the Node
host; default Rust/v1 consumers retain immediate route lookup, including the
original snapshot taken when `TargetedService::call_for` constructs its future.
The portable runtime, DOM/evaluation/Locator leaves and JavaScript facade do
not acquire a new navigation policy.

For the selected composition, DOM title/content and ordinary utility-world
Locator routes wait for the current utility context. By-value evaluation waits
for an explicit current main-world route, including on the main Frame instead
of falling back to an implicit default context during a gap. A capability that
is not installed still rejects as unsupported. A known live Frame with a missing
required world waits; retired ownership returns TargetClosed, Page crash returns
BrowserCrashed, and exhausted waiter admission returns ResourceExhausted.

The topology's borrowed Frame-to-Page iterator preserves ownership independently
of execution-world presence. Selected route preparation retains bounded sorted
ObjectId pairs. Only committed route replacement publishes a notification, after
the route mutex is released; dropped preparation never wakes a command.
Waiters register their notification before rechecking the authoritative exact
world route. Thus a commit between inspection and suspension is not lost, and
an intermediate removed context is not selected just because it once notified.
No extra generation counter, topology subscriber, CDP command, timer, task,
sleep, retry loop in JS, or committed-input replay is introduced.

The wait is caller-polled inside the existing controlled command after runtime
admission. It retains that command's original deadline and cancellation token;
readiness notifications do not restart the deadline. Public title/content and
evaluate retain their no-action-timeout behavior. Cancellation hooks themselves
keep immediate lookup and never await readiness. Page/Context retirement,
transport loss and dropped command futures use existing cancellation/cleanup
ownership, also used during host/Worker teardown.

The Node profile selects eight waiters and the topology's 64-Frame ceiling.
The additional declared BackendStateBytes ceiling is **20,736 bytes**:
`256 + 64 * 128 * 2 + 8 * 512`, with one state lease and up to eight waiter
leases. The two bounded arrays can coexist during route preparation. Atomic
waiter admission precedes its byte reservation, rollback/drop reclaims both,
and immediately ready operations need no waiter. Existing eight active-command,
sixteen pending-call, input-transaction and FFI limits are unchanged. This is
an accounting bound, not a measured RSS reduction or V8 allocation limit.
No dependency or browser process is added.

Nineteen new release tests cover missing/default route behavior, the default
unpolled snapshot, exact replacement worlds, local child ownership across a
world gap, world-kind separation, rollback, lost/spurious notifications,
retirement/crash, fixed original deadline, admission exhaustion, lease return,
transport shutdown without another topology event and dropped-future cleanup.
The complete composition lane passes **227**, with **14** pre-existing opt-in
browser tests ignored; topology **37** and safe-host **62** tests also pass.
Strict release Clippy passes for composition, topology, host and addon.

This deliberately covers command entry, not atomicity of the entire backend
operation across subsequent document replacement. A later dispatch race needs
its own explicit semantics; arbitrary evaluation and committed input must not
be replayed. Locator screenshot and Handle routes are not wired to this new
precondition, and child-Frame Node APIs remain unexposed. Narrow native
distributions, broader selectors/events/artifacts, NODE-022 cleanup policy,
intermittent NODE-013 setup behavior and process-memory comparison remain open.

### World-readiness artifact evidence (2026-09-07)

Logs are retained under `.work/node-world-readiness.YcU7iD/`. The first addon
`6690455c1e27c5914e7cc0b7b9f5e073716b7c966c1934e4206abf28e4509d8b`
used a bounded Frame map. Its five complete live lanes report actions **25/26**
(23.60 s), Locator **22/22** (22.99 s), evaluation **38/38** (26.97 s),
lifecycle **44/44** (72.95 s), and shared **36 matches / 19 unsupported /
2 oracle errors**, with zero value/candidate/setup/cleanup errors. Its shared
supervisor exits 1 without signal, timeout, overflow or native failure.
Only the ordinary NODE-022 Shift-state action comparison fails. The original
post-Enter case and two added replacement-document cases pass without retries.

The Frame map was then replaced with a bounded sorted array to make retained
capacity accounting explicit, and default call-construction snapshot timing
was preserved with its own regression test. The resulting final addon is
`1c8ee9ea2544c1c6e8f03ea5a74b4b29d390eb8cac4ca30b718a260224d9df65`.
Its separate `*-final.log` and `shared-final/` gates are recorded below; the
first artifact's passes are not substituted for final-artifact evidence.

Initial compile/test failures are retained separately: new route-error variants
needed exhaustive mappings, the test imported a profile from the wrong scope,
an Arc-wrapped deadline implementation violated the orphan rule, and the
missing-event-driver test first constructed a capability-free profile and
therefore correctly failed earlier validation. These test arrangements were
corrected without adding dependencies or weakening production validation.

Final-artifact gates complete without skips or cancellations:

| Lane | Result | Duration |
| --- | --- | --- |
| Actions | 25/26; NODE-022 remains failing | 19.94 s |
| Locator | 22/22 | 18.62 s |
| Evaluation | 38/38 | 19.90 s |
| Lifecycle | 42/44; NODE-014 and NODE-024 | 20.45 s |
| Shared API corpus | 36 match / 19 unsupported / 2 oracle errors | 52.71 s |

The original Enter comparison, concurrent post-Enter DOM/evaluation/Locator
reads, and three consecutive commit-navigation/action replacements pass on
this exact final artifact. NODE-022 still compares TimeoutError and identical
`aother` text but differs on the next key event's Shift modifier: Rust false,
pinned Playwright true. Its ordinary assertion is not skipped or normalized.

Lifecycle case 10 preserves a new NODE-014 observation: the fixture's captured
outer dimensions are **1282x800 in Rust, 0x0 in Playwright** this time, the
opposite direction of the earlier failure. Other viewport/media fields match.
Do not infer a stable zero-window implementation defect from either snapshot.
Case 36 fails in the candidate half at the independent controller's navigation
after disconnect, before retained-metadata assertions (NODE-024). It is not
evidence that those final assertions matched. All 44 cases completed and the
supervisor reports exit 1 without a signal; no native failure was reported.

`disconnect-order-probe.mjs` performs exactly three independently owned pairs,
with a 60-second watchdog and a 64-event-per-observation limit. All six
observations pass. In all three candidate observations, the controller receives
the previous document's load during its next-navigation stage; oracle delivery
occurs earlier in these samples. This records cross-connection delivery timing,
not a proven root cause or repaired lifecycle gate. The full 42/44 failure is
retained; neither the diagnostic nor the first artifact's 44/44 replaces it.

The shared final report completes at `2026-09-07T01:48:26.443Z`. Its native
hash matches the final artifact; value/candidate/setup/cleanup errors are zero.
The supervisor exits 1 with no signal, timeout, overflow or native failure.
The nineteen unsupported cases and two oracle history failures remain
non-passing. No shared spec, fixture, golden, timeout or browser launch flag
was changed. Earlier NODE-013 failures remain unexplained despite this run
having no attach setup error.

Final-source static checks pass: workspace release check, strict changed-crate
Clippy, package offline tests **9**, shared runner unit tests **30**, both
TypeScript checks, safe-host boundary **221** normal packages, and all **30**
umbrella dependency profiles. Production source stayed frozen across these
five final live lanes and the diagnostic. Formatting and whitespace checks pass;
the shared specs, fixtures and goldens have no diff.

Cleanup closes only this task's verified preflight target
`A27EA7F550E03940857237010CB90243` through its original Pwright state directory.
CDP discovery confirms it is absent. The external Chrome identity remains
`33b77c01-a5a4-42e4-b93b-ab822942d338`; no browser restart or unrelated Page close
was performed. HEAD remains the foundation commit `f76e3f2`; the Node work is
still uncommitted, with no additional commit or push. The goal remains active.

### Navigation-order investigation and semantic Locator binding

The unchanged `1c8ee9ea...d9df65` artifact now has a caller-released dynamic
HTTP test: the replacement document commits with its response body deliberately
unfinished. Native URL publication, title and evaluate remain usable while
`goto(waitUntil: 'load')` stays pending; only releasing the response completes
navigation. This gate passes against both implementations. Its 45-case full
lifecycle run passes 44, failing the same NODE-014 dimensions (Rust 1282x800,
oracle 0x0); NODE-024 does not reproduce. This is evidence for per-navigation
load waiting, not a repair or dismissal of cross-connection ordering.

Both Rust and pinned Playwright correlate new-document navigation with the
loader returned by Page.navigate. Playwright also accepts a different committed
document as an interruption. Independent controller delivery is not globally
ordered by another connection's goto/disconnect. No observer fence, retry,
timeout growth, browser restart or sleep was added to the NODE-024 comparison.
The new load test observes the native URL condition through the existing test
snapshot helper; it adds no production polling. Dynamic response fixtures bind
to Tailscale; all static files still use serve-lib.

Node now exposes all seven user-facing Locator constructors on Page, main Frame
and Locator: getByText, getByLabel, getByPlaceholder, getByAltText, getByTitle,
getByTestId and getByRole. String/exact and ECMAScript RegExp inputs construct
existing typed Rust selector leaves. Role exposes the pinned 1.58.2 name,
exact, checked, disabled, expanded, includeHidden, level, pressed and selected
options. The newer checkout's description option is not in that pinned API
and explicitly rejects. Mutable custom test-id configuration, RegExp filters
and complete selector-language parsing remain separate work.

The facade only projects argument shapes and RegExp source/flags; it never
constructs selector programs, resolves the DOM or performs input. Native
query drafts reserve count and aggregate byte capacity before string copying.
UTF-16 validation storage is dropped before UTF-8 copying, within the existing
two-copy allowance; unpaired UTF-16 explicitly rejects rather than replacing
characters. Test-id includes its implicit eleven-byte data-testid attribute.
Role input conversion is bounded, unsupported role/filter combinations remain
lazy fixed issues, and operation preparation still reserves a separate selector
clone. Limits remain 1,024 recipes / 8 MiB per environment, depth 32, 256 nodes
and 64 KiB aggregate selector strings. There is no new dependency, task,
browser process, query service or native handle.

The first semantic artifact `7c6314bf...b4d0e0c` passes 28/30 Locator cases,
with one timed-out composite query and one downstream FFI-count assertion while
that interrupted test still had an operation pending. A count-first diagnostic
finishes 29/30 without cancellation: only the Shadow DOM role-name query returns
zero. Its button has both text and title. The shared Rust selector runtime
incorrectly preferred title; for roles that permit name-from-content, nonempty
content now precedes the title fallback while explicit labels/native
alternatives still win. Added direct/explicit-role/link/empty-content/ARIA-label
comparisons retain the exact priority contract. This does not claim the entire
accessible-name algorithm, including slots/pseudo-content/hidden descendants,
is equivalent.

The selector runtime advances to version 7. During this change, an obsolete
version-6 unit assertion failed, followed by a real-browser regression that
identified 22 separately hardcoded version guards. The failed
`a182deeb...b6fa4f` lane was explicitly stopped after this cause was known;
it is not a pass or an unexplained timeout. All 22 Locator script guards now
interpolate the selector runtime's exported symbol/version constants, with
matching unit assertions, rather than maintaining independent literals.
All exact failure logs remain in `.work/node-navigation-order.KIOgvf/`.

The shared adapter now uses public getByText and getByRole. Its legacy label
case still requires Locator.evaluate to observe tag names; it remains
unsupported rather than substituting inputValue, a new fixture assertion or
a browser-evaluation fallback. Existing shared specs, fixtures and goldens
remain unchanged. Package-local tests separately verify actual label lookup.

The constant-guard artifact `ca9a906b...bcc414` completes 31/32 Locator
comparisons. It fixes the role-name/title cases but retains a newly exposed
global role-name RegExp failure: consecutive matching buttons count 2 instead
of Playwright's 3, on both reads, while caller `lastIndex` remains 2. Pinned
1.58.2's `matchesAttributePart` uses `String.match`; our role predicate used
stateful `RegExp.test`. Role name/description patterns now use the attribute
matching primitive. Text and label regex engines remain separate and unchanged;
their upstream text matcher intentionally uses `test`. The repeated-sibling
comparison stays an ordinary mandatory test. No regex flags, result values,
candidate counts or caller state are normalized to obtain a match.

### Semantic Locator artifact evidence (2026-09-07)

Handoff-stage release addon SHA-256:
`edd8ba969f29904184da9cc025397638cad99bd00860090c6ece267bf2b0c9b7`.
Production source and native binary stayed unchanged throughout all five full
lanes. Chrome is 152.0.7977.76, Node 26.7.0, macOS arm64; oracle is 1.58.2.
Evidence is `.work/node-navigation-order.KIOgvf/`:

| Gate | Result | Duration |
| --- | --- | --- |
| Locator | 32/32 | 35.87 s |
| Actions | 25/26; only NODE-022 | 26.95 s |
| Evaluation | 38/38 | 28.91 s |
| Lifecycle | 45/45 | 83.60 s |
| Shared, all 57 unchanged cases | 41 matches / 14 unsupported / 2 oracle errors | 62.60 s |

Package logs are `locator-final.log`, `actions-final.log`,
`evaluation-final.log` and `live-final.log`. Shared evidence is `shared-final/`,
completed `2026-09-07T02:32:42.941Z`. Its supervisor records exit 1 with no
signal, timeout, overflow or native failure; value/candidate/setup/cleanup
errors are zero. All package cases completed without cancellation or skips.
The action failure still compares Shift false against oracle true after timeout.
No input policy change was made. NODE-014 and NODE-024 do not reproduce on this
artifact, but remain open; no production navigation/window fix is claimed.

The five new shared matches are substring/exact text plus implicit button,
implicit link and explicit role lookup. Two label cases still encounter a
declared adapter gap because their original observation requires
Locator.evaluate. The other twelve unsupported cases cover reload, typing,
raw keyboard, screenshots, download, upload and the legacy touchscreen case's
mouseClick operation. Both history cases retain oracle errors and candidate
unsupported diagnostics. No shared spec, fixture, golden, timeout, assertion
or classification was changed to obtain this result.

Final release unit tests pass: Locator backend 100, selector runtime 4, safe
host 68. Strict Clippy on all four changed crates, workspace release check,
format and diff checks, Node offline 9, shared-differential unit 30, legacy CLI
harness 67, both type checks and the 221-package boundary pass. Locator-only
and Locator-composition dependency proofs remain 10 and 18 Pwright packages;
all 30 umbrella profiles pass. A direct umbrella-script invocation first
reported permission denied because it is not executable; the complete explicit
`bash` invocation passes, retained in `final-umbrella-bash.log`. This was a
command invocation error, not a dependency proof failure.

All supervised test processes were terminal before final discovery. The
preflight target `9045750AD60E1C756DF4A13B6F6AD3CA` was already absent, so no
close command was sent against stale Pwright state; its disappearance was not
attributed to an agent cleanup action. The unrelated XLQY Page remained and
Chrome browser identity stayed `33b77c01-a5a4-42e4-b93b-ab822942d338`. No browser
restart, broad target cleanup, new commit or push occurred. Foundation commit
remains `f76e3f2701a8ccde0474b5accf40dc328aaa511c`. The broad Node-compatibility
goal remains active; this checkpoint is not full API, distribution or RSS parity.

### Locator evaluation ownership prerequisite (2026-09-07)

The new [focused design](knowledge/pwright-locator-evaluation.md) records the
pinned Locator.evaluate phase contract: strict attached resolution first,
source/argument conversion afterward, main-world evaluation of the exact node,
and final disposal. Its timeout does not limit execution of the user function.
The existing Page.evaluate eager structural snapshot cannot simply be reused
at the initial Locator call. Node Locator.evaluate is still not exposed.

Independent backend and continuation leaves now implement one group-owned
utility-world resolution attempt, with five/seven-package normal closures.
They reuse the selector model/runtime, inject never-reused group IDs, reserve
one native-handle/owner budget and selector scratch before I/O, and retain the
original cleanup slot through release. Native cleanup failure, abandoned raw
ownership and discarded executor jobs terminally close further admission.
Main-world adoption, strict waiting and Node conversion/command binding remain
required work. Ordinary Locator dependencies and the Node profile do not select
these leaves, so no count/text consumer pays their ownership cost.

Final Rust release tests pass **15 backend / 10 composition**, including failed
cleanup and abandoned-executor cases. The real Chrome test verifies utility-
world label/Shadow DOM resolution despite a main-world prototype override,
exact detached-node identity and physical native-object invalidation after
explicit/drop-driven release. The successful test returns all leases and
continuation slots. A release failure is not treated as successful cleanup just
because local counters return to zero. Logs: `.work/node-locator-resolution.yiEN25/`
`guard-units.log`, `guard-clippy.log`, `guard-live.log`.

The shared CDP trait gains two explicit group-owned primitives. Existing methods
are unchanged, but its Node addon was rebuilt and all five gates rerun. Tested
SHA-256 is `db8f7337a628d4424e4ed252d8e2e27ed465b07a17878f72fcd6b0f6e6b7ea84`:

| Gate | Result | Duration |
| --- | --- | --- |
| Locator | 32/32 | 28.24 s |
| Actions | 25/26; NODE-022 | 19.70 s |
| Evaluation | 38/38 | 20.12 s |
| Lifecycle | 45/45 | 18.62 s |
| Shared, all 57 cases | 41 matches / 14 unsupported / 2 oracle errors | 52.69 s |

Package logs are `node-{locator,actions,evaluation,live}.log`. The complete
`shared/` report finished `2026-09-07T02:58:40.325Z`; its supervisor exits 1
with no signal, timeout, overflow or native failure, and zero value/candidate/
setup/cleanup errors. Neither NODE-014 nor NODE-024 reproduced; no fix for them
is claimed. The first short native-resolution Chrome test overlaps the start
of the Node Locator gate on independent owned Contexts, so these timings are
not an isolated performance comparison. Shared specs/fixtures/goldens remain
unchanged, and the two label observations stay unsupported pending actual Node
Locator.evaluate. Earlier failing artifacts remain part of the record.

Strict Clippy, workspace release check, both type checks, Node offline 9,
shared-differential unit 30, the 221-package Node boundary, the new five/seven-
package closures, existing 10/18-package Locator proofs and all 30 umbrella
profiles pass. Shared CDP and fake-client release tests also pass 67 and 23
respectively (`cdp-fake-units.log`). Final format/diff checks pass. Only preflight
`CFB0804177D51E1C143D9C01EE7B81ED` was closed by
the agent after runners finished; discovery confirms it absent and Chrome's
original `33b77c01-a5a4-42e4-b93b-ab822942d338` identity alive. No browser restart,
new commit or push occurred. The full goal remains active.

### Locator strict waiting checkpoint (2026-09-07)

The independent resolution composition now includes a bounded strict waiter.
It receives current utility-world routes, retry pacing and the original
deadline/lifecycle future from its caller. Every attempt revalidates that exact
route. A stale successful result is released before re-selection; a CDP error
is retried only with authoritative world invalidation. Strict violations,
invalid selectors, malformed responses and cleanup/resource failures are not
hidden behind retries. Cancellation continues only an already-dispatched
query/release, never the wait loop or user function.

One installed 256-byte counter lease enforces waiter capacity across clones,
independently of the shared production ledger's aggregate ceilings. One active
waiter adds one lease for 1,024 internal state bytes and two bounded selector
trees. Retained string capacity is checked separately from logical length.
The caller can lend one pinned termination future so selection and adoption
share a deadline without imposing it on subsequent user evaluation. See
[the exact API and resource contract](knowledge/pwright-locator-evaluation.md#strict-waiting-checkpoint-2026-09-07).

Release evidence in `.work/node-locator-wait.HOnILm/`:

- 15 backend / 32 composition unit tests pass (`units-complete.log`), including
  22 new strict-wait cases. The capacity test caught and corrected a ledger-only
  admission implementation. Intermediate compile/test-helper failures remain
  recorded separately and are not counted as passing evidence.
- Both real Chrome tests pass (`live-final.log`, 0.21 s / 0.48 s): delayed
  hidden/disabled attachment, document replacement with new CDP-observed worlds,
  missing timeout, strict violation, actual target close, detached-node identity
  and physical remote-object invalidation. Cleanup leaves zero test leases and
  reservations and preserves the external browser.
- Strict Clippy, exact five/seven-package dependency proofs, workspace release
  check, all 30 umbrella profiles, nine Node offline tests, Node types and the
  existing 221-package native boundary pass.

This is not the public Node method. The real test has a single-Frame route
driver; production topology integration, utility-to-main adoption, deferred
source/argument conversion and paired Locator.evaluate observations remain.
Node neither selects nor links the new waiting API. Its artifact hash is
unchanged (`db8f7337...b7ea84`), and this checkpoint does not claim a rebuilt
addon or a newly run full Node differential corpus. The previous shared 41/14/2
and action 25/26 outcomes, including NODE-022, remain the recorded Node results.
After the runners exited, only this checkpoint's preflight target
`357CACA86319831637BD330E9D0D4ACC` was closed. Discovery confirms it absent,
the preexisting XLQY Page untouched and Chrome's original browser identity
still alive. No commit or push was made; the full compatibility goal remains active.

### Locator main-world evaluation checkpoint (2026-09-07)

`pwright-composition-chromium-locator-evaluation` now composes the selected
element owner with native main-world adoption and one Rust value evaluation.
It is an independently opt-in 15-package normal Pwright closure, not a Node
installation or portable Handle API. Its caller supplies an exact same-session
main-world numeric/unique route and retains the resolution deadline during
adoption. Function source and arguments are accepted only after adoption.
Selection is not repeated, including after detachment or replacement, and user
code is never replayed. Source/argument bounds precede copying into the owner.

The raw adoption request accepts an optional original `objectGroup`, so both
utility and main-world wrappers are released together. Unknown native IDs from
malformed/lost resolve responses remain covered by group compensation. Another
RemoteHandle, bounded owner bytes, an installation-wide capacity counter and a
second caller-owned continuation reservation are acquired before adoption.
They survive through value/error retention and acknowledged group cleanup.
The existing value-evaluation factory separately budgets execution. See
[the exact accounting and ownership contract](knowledge/pwright-locator-evaluation.md#main-world-adoption-and-evaluation-checkpoint-2026-09-07).

An unpolled evaluation drop releases only; a started drop continues that exact
future through Promise completion and release. Cleanup failure retains the
primary error and closes admission across clones. An executor that discards
its admitted continuation also closes admission; zero host counters do not
prove native reclamation. No browser process, timer, cleanup task or CDP driver
is spawned by this leaf.

Release evidence in `.work/node-element-evaluation.D9og5u/`:

- 24 leaf unit cases pass (`units-guard.log`) and strict Clippy passes
  (`clippy-guard.log`). Broader backend Handle/CDP/fake/resolution tests pass
  17/67/23/15/32 (`units-all.log`); this earlier file has 20 leaf cases, before
  four final guards were added. Early live-test compile errors are retained in
  `clippy.log`, not counted as passing evidence.
- One real Chrome 152 case passes (`live-final.log`, 0.34 s). It verifies
  label-selected detached identity despite a main-world query override, main-
  world globals, BigInt/TypedArray values, structured TypeError, no execution
  on unpolled drop and exactly one execution after cancellation of a dispatched
  Promise. Both native IDs are physically invalid after cleanup; test leases
  and reservations return to zero. The test bounds its cleanup wait and
  disposes its owned Context while preserving the external browser.
- `dependencies-final.log` proves the 15-package closure, without aggregate
  composition, provider, portable Handle lifecycle, codec/server or Node.
  Resolution closures remain five/seven packages. Workspace release check,
  all 30 umbrella profiles, Node offline 9, shared unit 30, both type checks
  and the 221-package Node boundary pass.

The CI workflow now invokes both new Locator dependency-proof scripts; their
local runs pass. No remote CI run is claimed without a push.

Shared CDP/Handle changes required rebuilding the Node addon even though Node
does not select the new leaf. All five full lanes were rerun on SHA-256
`42a26fd0b5325c393b930a89f18dc628959c11bfa884e293756fb19c9d734e94`:

| Gate | Result | Duration |
| --- | --- | --- |
| Locator | 32/32 | 53.161 s |
| Evaluation | 38/38 | 48.937 s |
| Lifecycle | 44/45; NODE-014 | 23.958 s |
| Actions | 25/26; NODE-022 | 25.590 s |
| Shared, all 57 unchanged cases | 41 matches / 14 unsupported / 2 oracle errors | 51.743 s |

The package logs are `node-{locator,evaluation,live,actions}.log`. NODE-014
reproduces in the default viewport/media comparison: Rust outer window
1282x800, oracle 0x0, with the other compared fields matching. NODE-022 again
observes Shift released by Rust but retained by Playwright after delayed
Shift+Z timeout; both sides still have the same TimeoutError and text.
Neither is skipped or normalized, and no green retry replaces the failures.
NODE-024 does not reproduce in this run; it is not declared fixed.

The `shared/` report starts `2026-09-07T03:57:47.514Z` and completes
`2026-09-07T03:58:39.257Z`. Supervisor status is 1, with no signal, timeout,
overflow or native failure and zero value/candidate/setup/cleanup errors.
The two oracle errors remain history back/forward. Specs, fixtures and goldens
are unchanged. The two label observations remain unsupported because public
Node Locator.evaluate is still missing. Initial Node lanes overlap release
checks on this host; the durations are not isolated performance/RSS evidence.

Next required gates, before exposing the Node method:

1. Production Frame route pairing and authoritative invalidation: utility
   selection and main-world adoption must refer to the same live document;
   only pre-user-code work may retry under the original deadline.
2. Controlled-runtime admission and lifecycle cancellation for retained
   prepared owners and started evaluations, including authoritative context/
   session-retirement proof. Do not bypass runtime policy with a direct host
   CDP shortcut or treat release error strings as successful cleanup.
3. Bounded Node-API references/settlement and source/argument conversion after
   adoption. A conversion exception must explicitly dispose the prepared
   owner rather than waiting for GC; Worker teardown must preserve ownership.
4. Paired public tests for conversion/getter timing, mutation/detachment,
   deadline phases, close/navigation and cleanup, followed by all unchanged
   Node differential lanes. Only then may shared label observations count as
   new compatibility evidence.

After all runners completed, only this task's preflight target
`365A63420F1B6915479481AB1C9DE0D0` was closed with Pwright. CDP discovery verifies
it absent and Chrome browser identity `33b77c01-a5a4-42e4-b93b-ab822942d338`
remains alive. No browser restart, new commit or push occurred. The full goal
remains active.

### Production paired-world access checkpoint (2026-09-07)

The next Rust prerequisite is `ChromiumFrameWorldAccess`, independently
installed over the existing production `CdpPageServices` route table. Its
non-cloneable snapshot pairs exact Frame main/utility identities and one
shared native client, admits before polling and retains its declared resources
through caller use. Invalidation is caller-polled through existing bounded
world-readiness notifications; there is no new driver, task, timer, topology
copy or subscription. Defaults and Node profile selection are unchanged.
See [the full API and resource contract](knowledge/pwright-locator-evaluation.md#production-paired-world-access-2026-09-07).

The topology exporter now requires each world's session to equal the Frame's
authoritative renderer session. Old worlds retained during handoff or swap
gaps remain inventory, not current command routes. Late old-renderer world
announcements cannot overwrite the new renderer's current world. This does
not prove native destruction or permit skipping group release. An observed
snapshot cannot eliminate the gap between browser navigation and event
delivery; selection/adoption must still revalidate under the original deadline
without replaying user code.

Native evidence in `.work/node-frame-worlds.qDq1XE/`:

- `units-complete.log`: topology 40 passing; Chromium composition 247 passing
  and 15 explicitly ignored browser cases. This increment adds 20 paired-world
  access unit cases and three topology route/session regression cases.
- `live-2.log`: one real Chrome 152 case passes in 0.16 s using the actual
  topology event driver. It covers main/utility isolation, navigation replacing
  both worlds, invalid old world IDs, exact local-child pairing, detached-child
  invalidation, owned Context closure, returned resource allowances and the
  external browser surviving. Cross-process handoff is unit-tested, not
  claimed as live OOPIF evidence in this checkpoint.
- The legacy inventory fixture now announces its remote child's world on that
  child's renderer session; its exact retained-string expectation changes from
  156 to 162 bytes. The first real-browser attempt (`live.log`) failed because
  the test did not register its newly owned Context with production topology.
  The final test explicitly registers ownership; it does not weaken foreign-
  Context exclusion. Both failures remain recorded, not counted as passes.
- Initial compile/import errors and a guessed fixed-state allowance caught by
  tests were corrected before the final runs. State accounting now derives
  from `size_of::<State>() + 256`; per-snapshot accounting remains 8,192 bytes.
- Strict release Clippy (`clippy-complete.log`), workspace release check,
  all 30 umbrella profiles and independent Locator resolution/evaluation
  closures of five/seven/fifteen packages pass. No Cargo edge was added.
  Node offline 9, shared unit 30, both type checks and the unchanged 221-package
  Node boundary pass.

The production topology changes were rebuilt into addon SHA-256
`72644cf9eb2285380af2546833f18796dae7b5ee407864b83ee5cb4471b9ddcd`.
The first package supervisor sequence overlapped the shared corpus on the
same dedicated Chrome. The retained `node-{locator,evaluation,live,actions}.log`
and `shared/` report record these failures, not clean compatibility results:

| Overlapping gate | Result |
| --- | --- |
| Locator | 27/32; five candidate topology-initialization errors/timeouts |
| Evaluation | 34/38; four candidate topology-initialization errors/timeouts |
| Lifecycle | 43/45; one candidate topology-initialization error plus NODE-014 |
| Actions | 25/26; NODE-022 |
| Shared, all 57 | 34 matches / 10 unsupported / 2 oracle errors / 11 setup errors |

Ten shared setup errors report missing CDP sessions during attachment; the
remaining setup error contains a TargetClosed newPage failure and nested
browser-close missing-session cleanup error. A top-level cleanup-error count
of zero does not erase that nested failure. Setup failures prevent some cases
from reaching their unsupported operation, so the lower unsupported count is
not increased API coverage. These observations extend the unresolved NODE-013
attachment evidence. Concurrent target churn is a hypothesis, not a proven
root cause or excuse to suppress session errors.

All first-run supervisors finished normally with nonzero exits before the
same binary was rerun serially. No browser restart, timeout increase, case
filter, source/fixture/golden change or hidden retry was used. Serial results
are recorded separately below; they do not replace this failing evidence.

| Serial gate, same artifact | Result | Duration |
| --- | --- | --- |
| Locator | 32/32 | 56.106 s |
| Evaluation | 38/38 | 49.451 s |
| Lifecycle | 44/45; NODE-014 | 23.597 s |
| Actions | 25/26; NODE-022 | 19.676 s |
| Shared, all 57 unchanged cases | 41 matches / 14 unsupported / 2 oracle errors | 52.083 s |

Serial package logs are `serial-{locator,evaluation,live,actions}.log`. The
`shared-serial/` report begins `2026-09-07T04:46:36.083Z` and completes
`2026-09-07T04:47:28.166Z`. It verifies the same native artifact, fixture hashes
and runner source hashes as the overlapping report. Shared value/candidate/
setup/cleanup errors are zero; supervisor exit 1 has no signal, timeout,
overflow or native failure. Both history cases remain oracle errors, and both
label observations remain unsupported. Serial package attachment failures do
not reproduce. This comparison supports further investigation of target churn;
it does not establish cause, a fix for NODE-013, or safe concurrent attachment.

NODE-014 still observes Rust outer dimensions 1282x800 versus oracle 0x0, with
the other compared settings matching. NODE-022 still observes Shift false in
Rust versus true in the oracle after the same delayed-press TimeoutError and
text. NODE-024 does not reproduce; no ordering fix is claimed. None of these
assertions or classifications was skipped, normalized or rewritten.

Next implementation step is a production selection/adoption phase that holds
this snapshot under the original resolution deadline, revalidates both sides
of adoption, and retains native group cleanup through authoritative lifecycle
retirement. Integrate it with controlled-runtime admission/cancellation before
adding the Node two-phase reference/settlement boundary. Source/argument
conversion must occur after native preparation, with explicit disposal on
conversion failure. Public Locator.evaluate, shared label matches, narrow
distribution and process-memory parity remain unimplemented/unverified gates.

After every runner finished, Pwright closed only this task's preflight target
`C24673019B1812C26A6E124FBFDD42DD`. CDP discovery then reported no Page targets;
Chrome identity `33b77c01-a5a4-42e4-b93b-ab822942d338` remained alive. Final
formatting and diff-whitespace checks pass, and the addon hash is unchanged.
No new commit or push was made; HEAD remains the requested local foundation
commit `f76e3f2`. This is a completed Rust prerequisite, not completion of the
active Node compatibility goal.

### Paired element preparation checkpoint (2026-09-07)

`LocatorElementPreparationService` now connects strict utility-world selection
to main-world adoption through an injected `PairedWorldSource`. It validates
the selected element against that exact pair, checks both worlds before and
after adoption, and retries pre-user-code work only after acknowledged cleanup
and independently observed retirement. One original stop future governs
readiness, selection, adoption, stale cleanup and pacing; it does not become
a deadline for later arbitrary user evaluation. No user source/argument is
accepted by the preparation phase, and it returns a single-use native owner,
not a public portable Handle. See
[the complete contract](knowledge/pwright-locator-evaluation.md#paired-selection-and-adoption-phase-2026-09-07).

The route source is generic and stays outside the fifteen-package normal leaf
closure. A test-only assembler connects it to the actual production
`ChromiumFrameWorldAccess` and topology event driver. The Node host has not yet
installed this assembler or typed preparation/evaluation commands. This is
verified Rust phase composition, not a newly supported Node method.

Cancellation required extending the resolution continuation owner: optional
`ResolutionRouteSource::retain_route` / `call_retaining` keep the paired route's
resource guard through a started query, returned element, adoption/evaluation
and group release. The guard follows cleanup jobs instead of being dropped
with the awaiting phase. Stateless sources retain their old behavior. Native
cleanup failure/discard still closes admission; a route change or a missing
CDP session is not accepted as proof of native group reclamation.

Preparation declares 256 installed backend bytes, 8,192 bytes per admitted
operation, one original selector tree per operation and independent capacity
across clones. Both the generated operation and private route future are
size-checked before boxing. A first 4 KiB estimate failed at 4,304 bytes; the
final contract enforces its 8 KiB total and 1 KiB nested adapter ceilings and
has explicit oversized-stop/source rejection tests. Native services, snapshot
storage and cleanup execution keep their own resource declarations.

Evidence in `.work/node-element-preparation.YxRNYz/`:

- `units-final.log`: resolution composition 37 passing; evaluation composition
  41 passing and two explicitly ignored Chrome cases. This adds five route-
  retention and seventeen preparation tests. They cover readiness/adoption/
  cleanup cancellation, original deadline, no reselect after preparation,
  stale success cleanup before retry, strict/invalid-pair errors, failed cleanup
  remaining terminal, concrete future bounds and retained paired ownership
  through cancelled user evaluation.
- `live-serial.log`: both Chrome cases pass in 1.27 s, explicitly serial. The
  new production-port case waits for a delayed hidden label target, adopts it,
  replaces/detaches the DOM node, then evaluates the original exactly once in
  main world. It checks typed results and both native IDs with the same valid
  lookup before and after cleanup, verifies snapshot allowance return, and
  times out a missing selector. Only its owned native Context is disposed.
  The earlier standalone preparation pass (`live.log`, 0.29 s) and default
  test-runner two-case pass (`live-final.log`, 0.39 s) remain distinct evidence.
- `clippy-final.log`, `workspace-final.log`, `resolution-dependencies.log`,
  `evaluation-dependencies.log` and `umbrella.log`: strict release Clippy,
  workspace release check, exact five/seven/fifteen normal leaf closures and
  all 30 umbrella profiles pass. Two new direct selector edges were already
  in the evaluation closure; aggregate/codec/server integration is dev-only.
- `node-offline.log`, `node-types.log` and `node-boundary.log`: nine offline
  cases, types and the unchanged 221-package Node boundary pass. Node has no
  normal edge to the preparation leaves, and no production JS/CDP/process
  launcher import was added.

The first stable-Rust compile rejected an opaque associated future type; it
was replaced with the admitted boxed boundary. Initial live-test argument/type
errors are retained in `units.log`; no failed compile is a browser pass. The
pre-boxing size tests were added rather than deleting the failed bound check.

The Node addon remains unchanged at SHA-256
`72644cf9eb2285380af2546833f18796dae7b5ee407864b83ee5cb4471b9ddcd`.
This Rust-only change is outside its normal closure; the addon was not rebuilt
and the five browser lanes were not rerun in this checkpoint. Its preceding
serial 32/32 Locator, 38/38 evaluation, 44/45 lifecycle, 25/26 actions and shared
41/14/2 remain artifact-specific prior evidence, alongside the failed
overlapping attachment experiment. No new shared label match, closed discrepancy,
public Locator.evaluate, distribution or RSS claim is made.

Next: install the phase in Node host through controlled-runtime command and
target admission, preserve authoritative transport/session/Context retirement
and group-cleanup outcomes, then add bounded prepared-owner Node-API references.
Defer source/argument conversion until preparation completes; explicitly
dispose on conversion failure and preserve ownership through Worker teardown.
Only paired public timing/identity/deadline/error/cleanup comparisons and the
unchanged full corpus can enable the public method.

After all runners completed, Pwright closed only this checkpoint's preflight
target `08038932A50532730C04E984D8967C99`. Discovery verified it absent while
the two unrelated Page targets remained. Chrome browser identity
`33b77c01-a5a4-42e4-b93b-ab822942d338` stayed alive. Formatting and diff checks
pass. No browser restart, new commit or push occurred; HEAD remains the local
foundation commit `f76e3f2`, and the full Node compatibility goal stays active.

### Controlled preparation dispatch and attachment pressure (2026-09-07)

`ControlledDispatcher::dispatch_with` accepts an explicitly supplied typed
handler/cancellation pair without creating another runtime. Ordinary
`Service::call` delegates to the same implementation. Both paths share the
actual object graph, pending-call table, target/connection schedulers, deadlines,
resource ledger and abandoned cleanup executor. It neither installs nor
advertises a wire capability; handler/native continuation resources remain
separately declared. Nine new deterministic cases verify admission contention,
different request types, active/queued deadlines, target validation, parent
disposal, transport closure and matching drop-driven cancellation.

A new test-only preparation command also uses the real production Frame-world
accessor and controlled Chromium runtime. It proves successful selection/adoption
and physical group reclamation; preparation times out while an installed value
evaluation holds the same Frame lane, without starting the preparation backend.
The blocker uses an explicit completion signal, not a guessed delay. A started
missing-selector timeout separately drains native cleanup and checks health.
Its returned owner is explicitly released by the test: this does **not** provide
Node lifecycle ownership after a command returns. Its test-only error mapping
and cancellation acknowledgement are not public parity or native-release proof.

Rust evidence in `.work/node-controlled-preparation.vZZhDf/`:

- `runtime-composition-units.log`: 113 runtime (nine new), 247 aggregate
  composition, 31 runtime-control and 16 Tokio runtime cases pass. Aggregate's
  15 ignored live cases are not counted as passing tests.
- `leaf-units-final.log`: 41 evaluation-composition cases pass, three live
  cases ignored. `chrome.log` runs all three serially: 3/3 in 2.16 s, before
  the later attachment-pressure failures.
- Strict release Clippy, workspace release check, formatting, 5/7/15 leaf
  closures, 30 umbrella profiles, Node offline 9, shared harness 30, both type
  checks and the 221-package Node boundary pass. Browser-model is a new dev-only
  fixture import; no normal closure or production profile grows.
- `probe-units.log`: five bounded attachment-probe cases pass. It now counts
  at most 64 event methods with 128-byte keys, no event payload retention and
  explicit overflow reporting. Existing command-history bounds remain.

Initial test-helper compile failures are not live passes (`leaf-units.log`).
An incorrect unit assertion about dead-target timer ordering was corrected:
existing kind-before-timer and liveness-after-timer behavior was preserved.

The shared runtime change required an addon rebuild. SHA-256:
`3ff763b27be312a62e215b6f6d3a16ef809216bbd23cba51df09b3f15b7594ba`.
All five full browser lanes ran serially against unchanged specs/fixtures:

| Gate | Result | Duration |
| --- | --- | --- |
| Locator | 32/32 | 42.859 s |
| Evaluation | 38/38 | 25.849 s |
| Lifecycle | 25/45: NODE-014 plus 19 attachment failures | 34.114 s |
| Actions | 0/26: all attachment failures | 21.972 s |
| Shared, all 57 cases | 57 setup errors; zero matches | 47.634 s |

Lifecycle attachment failures begin at case 27, after 25 passes and the existing
outer-window discrepancy (Rust 1282x800, oracle 0x0). Actions/shared operations
were never reached; zero unsupported/oracle-error counts do not mean those
deficits are fixed. NODE-022's modifier behavior was not reached this time.
Shared runs `05:37:54.255Z`–`05:38:41.889Z`; supervisor status 1, no signal,
timeout, overflow or native failure, and zero top-level cleanup errors. No
green retry or replacement for the failed gate was made.

#### NODE-025: attachment event-item exhaustion (open)

`attach-diagnostic.log` independently reproduces the failure outside Node-API:
at `05:39:14.413652Z`, the Rust connection reader logs
`global event items usage 1025 cannot grow by 1; limit is 1025`, then closes.
`attach-events.log` makes one further diagnostic attempt: 66 ms, 32 commands /
31 responses, pending `Runtime.enable`, no recording overflow. It counts 1,012
`Runtime.executionContextCreated`, six console events, nine target attaches,
one detach and one worker-loaded event. Both diagnostic tests fail and remain
failed evidence; the Chrome browser process itself stays alive.

The initialization path awaits session prerequisites, including Runtime.enable,
before resuming event draining. The bounded reclaiming fanout and an event held
during initialization are concrete retention points to investigate. No event
consumption fix or attribution of the worlds to Rust versus Playwright is
claimed. The pinned Playwright utility-world name includes a fresh Page GUID;
NODE-016 already tracks repeated attachment on surviving Pages. This observation
is distinct from NODE-013's earlier timeouts, not their proven universal cause.
It is not a renderer RSS measurement.

Next: address this reproducible pressure boundary with explicit caps and correct
topology ordering, then finish host-owned prepared values, authoritative native
retirement, deferred Node conversion and Worker cleanup before public
`Locator.evaluate`. No cap increase, unrelated Page reload/closure, browser
restart, new commit or push occurred; the broad goal remains active.

After all browser supervisors and diagnostics finished, Pwright closed only
the task-owned preflight target `5B6AA26261E2DB2AC7F717B8F4F620A7`. Discovery
verified it absent and preserved all six unrelated Gemini Pages then present;
Chrome retained browser identity `33b77c01-a5a4-42e4-b93b-ab822942d338`.
`cleanup.log` records that exact close. Final probe Clippy, formatting and
diff-whitespace checks pass; the tested addon hash remains unchanged.

### Optional world-event admission checkpoint (2026-09-07)

NODE-025 now has a scoped implementation and regression evidence, not a general
attachment or RSS fix. The CDP production profile accepts one explicit immutable
`CdpEventAdmissionPolicy`; duplicate installation fails. Default, legacy and
unselected aggregate connections remain unfiltered. The reader enforces raw and
individual event byte limits first, then classifies before retained event
leases, session copies, shared allocation and fanout. Responses use their
unchanged branch. Needed-event exhaustion remains terminal.

The Chromium composition supplies a conservative main/Pwright-world policy.
Only schema-valid, explicitly non-default Frame-world creation with a nonempty
session and a foreign world name may be omitted. Main, possible Pwright,
malformed/unknown and all other event kinds stay visible to the existing exact
topology validation. The Node profile selects this because its current consumers
do not require arbitrary foreign-world inventory; future consumers must revisit
the connection-wide delivery contract. No queue, task, mutable table, capacity
increase, Cargo edge or native-world destruction is added. Full contract:
[CDP event admission](knowledge/pwright-cdp-event-admission.md).

Evidence directory: `.work/node-world-event-admission.SCODws/`.

- Six reader/profile cases exercise the actual reader, including 4,096 omitted
  events with its entire event-item budget already held, followed by a delivered
  response; selected-event exhaustion still leaves the following reply unread.
  CDP release units pass 73/73 (`cdp-units-guard.log`).
- Seven composition classifier cases preserve decoder/route semantics.
  Aggregate units pass 254, with 15 live cases ignored; resolution/evaluation
  leaves pass 37/41, with evaluation's three live cases ignored in this run.
- The bounded host probe adds a separate omission counter, retaining no event
  payloads. Its six offline cases pass. The first live Rust-only attachment
  passes in 381 ms: 61 commands/replies, no pending command or diagnostic overflow,
  1,440 omitted events and 32 retained context-created events. Explicit session
  close returns every recorded host metric to default (`attach-live.log`).
- Strict release Clippy, workspace release check, 5/7/15 leaf closures, all 30
  umbrella profiles, Node offline 9, shared harness 30, both type checks and the
  unchanged 221-package Node boundary pass. Initial test-helper failures remain
  recorded: a fake writer retained a queued wire lease until explicitly dropped;
  private reader test input now uses an infallible stream rather than a large
  Tungstenite error type. Neither initial failure is counted as a pass.

The final production addon SHA-256 is
`ab1a0857601008f56b83423bd53d8c950d158ef41d8284fde0b1039f566f7d9a`.
All five Node lanes ran serially on this artifact, with unchanged tests/fixtures,
no retries or substituted green reports:

| Gate | Result | Duration |
| --- | --- | --- |
| Locator | 32/32 | 68.600 s |
| Evaluation | 38/38 | 63.597 s |
| Lifecycle | Incomplete: 39 reported passes, then supervisor SIGKILL | 180 s deadline |
| Actions | 17/26; eight attachment failures plus NODE-022 | 83.502 s |
| Shared, all 57 cases | 33 matches / 11 unsupported / 2 oracle errors / 11 setup errors | 188.233 s |

Lifecycle's last reported case is number 39, the committed-document load wait
with independent reads. Its six remaining outcomes were not observed; zero
reported assertion failures does not make a watchdog-killed lane pass. NODE-014
and NODE-024 did not reproduce in their reported cases, but are not closed.

Actions fail candidate attachment in case 9 and oracle attachment in cases
10–13 and 17–19, all at the unchanged five-second deadline. NODE-022 again
observes Shift released by Rust and retained by Playwright after timed-out
delayed press. This checkpoint does not change modifier cleanup policy.

Shared ran `06:08:14.493Z`–`06:11:22.726Z`; its separate ten-minute supervisor
exits 1 with no signal, timeout, overflow or native failure. Value-mismatch,
candidate-error and cleanup-error counts are zero. Seven setup cases include
candidate topology timeouts, eight include oracle timeouts (four include both).
The two oracle errors are history back/forward. Eleven unsupported cases are
encountered operations, not new coverage; setup failures can prevent an
unsupported call from being reached. No event-item connection-close failure is
reported in this run, but the remaining timeouts are not attributed to a cause.
Changing activity on the shared browser prevents performance/RSS attribution.

After every runner became terminal, Pwright closed only preflight target
`B84A7805D3038B2D9D8368B031D55E9B`; discovery verified it absent and preserved
the five unrelated Pages then present (two XLQY, three Gemini). Chrome 152
retained browser identity `33b77c01-a5a4-42e4-b93b-ab822942d338`.
`cleanup.log` records the close. No browser restart, new commit or push occurred;
the local foundation commit remains `f76e3f2` and the broad goal stays active.

Next: distinguish remaining attachment/lifecycle stalls with bounded evidence,
and finish production prepared-owner runtime lifecycle, authoritative native
retirement, deferred Node source/argument conversion and Worker cleanup.
Public `Locator.evaluate` remains unsupported until that integration and paired
public behavior gates pass. The new filter is not permission to bypass them.

### Host-owned Locator evaluation checkpoint (2026-09-07)

`HostLocator::evaluate_with` now composes the production paired-world accessor,
strict waiter, selection/adoption and single-use evaluation in one controlled
Frame command. A Rust caller supplies a bounded, delayed conversion continuation;
the host invokes it only after preparation. Native ownership stays inside the
command through conversion, evaluation and release. This replaces the earlier
escaped prepared-owner/two-command plan: no JS token registry or second runtime
is needed for native ownership. The public Node binding is not yet exposed.

One resolution deadline covers the real queue and preparation, then ends before
conversion. Target/transport cancellation stays active throughout the command.
The original callback is never retried; missing/strict failure never invokes it.
Conversion errors await explicit native group release and preserve the primary
typed error unless cleanup also fails. Native CDP command limits still apply.
See [the ownership/resource contract](knowledge/pwright-locator-evaluation.md#host-owned-evaluation-continuation-2026-09-07).

The host selects eight active preparation/evaluation slots and sixteen pending
calls. Six extension scopes own the installed leaves and state. Per-call bytes
are formula-derived (2,490,368 on macOS arm64), with explicit callback/future size
guards before boxing. Cleanup reservations grow from 16 to 32 for the newly
selected two-per-evaluation native continuations; CDP/event limits are unchanged.
The normal host closure is now 224 (three added packages), while independent
resolution/evaluation closures remain 5/7/15 and all 30 umbrella profiles pass.

Evidence directory: `.work/node-owned-locator-evaluation.AbFw0k/`.

- `units-final.log`: 77 safe-host release cases pass, including nine new phase,
  admission/rollback, value-size and error cases; five live cases ignored.
- `host-live.log`: first four Chrome cases pass 4/4 in 1.55 s. The case originally
  labeled shutdown was renamed to correctly say caller cancellation; it aborts
  the caller while retaining native transport for cleanup.
- `host-live-lifecycle.log`: expanded run is 4/5 in 1.60 s, preserving the new
  failing Context-close gate. Passing cases prove original detached identity,
  main-world evaluation after 650 ms conversion despite a 500 ms resolution
  timeout, still-live runtime ownership at conversion, missing/strict ordering,
  explicit conversion/JS-error cleanup, eight-lane contention and caller drop.
- Strict release Clippy, workspace check, exact dependency proofs, Node offline
  9, shared harness 30, legacy CLI harness 67, both type checks and the safe-host
  boundary pass. `shared-units.log` is the legacy CLI harness;
  `shared-harness.log` is the actual paired harness, not interchangeable evidence.
- Earlier compile mistakes in new tests/API mapping and the initial too-small
  2 MiB call estimate remain in failed logs. The final estimate derives from
  actual value types/limits and keeps its executable bound test.

#### NODE-026: prepared element cleanup after native Context closure (initial failure)

The real test holds a prepared element in a pending conversion, then closes its
owned Context. The call returns TargetClosed, but host shutdown returns
`native command cancellation cleanup failed`. The failure remains nonzero;
no error is suppressed or classified as successful native destruction.

Code inspection confirms `ChromiumBrowserContextCloseCommandService::close`
awaits native disposal before committing runtime/route disposal. The temporary
group's session can disappear before the cancelled evaluation's group release.
The new command cancellation handler acknowledges continuation transfer, not
physical reclamation. Next work needs an authoritative native-retirement proof
or correctly ordered, per-operation bounded cleanup; a stale route or matching
error string is insufficient. This is separate from NODE-022/013/016/025 and
does not require changing public timeout or modifier policies.

The production host/profile change required an addon rebuild. SHA-256:
`8cc20b9e9999d85857b6c8401aab4fa9977fdf93d5d9dc9477c3c38200f6aad0`.
All five Node lanes ran serially, once, with unchanged public tests/fixtures:

| Gate | Result | Duration |
| --- | --- | --- |
| Locator | 32/32 | 35.053 s |
| Evaluation | 38/38 | 28.215 s |
| Lifecycle | 45/45 | 85.715 s |
| Actions | 25/26, NODE-022 | 25.808 s |
| Shared, all 57 cases | 41 matches / 14 unsupported / 2 oracle errors | 62.025 s |

Shared runs `06:39:30.214Z`–`06:40:32.239Z`, with zero value/candidate/setup/
cleanup errors. Supervisor exits 1, no signal, timeout, overflow or native failure.
History's two oracle errors and unsupported public APIs remain; no label match
is added because Node Locator.evaluate is still unavailable. NODE-022 remains
Shift release versus retention. Earlier attachment/window/navigation failures
did not reproduce here, but a later clean run does not establish their repair
or controlled performance/RSS improvement. The failing new Rust-only lifecycle
case is not exercised by today's Node public surface, so 45/45 does not cover it.

After all runners finished, only the owned preflight target
`4AC48869F015AD9222BD6D5021B23CA0` remained in discovery. Pwright closed that
exact target; discovery then showed no Page targets, while Chrome 152 retained
browser identity `33b77c01-a5a4-42e4-b93b-ab822942d338`. No browser restart or
unrelated-page closure was performed. No new commit or push; local foundation
HEAD remains `f76e3f2`. The full compatibility goal stays active.

## Native-destruction receipt checkpoint (2026-09-07)

The reproduced NODE-026 Context-close failure is fixed by an explicitly selected
`ChromiumFrameWorldAccessProfile::with_native_retirement()`, not by classifying
CDP error text as successful cleanup. Each paired snapshot can issue a monotonic
receipt for its exact connection epoch, Context, session and two unique execution worlds. The
existing topology driver marks exact world destruction or session-world clearing;
the Context-close service marks a successful native disposal acknowledgement
before publishing runtime cancellation. Lookup/registration and destruction /
route replacement share the same route-table transaction lock.

`close_route_with_outcome` distinguishes real native success from the existing
idempotent missing-Context error path; the latter grants no destruction proof.
The independent resolver consumes an optional `ObjectGroupDestructionProof`.
It skips group release only if both potentially owning worlds are proven dead,
or discharges an in-flight release failure when that same proof has arrived.
Without proof, release failures and abandoned native ownership still fail closed.
Detach, crash, route replacement, numeric world IDs, wrong sessions and matching
error strings are not substitutes. This is a scoped cleanup fix, not proof that
every remote disconnect/OOPIF/late-event race is solved.

The registry is opt-in and weak, limited to snapshot capacity, with no task,
subscription or historical tombstone collection. Its installed state also pays
for expired Weak allocation metadata until pruning. Receipt clones retain the
original 8 KiB snapshot permit, so snapshot drop cannot prematurely return the
count/byte allowance. Default profiles install no registry; dependency closures
remain unchanged. See [resource details](knowledge/pwright-locator-evaluation.md#optional-native-destruction-receipts-2026-09-07).

Evidence: `.work/node-native-retirement.bA2K7k/`.

- `units-final.log`: 441 release tests pass across six affected crates (6 Context
  lifecycle, 19 raw resolution, 261 Chromium composition, 41 evaluation
  composition, 37 resolution composition, 77 host); 23 explicit live tests ignored.
  Twelve new cases cover proof provenance, both worlds, wrong IDs/session,
  replacement/detach, retained budget, cycle-free drop and in-flight release.
- `host-live.log`: 5/5, 3.75 s. `host-live-repeat.log`: 5/5, 1.61 s.
  `context-repeat.log`: the original failing Context-close case also passes alone.
  Both TargetClosed and clean native shutdown are asserted, not just local counts.
- Strict release Clippy passes. Existing Node offline tests, type checks and
  safe-host boundary are rerun; complete rebuilt Node lane results follow below.

Rebuilt addon SHA-256:
`2cdd97b25da6ff5b182df283a2826536326c7286756a9e2585f82fe00da4a09e`.
Node's public Locator.evaluate API is not yet enabled. The next implementation
step is admitted Node-thread source/argument conversion after Rust preparation,
with deterministic exception settlement and Worker/environment teardown tests.
No new commit or push; the broader goal remains active.

The first receipt-enabled addon above completed its unchanged public lanes:
Locator 32/32 (31.619 s), evaluation 38/38 (29.665 s), lifecycle 45/45
(176.065 s), actions 25/26 with NODE-022 (43.374 s), and shared 41 matches /
14 unsupported / 2 oracle errors (97.457 s). Shared ran all 57 cases without
value/candidate/setup/cleanup errors; supervisor status 1, no timeout, signal,
overflow or native failure. The lifecycle run approached its 180 s watchdog;
this is not a performance or RSS improvement claim.

Final review added a connection-epoch guard: the receipt identity and paired
snapshot validity both include the published epoch. A close acknowledgement
uses its prepared candidate's epoch, so it cannot mark newly reused identities
after reconnect. The adversarial test deliberately reuses the same Context,
Frame, session, numeric and unique world IDs. This adds no dependency and the
same formula automatically accounts for the larger receipt. The first addon
hash/results above predate this guard; final guard-enabled evidence follows.

The guard-enabled addon is
`930469adb97228a9d375c1d35736f8388811dcd619c2b65faac809f626a7a4a3`.
`units-epoch.log` passes 442 release tests, with 262 Chromium composition cases
and thirteen new tests in this checkpoint. Strict Clippy, workspace all-target
check and the unchanged 224-package host boundary pass again (`*-epoch.log`).
The host gate is **4/5 in 8.94 s**, preserved in `host-live-epoch.log`: the queue
case fails at fixture creation with `ConnectTimeout { timeout_ms: 5000, phase:
TopologyInitialization }`, before its concurrency assertions. The other four,
including NODE-026, pass. `queue-epoch-diagnostic.log` separately runs that one
case successfully; it does not replace the failed complete gate or prove the
existing NODE-013 attachment issue fixed. No timeout or test expectation changes.

The final public lanes remain non-green: Locator 29/32 (73.667 s), evaluation
17/38 (149.198 s), and lifecycle is interrupted by its 180 s watchdog after
11 passes / 5 failures were reported (16 of 45 completed). The last completed
lifecycle case was ordinary Page.close after external closure/disconnection;
no result is claimed for the remaining cases. Locator and evaluation failures
occur during attachment/topology initialization, including both upstream
Playwright and candidate failures; two evaluation setup errors report native
response timeout rather than the outer 5 s deadline. This does not establish
causality or excuse the failed gates. CDP HTTP discovery still returns 200 in
under a millisecond during the failed run; this alone says nothing about its
session initialization progress. No browser restart, timeout increase, fixture
change or skipped test was used. Final shared/actions results follow below.

| Final guard-enabled gate | Result | Duration |
| --- | --- | --- |
| Locator | 29/32, three attachment failures | 73.667 s |
| Evaluation | 17/38, initialization failures | 149.198 s |
| Lifecycle | 11 pass / 5 fail reported; remaining 29 not completed | 180 s watchdog, SIGKILL |
| Actions | 25/26, unchanged NODE-022 | 44.845 s |
| Shared | 41 matches / 14 unsupported / 2 oracle errors; all 57 pairs | 106.414 s |

Shared ran `07:27:36.194Z`–`07:29:22.608Z` with zero value/candidate/setup/
cleanup errors. Supervisor status 1, no timeout, signal, overflow or native
failure. Its passing subset does not erase earlier package/host failures.
Final offline Node 9, shared harness 30, legacy CLI harness 67, both type checks,
strict release Clippy, workspace all-target check, 5/7/15 and 30 umbrella proofs
pass. Native SHA was rechecked after all runs; it remains `930469ad...a7a4a3`.

All runners are terminal. Pwright closed owned preflight target
`7DD836C1FB7149FD8729225D3A52430B` and the lifecycle watchdog's unique fixture
`0FCFAF3C026DF81C97A1D036A05F49EC`. HTTP discovery confirms both absent; two
unrelated game Pages remain. Chrome 152 keeps browser UUID
`33b77c01-a5a4-42e4-b93b-ab822942d338`; it was not restarted. Cleanup initially
passed native target IDs to legacy `tab-close`, which silently did nothing:
that command expects the task-state tab alias. Rebinding the owned task state
to the exact verified native target and closing its alias succeeded. Do not
treat the legacy CLI success message alone as target closure evidence.

Next: instrument the unresolved attachment/world-retention boundary without
resetting the shared browser, then complete admitted Node-thread Locator
conversion, settlement and Worker teardown. The scoped NODE-026 fix is retained;
no overall compatibility, RSS improvement, new public method, commit or push is
claimed. Foundation HEAD remains `f76e3f2` and the full goal stays active.

## NODE-013 three-way registration checkpoint (2026-09-07)

Evidence directory: `.work/node-attach-phase.j2k3aV/`. Before changing code,
the exact safe Rust host completed 28/28 traced commands in 32 ms. With an
upstream Playwright 1.58.2 connection alive, Node main attachment took 2,209.7 ms,
Worker attachment 2,673.4 ms and the native host 3,029 ms. All native replies
arrived, pending records were empty and the bounded recorder did not overflow.
Target-local Page enable, script registration, world creation, recursive attach
and resume each incurred latency; this is not a single unanswered command or
proof of an FFI callback deadlock. Node's sampled event-loop utilization was
1.50%; two Chromium renderers each consumed about 1.5 CPU seconds during the
2.2-second sample. Renderer load and oracle attachment correlate with latency,
but no CPU/focus/world-retention root cause is established. Existing game Pages
were not stopped, reset or closed.

The pinned oracle's local `playwright-core@1.58.2/lib/server/chromium/crPage.js`
lines 368–406 issue Frame Tree, Runtime enable and empty world registration
independently. The Rust topology production profile now offers
`with_parallel_utility_world_registration()` and the Node host selects it.
This fixed three-future join is caller-polled; all replies precede actual world
creation. Page enable/Context settings remain before it, recursive attachment,
resume and ready publication after it. Default serial and paired-only profiles
keep their previous behavior. There is no task, cross-session queue, new
dependency, timeout increase, retry or Node automation implementation.

The option requires three pending CDP slots and a third response expansion:
16 MiB additional declared ceiling over the earlier paired Node profile, not
an allocation or RSS improvement claim. Selection is idempotent. Without the
utility-world capability, no script/world command is sent. An invalid frame
tree may have an empty script registered by the opt-in join but cannot create
worlds or publish a discovered Page. Actual-world creation failures retain the
existing authoritative frame-disappearance check and fail-closed behavior.

Ten new deterministic tests cover three-way polling/cancellation, each pending
and error branch, success, capacity/accounting, and driver discovery/maintenance
ordering, failure and absent-capability behavior. `units-final.log` passes
272 composition + 77 host tests; 20 explicitly browser-dependent tests are
ignored in that offline invocation. Strict release Clippy, 9 Node offline
tests, TypeScript checking and the unchanged 224-package dependency boundary
pass. The rebuilt addon SHA-256 is
`8c9d903cfa07e25b30c4a3cb0e2904df5d56b8c9151a6886af99a8807cd9c39f`.

The first post-change overlap diagnostic is retained as `overlap-triple.log`:
Node main/Worker attachments succeeded (overlapped main 1,561.6 ms, Worker
1,057.4 ms), but its Rust child timed out waiting for the Cargo build-directory
lock held by parallel Clippy. It produced no native command trace and the
diagnostic exited nonzero. It is not counted as a successful complete probe or
as a native attachment failure. A separately recorded probe after build-lock
release and the unchanged full browser regression results follow below.

The isolated diagnostic completes successfully in
`overlap-triple-isolated.log`: standalone Node 35.4 ms, overlapped main
2,050.3 ms, Worker 2,068.8 ms, and native host 2,152 ms. The native trace
contains 28/28 replies, empty pending state and no overflow. Each session's
Frame Tree, Runtime enable and script registration now share its preparation
timestamp (9 ms, 138 ms and 1,279 ms respectively); this proves actual wire
overlap, not just an async helper in tests. These uncontrolled before/after
samples do not establish a stable speedup, the cause of renderer latency,
an RSS reduction or that the intermittent five-second timeout is fixed.

| Fresh gate on the three-way addon | Result | Duration |
| --- | --- | --- |
| Native host Locator evaluation/lifecycle | 5/5 | 1.80 s |
| Node lifecycle | 45/45 | 108.070 s |
| Node evaluation | 38/38 | 29.644 s |
| Node Locator | 32/32 | 34.631 s |

Each Node lane ran the full existing test list serially with the unchanged
five-second connection limit and 180-second process watchdog. No retries or
skips replaced failures. All runners exited normally; fixture cleanup passed.
Formatting and `git diff --check` pass. Actions/shared lanes were not rerun in
this checkpoint, so their earlier NODE-022/unsupported/oracle discrepancies
remain recorded rather than inferred green. Only the owned preflight blank
target was closed, with absence verified by a fresh target listing. Both game
Pages were present immediately before cleanup; the final inventory contains
only the male-game target. This task issued no game-target close command, and
the female target's disappearance has no established cause in this checkpoint.
Chrome's browser UUID is unchanged; it was not restarted.

Next is the admitted Node-thread continuation for public Locator.evaluate:
retain source/argument references within explicit limits, convert only after
Rust element preparation, and prove exception/Worker teardown releases both
Node references and native ownership. NODE-013 remains open; these green
subset regressions do not complete VS3 or the broader compatibility goal.
No new commit or push.

## Public Locator.evaluate FFI checkpoint (2026-09-07)

The public Node Locator now calls the existing Rust `HostLocator::evaluate_with`.
Strict selection, main-world adoption, the preparation-only timeout, execution
and native group cleanup remain in one controlled Rust Frame command. Node
retains only the conversion callback and handles JS value/exception conversion;
it adds no CDP implementation, browser algorithm, launcher or driver process.
The original function/argument are captured without coercion at API entry.
Rust requests conversion only after it owns the prepared element. A function's
source and argument getters therefore observe mutations made while selection
was pending, and missing/strict failures never invoke them.

`locator_conversion.rs` installs one environment-local, unreferenced Node-API
thread-safe callback with queue capacity 32. It carries only monotonically
allocated integer IDs, not JS values or native element tokens. The Node-thread
registry has 32 entries and a one-shot Rust reply per admitted call. Entry
allocation requires the existing 8 MiB evaluation FFI reservation first; the
environment-wide 64 MiB and 32-command ceilings remain unchanged. An additional
event bridge is selected by the broad Node package, not independent Rust leaves.
Raw callbacks/exceptions are bounded in root count, **not** the size of arbitrary
caller-owned JS object graphs. This is not a bound on V8 heap or RSS. Converted
source/tree/binary data retain their existing checked limits.

Binary inputs still use structural conversion followed by later byte copying:
the registry schedules a second Node turn only when binary references exist.
No registry borrow spans getters, source coercion or that scheduler. Cancellation
before callback delivery skips conversion when the Rust reply receiver is gone.
The Node settlement guard removes entries even on an early FFI/publication error.
Environment teardown empties Node roots and reply senders before joining FFI
and native cleanup; the final Rust notification owner releases its callback.
Ordinary conversion exceptions remain on the Node thread until native release
finishes. A cleanup or liveness error overrides them; only the exact untouched
conversion marker can restore the saved JS error.

The pinned oracle mutates conversion exceptions at its API boundary. Ordinary
Error identity is retained; thrown primitives become JavaScript TypeErrors when
they cannot carry the message property. Initial candidate behavior preserved
primitive rejection values instead, causing four differences. The Locator
facade now follows that error-decoration behavior without synthesizing error
messages. Expanded tests compare the resulting type and message as well as
identity. Existing Page/Frame error mapping is unchanged.

Evidence directory: `.work/node-locator-ffi.DTyrg0/`. Release addon SHA-256:
`2107bdcd03272bd7a2e4fe97218a44b7e0d2e095754e9d077de3df58284b0c66`.

- The first public lane passes 13/13 (`locator-evaluation.log`), covering main
  world/element identity, hidden elements, delayed source/getter conversion,
  missing/strict failures, timeout scope, Error identity, binary microtasks,
  rich values, expressions, browser exceptions and bounded raw-reference pressure.
- The expanded lane initially has 18/22 assertions plus a native shutdown
  failure (`locator-evaluation-expanded.log`); the four assertion failures are
  the primitive-exception discrepancy described above.
- After error alignment, all **22 assertions pass**, but the **gate still fails**
  (`locator-evaluation-errors-aligned.log`, 17.767 s). Worker termination during
  a never-resolving browser evaluation emits `Pwright environment cleanup
  failed: native command cancellation cleanup failed`. The existing supervisor
  correctly rejects that run despite child TAP exit 0. It is not counted green.
- Strict release Clippy, package and shared TypeScript checking, 9 offline Node
  tests, and the unchanged 224-package safe-host boundary pass. Existing package
  regression and shared-corpus results follow separately; earlier hashes are
  not reused as evidence for this new addon.

### NODE-027: running Locator cancellation cleanup is not proven

The new Worker tests distinguish raw references, binary staging, an executing
browser Promise, and a queued native result. Each test verifies removal of its
exact owned native Context/target and leaves the external Chrome process alive.
Raw, binary and queued teardown do not emit cleanup failure. The running case
does: its Context eventually disappears, but the cleanup report is non-clean.
Code inspection shows the transferred `ElementData::evaluate` continuation
awaiting the same browser response before `finish_source` releases the group;
the never-resolving Promise encounters the native cleanup deadline. Host
shutdown currently drains that cleanup before dropping native routes/connection.

Next design/implementation must cancel or retire this outstanding evaluation
while proving temporary-group release, without waiting indefinitely for user
JavaScript and without treating timeout/error text as destruction evidence.
An owned-Context close acknowledgement may provide evidence, but borrowed
default/external Contexts must not be closed to make this test pass. Review
late-result ownership and independent-leaf cancellation before changing close
ordering. Keep the failing Worker lane and supervisor diagnostic gate intact.
No timeout increase, skipped Worker case or green-by-TAP-only claim is allowed.

The shared adapter can now perform its original getByLabel tag-name observation
through public `Locator.evaluate`; it does not substitute inputValue or direct
Page.evaluate. The shared cleanup assertion now checks this registry is empty.
The full goal remains active; public evaluateAll/Handles, other remaining APIs,
NODE-027 and the previously recorded discrepancies are not complete. No new
commit or push.

Fresh regressions on this addon pass evaluation 38/38 (19.362 s), existing
Locator 32/32 (28.012 s), and lifecycle 45/45 (18.984 s), each run serially with
its complete existing test list. These timings reflect the current shared
Chrome state, not a controlled performance comparison. Actions were not rerun.

The first shared attempt (`shared/`) stops before executing any case because
the `/pwright-shared` serve-lib route is absent and fixture validation returns
404. Its nonzero supervisor result is preserved. The same exact fixtures were
re-registered at that route with a two-hour lifetime on the Tailscale IP and
the full MagicDNS URL was verified as 200 before the complete fresh run.
`shared-registered/` then executes all 57 cases: **43 matches / 12 unsupported /
2 oracle errors**, with zero value, candidate, setup or cleanup errors. Both
previously unsupported label cases now match through the real public method.
Supervisor status is 1 (remaining unsupported/oracle cases), with no signal,
timeout, overflow or native failure. Fixture/catalog/golden expectations and
deadlines are unchanged; this shared result does not override NODE-027.

All runners are terminal. Only the task-owned blank preflight target
`1514F5C1EAA728338B31ECD36E0F18ED` was closed via its saved local alias; a final
Pwright listing confirms absence. No game target or external Context was
closed by this task, and the Chrome browser UUID remains unchanged.

### NODE-027 native cancellation boundary evidence (2026-09-07)

An early-group-release / dropped-by-value-receiver implementation was considered
and **not selected**. The new release-mode Chrome counterexample in
`crates/pwright-composition-chromium-locator-evaluation/src/live/cancellation.rs`
establishes native dispatch with a same-session FIFO marker and a page-owned
Promise resolver, without sleeps:

1. Acknowledged group release invalidates the original element wrapper.
2. The already-running `awaitPromise` request remains pending.
3. Resuming the user Promise with a native exception produces a usable late
   `exceptionDetails.exception.objectId` despite `returnByValue: true`.
4. That late wrapper requires another group release. Execution occurs once.

This is a raw-CDP counterexample, not an assertion that every normal Pwright
structured-error envelope allocates a native exception. It invalidates the
proposed general proof that a by-value request cannot allocate after release.
The current wrapper also reads user-controlled error properties, so its normal
envelope path must not be assumed infallible without separate verification.

The companion real-browser test explicitly detaches only an owned debugger
session. A separate observer still accesses the same Page and resumes its
user Promise, whose counter advances exactly once. Thus session detachment is
not Page destruction or cancellation of user JavaScript. Both tests dispose
only their own fixture Contexts after assertions and check browser liveness.

Upstream evidence: [CDP Runtime](https://chromedevtools.github.io/devtools-protocol/v8/Runtime/)
distinguishes group release from Promise waiting. V8's
[InjectedScript implementation](https://raw.githubusercontent.com/v8/v8/main/src/inspector/injected-script.cc)
tracks evaluation callbacks separately and can wrap exceptions as remote
objects. The [session implementation](https://raw.githubusercontent.com/v8/v8/main/src/inspector/v8-inspector-session-impl.cc)
also weakens matching Promise handlers on group release; this is not settlement
or an immediate-GC guarantee. Session teardown discards injected scripts and
weakens session handlers. These are current upstream `main` sources, not a
pinned source audit of Chrome 152; the tests above are live Chrome 152 evidence.

Next implementation candidate: a narrow, explicit **owned-session retirement
receipt** for host teardown, separate from existing world/Context-destruction
proofs. Before selecting it, prove exact session and connection epoch identity,
command admission shutdown before detach, ordering against in-flight native
commands, and acknowledgement before accounting retirement. Generic detach
events, crashes, missing-session error strings and disconnect observations stay
insufficient. Detaching a shared active session for an ordinary single-call
cancellation is not authorized by this design. Borrowed Pages/Contexts must
remain untouched; user JavaScript and its reachable heap are not claimed to
stop or be reclaimed. The standalone leaf retains its current fail-closed
continuation contract pending that implementation.

A new deterministic regression checks that cancelling a running evaluation
retains its leases and emits no group release while the native gate remains
closed; opening the gate drains the same continuation and returns to baseline.
Evidence directory: `.work/node027-native-proof.HY4spM/`. The leaf's release
tests pass 42/42; both new explicit live tests pass 2/2. This is boundary evidence,
not a NODE-027 fix: the Node addon, Worker lane and shared comparison were not
rerun, and their previous non-green statuses remain unchanged. No production
code, dependency, public API, timeout or supervisor gate was changed.

Final verification: release suite 42 passed / 5 explicit Chrome tests ignored
by the offline command; the two new Chrome tests were separately executed and
passed. Strict all-target Clippy, formatting and diff checks pass. All runners
are terminal. Only the task-owned preflight target
`7A6A4A08A9717396ECD5FB2794EAF271` was closed and its absence verified; the
independent XLQY Page in the final target listing was preserved. No commit or
push was made in this follow-up.

### NODE-027 receipt-driven Rust evaluation implementation (2026-09-07)

The independent Locator evaluation composition now exports the opt-in
`NativeEvaluationRetirement` contract and
`MainWorldElement::evaluate_with_retirement`. A caller-budgeted, single-consumer
receipt is polled against the exact shared CDP client and unique main-world ID.
It must prove fenced dispatch and acknowledged retirement of the native
evaluation session/callbacks, not ordinary user cancellation. A pending receipt
must register the current task waker without lost wakeups. The leaf creates no
task, channel, timer, browser lifecycle or additional dependency for this path.

The receipt is checked before the native future's first poll and before later
polls. Acknowledged retirement returns the distinct `NativeRetired` outcome and
drops only the outstanding native wait; the group owner still runs its separate
cleanup. Dropping the public future transfers this same race, receipt and owner
to the existing pre-reserved cleanup continuation. Unpolled cancellation never
polls the receipt or dispatches user code. Once group release has begun, the
receipt cannot replace, interrupt or duplicate that release. The ordinary
`evaluate` entry point retains its existing completion-on-cancellation behavior.

`ObjectGroupDestructionProof::all_group_objects_retired` is a new defaulted
method which delegates to the existing world-destruction predicate. A composing
owner may override it with an exact, acknowledged inspector-session retirement
proof, without falsely claiming the live worlds died. Both release checks and
the raw-owner drop guard use this predicate. Generic detach/error observations
still do not qualify. Evaluation retirement alone cannot hide unproven group
cleanup: a release failure remains a compound cleanup error and poisons further
admission unless the independently supplied group proof discharges it.

The new real-Chrome test in `src/live/retirement.rs` uses the actual resolver,
native DOM adoption, evaluation runtime and cleanup queue. Its fixture owns the
Context, while a separately owned evaluation session borrows the Page. After
causal native-dispatch confirmation, it cancels the Rust caller, fences further
session commands structurally, awaits exact `Target.detachFromTarget` success,
and publishes the two receipt projections. Cleanup finishes with the user
Promise still unresolved, no borrowed Page/Context closure, a healthy service,
zero cleanup reservations and baseline leases. An independent observer then
resumes the same Promise; its counter advances from one to two. This validates
the mechanism, not the production Node host's still-unimplemented session
inventory/admission/epoch coordination or physical V8 heap reclamation.

Evidence: `.work/node027-retirement-leaf.VvZ61y/`. Release tests pass 21 raw
resolution + 51 evaluation/preparation tests; all six explicit live tests pass
on Chrome 152. Strict all-target Clippy includes both leaves and the Node addon;
the exact 15-package evaluation dependency proof passes unchanged. The concrete
optional-receipt composition future fits the existing 8192-byte inline owner
allowance. Dynamic receipt allocations remain caller-owned and caller-budgeted,
not included in that inline assertion. No Node package build, Worker lane or
shared differential rerun is claimed.

Next required integration: give the host a bounded inventory of its own exact
native sessions, seal registration/dispatch before detach, retain receipts across
route retirement, and publish acknowledgements with matching connection epochs
before its cleanup executor deadline. Handle partial detach failures without
promoting them to receipts, and preserve borrowed Pages/Contexts. Only after
this is wired into actual host shutdown should the complete Worker lane decide
whether NODE-027 is fixed. The full goal and NODE-027 remain open; no commit/push.

### NODE-027 host integration and Worker verification (2026-09-07)

The Node host now selects the receipt-driven evaluation path. Successful paired
preparation supplies one budgeted notification projection; its exact CDP client
identity and unique main-world ID must match when polled. The group proof reads
object retirement separately from world destruction. Default non-Node paired
sources supply no receipt and preserve existing evaluation behavior.

Shutdown stops the topology loop and seals command admission under the same
session-state mutex used by `admit_operation` for its connected check and tracker
token publication. It then waits for admitted host calls to leave. This closes
the race where tracker close/wait could observe zero just before a previously
checked admission published a token. Only then does the host seal the existing
bounded weak native-receipt inventory and detach sessions retained by those
receipts through its own composition's browser connection. This is not an
inventory of foreign sessions or an instruction to close their Pages/Contexts.

The sealed inventory cannot grow or be retried. It retains one existing snapshot
permit across each native await, rather than allocating another session list.
Receipts sharing the same epoch/session are attempted once; failures remain
failures while independent sessions can still retire. Old-epoch receipts are
rejected without native I/O. Only successful exact detach acknowledgements mark
object retirement and wake the evaluation/cleanup task. Generic detach events,
missing-session errors and transport loss remain insufficient evidence. Dropped
or failed retirement does not reopen registration or fabricate an acknowledgement.

An additional `poll_release_ready` group-proof fence handles a native response
or release failure arriving before the detach acknowledgement. While retirement
is in flight, cleanup retains the original owner/error instead of deciding the
outcome early. Both successful and failed detach wake this fence; a failed
detach still requires ordinary group release and cannot hide its failure. All
wakes occur outside registry and receipt locks. One notification consumer moves
from evaluation to its group cleanup; ownership-only receipt clones do not poll.

Resource accounting: the registry's installed-state allowance is derived from
its actual receipt/weak-entry layout, including waiter and retirement flags.
The snapshot permit follows temporary inventory ownership. Node call admission
additionally charges `size_of::<Retirement>() + 1024 + 64` bytes for its boxed
projection and bounded unique-ID copy. No extra task, timer, channel, native
session creation or dependency is introduced. The FFI's 8 MiB per evaluation /
64 MiB environment budget is unchanged. User JS and V8 heap reclamation are not
claimed to stop merely because inspector ownership was retired.

Final addon SHA-256:
`ec4858b9ab298c24854817571c57321b893490f01dff6b589f060d320f742961`.
Evidence directory: `.work/node027-host-retirement.uhX3zj/`.

- `locator-evaluation-budgeted.log`: **23/23**, supervisor exit 0, 29.854 s,
  including every existing Worker phase and a new borrowed-default-Context Page
  case. The observer creates the exact fixture Page before Worker attachment.
  After terminating the Worker it verifies the Page and its unresolved Promise
  survive, then resumes the same Promise and observes one continuation. No
  native cleanup diagnostic is present. NODE-027's reproduced failure is fixed.
- `unit-sealed.log`: 23 raw resolution + 277 Chromium composition + 51 independent
  evaluation/preparation + 77 host tests pass. The final explicit call-budget
  change is additionally covered by `host-budgeted.log` (77/77). Tests cover
  acknowledgement gating, release-error races, failed detach without retry,
  duplicate session receipts, independent partial success, epoch mismatch,
  permanent registration seal, default behavior and lease retention.
- Strict all-target Clippy passes for the affected leaves/composition/addon;
  `clippy-budgeted.log` covers the final host/addon budget change. The exact
  15-package leaf closure and 224-package safe-host boundary pass. Types and
  all 9 Node offline tests pass. Formatting and diff checks pass.
- Earlier integration artifacts pass evaluation 38/38 and Locator 32/32. The
  first lifecycle invocation lacked the required dynamic-fixture bind variable
  and stopped before tests; its failure log is preserved. The configured full
  lifecycle attempt observes NODE-014 again (Rust outer window 1282x800 versus
  oracle 0x0), reports 38 passing cases / one failed case through case 39, then
  reaches the unchanged 180 s supervisor deadline and is killed. It is not a
  complete or passing lifecycle regression. No timing cause is claimed.
- `locator-evaluation-final.log`, on the intermediate admission-sealed artifact,
  is 22/23 because case 9 fails during the existing 5 s topology-attachment
  deadline (NODE-013), before its binary-copy exercise. Every Worker case passes
  and there is no native cleanup diagnostic. This failed full run remains valid
  evidence despite the later final-budgeted 23/23 pass.

The fixture route `/pwright-node-actions` had expired; its exact directory was
re-registered through serve-lib and the full MagicDNS URL verified as 200 before
testing. The killed lifecycle runner left one uniquely identified fixture Page
at that route's `index.html`, with title `__pwright_native_case__`. It was closed
through the public Pwright Node API after asserting that exact URL/title match;
no default Context or unrelated game Page was closed. No Chrome restart,
timeout increase, filtered replacement gate, fixture/golden change, commit or
push was used. Full Playwright compatibility and the overall goal remain open.

The final-budgeted shared run (`shared-budgeted/`) completes all 57 cases on the
same addon: **43 matches / 12 unsupported / 2 oracle errors**, with zero value,
candidate, setup or cleanup errors. Its supervisor status is 1 solely with these
non-matching categories retained; no timeout, signal, overflow or native failure
is reported. Fixtures, golden observations and operation deadlines are unchanged.
The shared result is not full compatibility and does not erase the lifecycle
deadline/viewport failure or intermediate NODE-013 attachment timeout.

All runners are terminal. The exact preflight target
`DE5226C46A30758523F797B8D0A48EFD` was closed; a later target listing confirms
both it and the orphan fixture target are absent, with only unrelated game
Pages remaining. A close acknowledgement alone was not treated as absence.
Chrome retains browser UUID `33b77c01-a5a4-42e4-b93b-ab822942d338`.

### NODE-013 retained-world diagnostic (2026-09-07)

Added `packages/pwright/scripts/probe-retained-worlds.mjs`, an explicitly separate
diagnostic, not a parity test or production behavior change. After the standard
dedicated Chrome/Pwright preflight it creates one owned blank Page, holds a
pinned Playwright observer, and performs the fixed sequence candidate, oracle,
oracle, candidate, candidate, oracle. Every connection retains the five-second
public timeout. A failed connection/cleanup stops the sequence without retry.
Auxiliary observations/cleanup have the existing ten-second diagnostic deadline.
The probe closes only its owned fixture Page and its own connections; it does
not navigate or close preexisting Pages or reset Chrome. Browser-wide attachments
still initialize preexisting targets; this is not a no-impact or isolated browser
benchmark. The inventory itself observes only the owned main frame.

`probe-world-inventory.mjs` retains at most 512 context IDs, with names capped at
128 JavaScript code units before storage. It discards event payloads and origins,
retains names only for Pwright worlds, and reports category counts, bounded
Pwright names, the owned main-world ID, and a sticky overflow bit. Every sample
disables/re-enables only its observer Runtime domain to enumerate the current
contexts. An overflow or missing/duplicate main world fails the diagnostic.
Five offline tests cover ownership filtering/deduplication, oversized names,
invalid IDs, capacity and overflow persistence through reset.

Evidence: `.work/node013-world-audit.ejhq1v/`, unchanged native SHA-256
`ec4858b9ab298c24854817571c57321b893490f01dff6b589f060d320f742961`,
Chrome 152.0.7977.76 and Playwright 1.58.2. Both the initial diagnostic
(`worlds.log`) and the final identity-checked diagnostic (`worlds-final.log`)
exit zero. The final owned-page observations are:

| Sample | Pwright worlds | Playwright worlds | Main-world ID |
| --- | ---: | ---: | ---: |
| Baseline observer | 0 | 1 | 1 |
| Candidate disconnect 1 | 1 | 1 | 1 |
| Oracle disconnect 1 | 1 | 2 | 1 |
| Oracle disconnect 2 | 1 | 3 | 1 |
| Candidate disconnect 2 | 1 | 3 | 1 |
| Candidate disconnect 3 | 1 | 3 | 1 |
| Oracle disconnect 3 | 1 | 4 | 1 |
| Reload of owned document | 1 | 4 | 7 |

The candidate reuses `__pwright_utility_0000000000000005` within this final run;
the initial run used a different Page-derived name ending in `0003`. Do not
generalize same-name reuse to changing topology or all future connections.
All six final samples attach successfully: candidate approximately 735, 1446,
1539 ms; oracle 289, 232, 56 ms. This fixed sequence is not a causal performance
comparison. Owned-document reload changes the main-world identity but does not
reduce the observed utility-world counts. These are contexts, not RSS, retained
heap bytes, live init-script registrations or proof of an unbounded leak.

Source cross-check: installed Playwright 1.58.2 `crPage.js` derives its world name
from the Page GUID. Chromium's version-matched
[DOMWrapperWorld implementation](https://github.com/chromium/chromium/blob/152.0.7977.76/third_party/blink/renderer/platform/bindings/dom_wrapper_world.cc)
looks up inspector worlds by name. Its
[WindowProxyManager](https://github.com/chromium/chromium/blob/152.0.7977.76/third_party/blink/renderer/bindings/core/v8/window_proxy_manager.cc)
clears each proxy for navigation without erasing the isolated-world map, and
updates those proxies on document changes. This supports named-world retention
as an explanation for the observed counts, but does not measure memory or prove
that disconnected initialization scripts remain registered. No production world
naming or teardown contract was changed based on this observation.

The separate release Rust attachment probe (`attach.log`) subsequently passes
in 25 ms, with 21 commands/replies, no pending commands, no overflow, seven
delivered execution-context events and 285 pre-admission omissions. Its slowest
recorded command durations are nine milliseconds. It runs without the overlapping
oracle or owned fixture, so this is not a controlled comparison with Node timing.
NODE-013's intermittent deadline failure remains open.

Verification: final Node offline tests 14/14, TypeScript check, 224-package host
dependency boundary and `git diff --check` pass. Production Rust/addon code is
unchanged, so the previous full parity evidence is not relabeled as a new run.
The diagnostic fixture Pages and the exact task preflight are cleaned up; the
final CLI inventory contains only an unrelated game Page. Shared target activity
changed during the investigation and cannot be used for causal timing claims.
All processes are terminal; no commit or push. Next: test Page-ID/topology churn
separately and localize overlapping-session command latency before selecting a
production fix. Memory claims require the independent process/heap workload gate.

### Node Page screenshot binding and ownership checkpoint (2026-09-07)

The Node facade now exposes `page.screenshot()` through the existing optional
Rust `PageScreenshot` command. Supported arguments are PNG/JPEG type, JPEG
quality, viewport/full-page capture, clip, CSS/device scale and timeout. JS only
validates/marshals options; the safe Rust host dispatches the controlled command,
the existing CDP backend obtains geometry/captures/decodes, and the addon returns
a Node-owned Buffer. The shared adapter now calls this public method without
changing either original screenshot fixture or its observations.

This is a VS5 increment, not full screenshot compatibility or completion of the
goal. Filesystem `path`, animations, caret, masks, style and background omission
are not exposed. Combined `fullPage`/`clip` is explicitly unsupported under the
existing content 1.0 contract. Default caret hiding/font-readiness behavior is
not implemented and must be investigated with rendering-transaction fixtures.
Public Node type remains PNG/JPEG, not the backend's additional WebP format.

Explicit host composition and budgets:

- Install `ContentInstallationProfile` only in the Node compatibility recipe;
  default Rust/leaf consumers do not acquire Node, a queue or a changed policy.
- Two native captures maximum, one active capture per Page, 2 MiB decoded image
  ceiling. The 4 MiB CDP response limit is unchanged; it bounds encoded input.
  Content control frames allow 16 KiB plus one bounded binary attachment.
- A host-owned queue accepts at most eight active/waiting calls, with independent
  per-Page FIFO mutexes and a two-permit capture semaphore. Queue wait and native
  capture share one public timeout. Timeout zero has no local deadline; session
  shutdown still cancels it. No task, timer loop, browser process or Node driver
  is installed by the queue.
- Queue metadata reserves 4 KiB once and 4 KiB per admitted waiter, with a fixed
  eight-slot ceiling; production requirements include nine scopes/leases. The
  queue lives inside `LiveSession`, so closing the host releases idle state too.
- FFI admission reserves `2 * 2 MiB + 4096` per screenshot within the existing
  64 MiB environment allowance. The native result is copied to the host's bounded
  caller-owned output **before** releasing the Page queue. That releases the
  core `ScreenshotBytes` lease before admitting the next capture. At settlement,
  `BufferSlice::copy_from` copies the host bytes into Node-owned memory while
  the FFI ticket is still held. Each handoff has at most two image copies; no
  external buffer, Rust finalizer or native byte pointer escapes into JS. Direct
  safe-Rust callers own the bounded returned bytes; the FFI caller supplies the
  additional result-retention admission. User-held Buffers are not an RSS bound.

The first nine-case live lane (`live-first.log`) passes seven and finds two
real discrepancies, tracked here as NODE-028's screenshot integration failures:
an edge clip returns 300x300 instead of oracle 80x70, and concurrent same-Page
capture rejects its second call. `with_viewport_clip_intersection()` is now an
explicit CDP production-profile opt-in; only the Node profile selects it. The
default exact-clip behavior and content 1.0 validation remain unchanged.

The second ten-case attempt (`live-final.log`) fixes geometry but remains 9/10:
the first queued native result still owns 4,378 bytes when the next capture
reserves its 2 MiB per-Page maximum. The host-copy handoff above fixes this
ownership overlap without increasing the backend screenshot limit. Both failing
logs remain evidence; tests and deadlines were not relaxed to make them pass.

Final release addon SHA-256:
`e0dcbd2be9e895bb3a793a75b4b38656b911a7a672c5b2c20d229e683e747b92`.
Evidence directory: `.work/node028-screenshot.jpQlHX/`.
`live-handoff.log` passes all **10/10** in 10.657 seconds with a clean supervisor.
It compares decoded pixels and dimensions for PNG, default/explicit-quality JPEG,
full-page, CSS scale and fractional clips; verifies edge intersection, independent
concurrent results, mutable Buffer survival after Context/Browser close and
settlement during Page close. The fixture is a deterministic CSS-color document;
it does not prove text/font/animation fidelity or the missing options above.

Rust verification passes 88 safe-host tests (five unrelated live tests ignored)
and seven screenshot-backend tests. Queue tests cover finite admission, Page
isolation, cancelled waiter reclamation, wait-inclusive deadlines, shutdown with
timeout zero and metadata sizing. The result-handoff test observes the native
byte lease reach zero while the host output remains readable. Node offline tests
pass 20/20; TypeScript and strict Clippy pass. Clippy first exposed a missing
standalone `tokio/test-util` dev feature hidden by the combined test invocation;
it is now declared explicitly and isolated host tests pass. The content-only
dependency proof remains exactly 17 Pwright packages; safe-host normal dependency
closure remains 224 packages without Node-API/driver/browser-launch imports.
The full shared run on this artifact (`shared/report.json`) completes all 57
cases in 68.103 seconds, with **45 match / 10 unsupported / 2 oracle-error**
and no value/candidate/setup/cleanup errors. The two unchanged screenshot cases
move from unsupported to match. Its supervisor exits 1 because remaining
categories are non-passing, with no timeout/signal/overflow/native failure.

Final resource-scope audit assigns queue state its own `(120, 16)` extension
scope, avoiding the Locator resolution profile's `(120, 10)` scope. This does
not increase queue/image limits or change screenshot ordering. The resulting
final addon is
`acabe149c2927238ad77256d1913dd819efd81a9028deb70a62623f7fb698ef5`.
`unit-scoped.log` passes 88 host tests; `clippy-scoped.log`, formatting, dependency
boundary and diff checks pass. Shared harness unit tests pass 30/30 and its
TypeScript check passes. Current artifact evidence is deliberately separate:

| Gate | Result | Interpretation |
| --- | --- | --- |
| Screenshot (`live-scoped.log`) | 8/10, supervisor exit 1, 32.277 s | Cases 8/9 fail at candidate connect/topology initialization after 5 s, before concurrency or Buffer-survival exercise; not a verified green final-artifact gate |
| Locator evaluation (`locator-regression.log`, preceding handoff artifact) | 20/23, supervisor exit 1, 52.997 s | Case 14 candidate attachment and cases 15/16 oracle attachment time out; all five Worker cases pass. Log completion precedes replacement of the addon by 27 seconds |
| Full shared (`shared-scoped/report.json`) | Complete 57: **25 match / 9 unsupported / 2 oracle-error / 21 setup-error** | No value/candidate/cleanup error. Initial cases record both oracle and candidate attachment timeouts; later cases recover, including both screenshot matches |

The final shared run starts `09:47:26.764Z` and ends `09:51:36.443Z` (249.679 s).
Its supervisor exits 1 with no signal, watchdog timeout, overflow or native
failure. Its lower unsupported count does not mean another API was implemented:
setup failures prevent an unsupported operation from being reached. Neither the
earlier 10/10 screenshot result nor the earlier 45-match shared report replaces
these final-artifact failures. NODE-013 and the full compatibility goal remain
open; no deadline, fixture, original observation or failure classification was
weakened. Next screenshot gates are rendering transactions/options, independent
binary-pressure/Worker tests and a green final-artifact concurrency/ownership
comparison once the attachment failure is understood.

All runners are terminal. The task-owned preflight target
`F28A0A2CF9DB5B9F48FF2CF405E3C664` is absent from `final-tabs.log`; only unrelated
game targets remain. No preexisting Page was navigated/closed, Chrome was not
reset and no commit/push was made. The local foundation commit remains `f76e3f2`.

### NODE-013 target correlation and bounded pair experiment (2026-09-07)

Evidence: `.work/node013-session-latency.uJfjz8/`. `CdpSession::new` now emits a
TRACE-level opaque session/target binding. There is no production correlation
table, subscriber, task, payload capture or new dependency. The test-only
`attach_probe.rs` subscriber retains at most 64 bindings, caps each identity at
128 UTF-8 bytes before copying, deduplicates identical bindings and fails its
sticky overflow gate on conflicting identities. The report's additive `sessions`
field associates its existing bounded command history with targets. Ten offline
probe tests pass, including four new binding/budget checks.

The standalone safe Rust host completes in 81 ms without the pinned oracle and
3,758 ms while it remains attached. Both traces contain 35 commands/replies,
zero pending commands and no recording overflow. Under overlap, seven-command
native initialization intervals are:

| Target | Start / end from trace start | Duration |
| --- | --- | --- |
| Owned blank `9E4F…EA50` | 4 / 5 ms | 1 ms |
| Existing Page `A956…41E6` | 5 / 1,243 ms | 1,238 ms |
| Existing Page `E80F…7E19E` | 1,244 / 2,541 ms | 1,297 ms |
| Existing Page `EEF0…C3C0` | 2,542 / 3,756 ms | 1,214 ms |

The Node main-thread and Worker overlap observations are 4,448.592 and
4,991.675 ms; the main-thread event-loop utilization is about 0.0076. This
locates additive latency in successive target initialization, not in N-API
settlement. It does **not** establish why Chrome replies are delayed or prove
that cross-session concurrency fixes NODE-013.

Source inspection corrects the initially suspected intervention point:
`initialize_snapshot` awaits `apply_event` for each announcement, and
`attach_discovered_page` performs native initialization before publishing each
Page. Merely parallelizing `synchronize_sessions` cannot overlap those initial
Page announcements. A future design must separate bounded discovery preparation,
native work and ordered publication, accounting for Context classification,
duplicate announcements, opener/child ordering, network registration, partial
failure, reconnect epochs and readiness/retirement fences. Default callers must
remain serial; every extra concurrently decoded response requires an explicit
budget. No such production scheduler change was made here.

`attach_pair_probe.rs` and `scripts/probe-attach-pairs.mjs` add a separate fixed
ABBA experiment (widths 1, 2, 2, 1), using fresh Rust CDP connections while one
Playwright 1.58.2 observer stays connected. It requires the exact verified
loopback browser WebSocket, two to eight stable sorted Page IDs, at most two
caller-polled sessions/four pending prerequisite commands and five-second stage
deadlines. It only performs `Page.enable`, then paired `Page.getFrameTree` and
`Runtime.enable`: no script registration, world creation, navigation, debugger
resume, auto-attach policy, Page closure or runtime publication. Each acknowledged
probe-owned session is detached; connection Drop covers a lost attachment reply.
The existing transport configuration is finite; this diagnostic is not the
production-ledger host and makes no host/RSS-budget claim. Three offline inventory/
endpoint tests, Clippy and the live diagnostic process pass.

By this later experiment the live inventory has changed to two owned blank Pages
and only one existing slow Page. `pair-overlap.log` records serial 567/555 ms and
paired 587/591 ms, with identical inventories across all four legs and no errors
or detach failures. There is **no demonstrated performance improvement**. One
slow Page cannot test overlapping several slow Pages; the prerequisite-only
experiment also omits much of the production pipeline. Preserve this negative/
limited result instead of adopting concurrency or substituting it for parity.

### Screenshot Worker gates added; attachment still failing (2026-09-07)

The same evidence directory contains `screenshot-workers.log`. The release addon
was rebuilt after the trace addition, with SHA-256
`cde360daeea146ca107278fb36c89f34993be9c1693eb305562cef87f92d016c`.
No screenshot implementation, queue limit, five-second connection deadline,
fixture pixel expectation or supervisor deadline changed in this checkpoint.

Two new tests bring `test:screenshot` to 12 cases. A Worker submits four screenshot
calls and blocks its Node event loop until four native settlements are queued.
It checks the admitted FFI allowance and zero active host operations. One case
terminates there and checks exact owned Context/target disappearance. The other
unblocks delivery, requires four actual Buffers, closes/drains the host, copies
the images to the parent, then terminates while the Worker retains its original
Buffers. Parent-side decoding compares each image with an independent oracle
reference and checks copy independence. The queued-phase test checks settlement
lifecycle, not the success type of undelivered values; the publication phase
must separately prove image success. These are not RSS measurements.

The completed live run is **3/12, supervisor exit 1, 69.011 seconds**. Cases
1 (default PNG), 3 (explicit-quality JPEG) and 10 (Page-close queue settlement)
pass. Cases 2/4 fail candidate connection initialization, cases 5–9 fail oracle
connection, case 11 fails the Worker's candidate connection, and case 12 fails
the parent oracle connection before creating its Worker. Thus neither new
Worker lifecycle assertion was exercised successfully. No native panic,
cleanup diagnostic, overflow or supervisor signal appears. This is a red gate,
not a screenshot ownership regression diagnosis or a passing Worker proof.

Offline verification passes 88 host unit tests, 10 trace-probe tests and three
pair-probe tests (seven explicit live tests ignored in that offline command),
20 Node offline tests, Node type checking, diagnostic Clippy and the unchanged
224-package normal dependency boundary. No shared-corpus rerun was performed
for this trace-only rebuild; the preceding artifact's 25/9/2/21 report remains
historical, not evidence for this binary. NODE-013, both Worker gates and broader
screenshot compatibility remain open.

All runners are terminal. Both task-owned blank targets `9E4F…EA50` and
`C342…A25C` are absent from `final-tabs.log`; the two remaining unrelated game
Pages were not navigated or closed. The older task-owned target was reselected
through this task's saved CDP state because the CLI's ephemeral aliases do not
resolve native IDs across invocations. Chrome was not reset. No commit or push
was made; the local foundation commit remains `f76e3f2`.

### NODE-013 recursive attachment prerequisite join (2026-09-07)

The pinned Playwright 1.58.2 source at
`tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/lib/server/chromium/crPage.js`
(`FrameSession._initialize`, lines 369–408) starts `Target.setAutoAttach`
alongside Page/Runtime enable, frame-tree acquisition and empty world-script
registration. Pwright had awaited utility-world creation before sending recursive
auto-attach. Unlike cross-Page initialization, overlapping this one same-session
command does not require reordering discovered Page publication.

`ChromiumTopologyEventDriverProductionProfile::with_parallel_recursive_attachment()`
is a new explicit opt-in, now selected by the Node host. Page enable and the
injected Context initializer still finish first. The next caller-polled join
awaits frame tree, Runtime enable, optional empty world-script registration and
recursive auto-attach. Only after every branch succeeds may frame validation and
utility-world creation proceed, followed by debugger resume and Page publication.
Auto-attach is sent exactly once; both discovered Pages and existing session routes
use the same selected helper. Without a utility-world capability the registration
branch is inert and no script/world command is sent. Default, paired-only and
three-way registration profiles retain their prior command order.

The option requires four pending CDP slots and adds one `4 * max_response_bytes`
retained-response expansion beyond the three-way profile: **16 MiB extra declared
BackendStateBytes** in the Node recipe, not an allocated buffer or measured RSS
increase. It includes older prerequisite selections idempotently. No task, timer,
retry, cross-session parallelism, new dependency, event-queue enlargement or
connection-timeout increase is introduced. Target announcements arriving earlier
remain in the existing bounded driver inbox; they cannot recursively publish a
child before the parent transaction finishes. Early attachment failure is terminal;
world failure after successful attachment still cannot resume or publish the Page.

Evidence directory: `.work/node013-recursive-join.gnVF8T/`. Release composition
tests pass **287**, with 15 explicit live tests ignored in that offline command.
Ten new deterministic checks cover four-slot admission, exact/idempotent budgets,
each pending/error direction, owner cancellation, unchanged tree return, Context
initialization failure, discovered/existing session paths, absent world capability,
single attachment, world failure and queued announcement/publication ordering.
The previous 3/12 screenshot result remains a different artifact; this option is
not declared a NODE-013 fix from offline tests alone.

Final Node offline verification passes 20/20 and strict TypeScript checking;
workspace formatting and diff checks also pass.

The release Node addon is
`31333abc8490dd4ce36192fa7ce32abbf2b003d3223c47d7280deaa7c945cdd9`.
`host.log` passes 88 safe-host and ten trace-probe tests; the all-targets Clippy
run passes in `clippy-final.log` after replacing a test's `len() > 0` assertion
with `!is_empty()`. The normal dependency boundary remains exactly 224 packages.

`overlap.log` completes one fixed main/Worker/independent-host diagnostic without
errors. Main attachment takes 459.129 ms without the oracle and 1,724.204 ms with
it; Worker attachment takes 1,447.910 ms and the independent Rust host 1,534 ms.
The latter retains 28 commands/replies, no pending commands and no overflow.
For one slow target the four commands are prepared together at 33 ms and all
finish by 246 ms; world creation then runs 246–478 ms and resume 478–669 ms.
Another target's join runs 778–960 ms before world creation and resume. This is
direct evidence for the selected ordering and overlap, not a controlled speedup
claim: the browser now contains a different set of three Pages from the preceding
trace, with independently changing renderer activity. NODE-013 remains open.

Completed verification on that exact addon:

| Gate | Result | Scope |
| --- | --- | --- |
| Screenshot (`screenshot.log`) | **12/12**, supervisor exit 0, 13.904 s | Decoded PNG/JPEG, clipping, queued result ownership, Page close and both Worker phases now pass |
| Lifecycle (`lifecycle.log`) | **45/45**, supervisor exit 0, 101.026 s | Full package lifecycle/resource/navigation suite, no skipped cases or native cleanup diagnostic |
| Shared (`shared/report.json`) | Complete 57: **45 match / 10 unsupported / 2 oracle-error** | Zero value-mismatch, candidate-error, setup-error and cleanup-error; both screenshot cases match |

Shared execution runs `10:32:16.765Z`–`10:33:15.956Z` (59.191 s). Its supervisor
correctly exits 1 and reports `passed: false`, with no signal, overflow, timeout
or native failure. The two oracle errors are the existing back/forward history
cases; unsupported APIs remain unsupported. No fixture, expected value, case
catalog or deadline changed. This is not a complete compatibility pass.

The separate `live_frame_evaluation` integration target now retains the original
serial/main-world-only case and adds a recursive-join/utility-world case. Both
use the production event driver to initialize a cross-process iframe and route
typed evaluation to its main execution world, then dispose the exact fixture
Context/target while keeping Chrome alive. The first run is 1/2 because the new
fixture incorrectly reused the original two-world budget. Its opt-in profile now
explicitly declares four slots (main plus utility for each of two Frames); the
original case keeps two. `oopif-final.log` passes **2/2** in 0.66 s and the target's
Clippy check passes. This changes only the selected test fixture's resource plan,
not production Node limits, timeouts or assertions. It exercises OOPIF maintenance
routing, not a claim that every public Node Frame API is implemented.

All runners are terminal and reaped. `final-tabs.log` contains only two unrelated
game Pages; the owned preflight `4FFEBED2DA843FE8902E066B6DDDEB37` and temporary
fixtures are gone. No unrelated Page was navigated or closed, Chrome was not reset,
and the addon was never replaced while a browser lane was running. No commit or
push was made. The foundation remains `f76e3f2`; the full Node compatibility goal,
unsupported APIs, rendering transactions and intermittent NODE-013 diagnosis
remain open despite these completed green narrow regressions.

### NODE-029 sequential-typing foundation (2026-09-07)

Historical foundation stage; the later public binding and overlap failure are
recorded in the following checkpoint and the linked typing document.

The next public slice is `Locator.pressSequentially()` and the deprecated
`Locator.type()` alias, with browser semantics in Rust. Its independent input
foundation is now implemented: a borrowed 1 MiB UTF-8 view, scalar iteration
through literal layout keys or `Input.insertText`, injected delays and existing
Page transaction cleanup. It creates no task/timer/selector dependency and
retains the five-package input-only closure. Temporary modifier transitions now
use rich typed dispatch to fix DOM key location from 0 to the expected left-side
1, exposed by the new oracle rather than hidden by value-only comparison.

Evidence `.work/node029-sequential-typing.vs2gsP/`: 27 input/model release tests,
103 Locator regression tests and all-target input Clippy pass. A fresh pinned
Playwright 1.58.2 oracle on macOS Chrome 152.0.7977.76 compares 13 cases, including
exact key/input event fields and final values. The initial 9/13 modifier failure
is preserved in `live.log`; the fixed 13/13 run is `live-final.log`. A subsequent
test-only Chrome-version guard used reduced navigator UA incorrectly
(`live-complete.log`); the final native-metadata guard and all 13 comparisons
pass in `live-verified.log`. Owned temporary contexts/targets are disposed before
assertions; externally owned Chrome remains alive.

**NODE-029 remains open.** No public typing methods, new wire command or Node
addon were built here. Shared compatibility remains the earlier 45 match /
10 unsupported / 2 oracle-error result, not a new run. Required next work is to
freeze current Locator 1.22/2.5 baselines, add both typed routes and focus/input
composition (without Press navigation waiting), bind host/FFI/JS aliases, decide
unpaired UTF-16 handling, and prove public error/cancel/Worker behavior with
same-artifact differential tests. Raw Keyboard lifecycle is a separate optional
capability, not implicitly implemented by this primitive. [Detailed contract,
source findings and reproduction](knowledge/pwright-sequential-typing.md).

### NODE-029 public binding and overlapping-input discrepancy (2026-09-07)

The typed vertical slice is wired through independently frozen Locator 1.22/2.5
baselines, generated 1.23/2.6 schemas/capability/server/reference outputs, method
131095 and tags 45/46, bounded handwritten codecs and client extensions, both
Chromium compositions, safe host action dispatch, N-API conversion and thin
Node aliases. The backend manifest advertises `locator.press-sequentially`.
Strict focus and literal input execute in Rust without Press's navigation wait.
Owned text capacity is capped before backend boxing and v2 compatibility cloning;
N-API reuses its admitted 6 MiB action ticket. Unpaired UTF-16 explicitly rejects.

Evidence is `.work/node029-public-typing.R4GzWn/`. Initial artifact
`4a098d75...710dcd` passes public 27/27 and shared 46/9/2 (47.801 seconds, complete,
no value/candidate/setup/cleanup error). The first shared fixture preflight
failure remains recorded: case JSON moved to `tests/data` rather than weakening
the HTML MIME verifier. Required core/version/capacity checks were updated to
include the new route; original old-minor/golden-byte tests remain intact.

Final manifest-complete artifact is
`cfc74e536ed6260e8137226f74fea7b62396c6b62b9a743733c826c4c61b4b4b`.
Aggregate backend/Chromium/standalone Locator tests pass 18/287/4 respectively;
the portable/backend/client/codec/server/host regression matrix, generator checks,
27 input tests, 20 Node unit tests, TypeScript, release Clippy and the 18-package
Locator / 224-package host dependency boundaries pass. Ignored live targets are
not promoted to passes.

Expanded public verification is **27 pass / 1 fail / 0 cancelled** in
`typing-isolated.log` (23.374 seconds), supervisor exit 1. After a confirmed
`keydown(a)` in a 30-second delay, Playwright completes a second `type('C')` and
shows `aC`; Rust stays pending after two seconds and shows `a`. Closing the test
Page then settles both calls and releases all checked resources. The earlier
`typing-expanded.log` timed out before closing this Page and contaminated the
following FFI-count assertion; preserve it as a test-isolation failure, not an
independent production leak claim. The corrected test fails on the actual
progress/value discrepancy and leaves subsequent tests clean.

`begin_transaction` and `cleanup_pending` contend for one Page lock held across
the first delay. A waiting call has no per-operation input ownership token, yet
its cancellation drains the shared coordinator. Next work must separate global
physical-key state, per-action ownership and explicit concurrency policy while
preserving bounded resources and default behavior. Do not hide this by longer
API deadlines, skipped comparisons, text replay or falsely completed cleanup.
NODE-029, NODE-022 and the full compatibility goal remain open.

The final artifact's own `shared-verified/report.json` is now terminal and
complete: **46 match / 9 unsupported / 2 oracle-error**, zero other failures,
62.701 seconds (2026-09-07 11:33:38.173Z–11:34:40.874Z), supervisor exit 1.
This is same-artifact evidence alongside the expanded public 27/28 gate.
The [typing design](knowledge/pwright-sequential-typing.md#concurrency-implementation-constraints)
records the next execution/cancellation identity boundary and the distinction
between Page-wide physical-key state and per-action retirement. Upstream's key
set is not a per-call reference count; simply splitting the mutex and releasing
all keys at action completion would introduce additional discrepancies.

### NODE-029 interleaved input policy and scoped NODE-022 fix (2026-09-07)

The input leaf now exposes `CdpInputPolicy::{Exclusive, Interleaved}`. The former
retains existing Rust behavior. The latter owns a fixed Page key/button state
record, mutates logical state before dispatch, and gates individual native
events without retaining a lock through an action delay. Node selects it in
the Locator production profile; Chromium shares the resulting Page coordinator
with its Frame routes. No new schema, JS browser logic, task or dependency is
introduced. A 16-code Page limit rejects new codes before dispatch when full;
repeat downs do not consume more slots. Timeout intentionally retains Page
state and does not invoke release-all. Runtime native retirement remains
separate from the input cleanup hook, which has no per-call physical-key owner
in this policy. See the [input contract](knowledge/pwright-sequential-typing.md#opt-in-interleaved-page-input-2026-09-07).

Evidence: `.work/node029-interleaved.jRHGpv/`. The initial interleaved artifact
`e20765bf...781e81` passes 30/30 typing, but actions remain 25/26: Shift is now
retained after timeout, revealing that `press('a')` was incorrectly resolved as
physical `KeyA` under Shift. Pinned upstream's literal aliases have no shifted
variant. The shared Rust resolver now keeps literal characters literal and
shifts physical key codes only. The original red evidence is preserved.

Final artifact SHA-256:
`5a746ac883be46d8fe4516df6c65a92e24457b39bb47af60a61cb3b34f3ba900`.

- `typing-final.log`: **30/30**, 29.065 seconds, exact native event/value
  comparisons for different/same-key overlap and timeout while waiting for a
  missing target, plus Page/Worker ownership checks.
- `actions-final.log`: **27/27**, 26.557 seconds, including the unchanged
  NODE-022 assertion and a new literal/code Shift distinction case. No test was
  skipped or normalized into a pass.
- `shared/report.json`: complete **57 cases**, **46 match / 9 unsupported /
  2 oracle-error**, zero value/candidate/setup/cleanup errors, 61.122 seconds.
  The report records the final artifact hash. Exit 1 remains correct for an
  incomplete compatibility corpus.
- Final input/Locator release regression passes **32/103**; the initial policy
  matrix also passes **287 composition / 89 host** (15/5 live cases ignored).
  Final all-target release Clippy, 20 Node unit tests and TypeScript pass.
  Dependency proofs remain **5 input / 18 Locator composition / 224 host**.

This resolves the recorded overlap and NODE-022 examples, not the full input
contract. Still required: cross-Frame focus/state, element/document replacement
during typing, cancellation at native enqueue/ack, modifiers changed between
physical down/up and repeated-timeout key capacity. Native events currently
serialize through acknowledgement; upstream pipelining parity is not proven.
Raw Keyboard and the remaining public API/packaging discrepancies remain open.

Broader regression remains red: `lifecycle.log` has 18 passing and 3 failing
completed cases before the 180-second SIGKILL, not a complete 45-case result.
Failures cover native window geometry, Worker readiness and topology setup;
the cause is unproven despite similarity to NODE-013/014. The following
`screenshot.log` fails 12/12 during oracle/candidate attachment (73.007 seconds),
then `screenshot-recheck-no-cleanup.log` passes 12/12 (15.091 seconds) without
changing the artifact or restarting Chrome. No successful intervening cleanup
occurred, so it is not a cleanup-based fix. Both records remain authoritative.
After every runner was terminal, the exact leaked lifecycle fixture target was
reattached and closed using Pwright; target absence was verified while preserving
unrelated Pages. Manual cleanup is not automated lifecycle success. Recovering
the full lifecycle gate and diagnosing attachment progress is the next priority.

## NODE-013 lifecycle supervision evidence (2026-09-07)

Evidence directory: `.work/node013-lifecycle-trace.ccVzSR/`. The native addon
remains `5a746ac883be46d8fe4516df6c65a92e24457b39bb47af60a61cb3b34f3ba900`;
no Rust implementation, public API deadline or oracle assertion changed.

Immediate bounded TAP-comment tracing distinguishes test start, controller and
tested-client attachment, action execution, browser close and fixture cleanup.
The first traced `lifecycle.log` completes **44 passing cases, zero reported
failures**, then the original 180-second suite supervisor kills test 45. Its
candidate connection starts at 176602 ms: the kill occurs only about 3.4 seconds
into an operation with a 30-second API deadline. This is an incomplete run, not
45/45 and not evidence of that operation exceeding its API timeout. The reserved
close-admission case 22 completes: action starts at 67113 ms, browser close is
done at 67115 ms and all helper cleanup finishes at 67141 ms. The previous
last-completed TAP case was not enough to identify a stable close deadlock.

After that runner ends, `attach-probe.log` passes its unchanged five-second Rust
diagnostic: 40 ms, 42 commands / 42 responses, no pending command, 483 omitted
events and no overflow. Native close/resource assertions also pass. This is one
successful diagnostic, not a reproduction or resolution of intermittent
attachment failure. The interrupted fixture target
`D047B0D3762EF3BAA47B100923821043` is verified by task URL/title and closed through
its own Pwright state; `orphan-cleanup/targets-after-cleanup.log` proves absence
while all three unrelated game Pages and the preflight Page remain.

The lifecycle test supervisor now separates **600 seconds for the whole corpus**
from **60 seconds per case**, including setup, action and teardown. Startup,
between-case and process-exit waits each have an independent 60-second bound.
Only validated start/end IPC announcements replace the timer; fixture phase
logging cannot extend it. Names are bounded to 512 characters, announcement
order is checked and at most 128 cases are accepted. Tracing admits at most
4096 records with 128-character phases. All package lanes now bound stdout to
4 MiB; the existing stderr diagnostic cap and native panic/cleanup failure
checks remain. Other lanes retain their original 180-second suite limit. These
are test-process policies, not new browser timeouts or production tasks.

Offline verification: **29/29 Node unit tests**, TypeScript and the **224-package**
safe-host boundary pass (`unit-final.log`, `types-corrected.log`,
`boundary-final.log`). The earlier `types-final.log` used a nonexistent script;
the corrected command is `pnpm --dir packages/pwright test:types`.

`lifecycle-watchdog.log` now completes **45/45**, no failures, cancellations or
skips, in **192.715 seconds**, supervisor exit 0. Every original case and API
deadline is retained, including window geometry, Worker destruction, queue-full
close admission and document replacement. The native addon hash is unchanged.
The full corpus duration exceeds the old 180-second bound; separating aggregate
test supervision from individual API deadlines permits completion while a
stalled case still fails at its independent 60-second bound. This is one green
rerun, not proof that the earlier NODE-013/014 failures or broad input/packaging
discrepancies are fixed. No browser restart or unrelated Page cleanup is used.

The subsequent serial `screenshot-watchdog.log` passes **12/12 in 19.324 seconds**,
supervisor exit 0, including both Worker ownership cases, with the same addon
and its unchanged 180-second lane budget. The final target inventory contains
only the three unrelated game Pages plus our original preflight Page; no newly
orphaned test Page remains. The preflight target is then closed through its own
Pwright state and `targets-final.log` verifies that only those three unrelated
Pages remain. Neither passing run uses a browser restart. Remaining priorities
are causal NODE-013/014 diagnostics, broader native input cancellation/Frame
coverage and the unsupported API/packaging work, not declaring complete parity.
