<!-- agent-updated: 2026-10-20T19:54:17Z -->

# Playwright Server connect boundary (pinned v1.58.0)

Current source checkpoint (ABI127, 2026-10-20): Python now has a separate,
default-off `playwright-server-close-notices` feature. It composes the Rust
session `close-notices` policy and browser-ops `close-state` feature without
selecting full remote Page network events or requiring `frame-title`.
`EventPolicy::ProjectCloseNotices` validates ingress through the existing
protocol core. Its additional close-only projection retains only
Root/BrowserType/Browser/Context/Page topology and advances object generations
even for unretained objects; it does not project or queue Request/Response
notices. The base supervisor still parses and retains protocol objects, with
its separate 16,384-GUID and 4 MiB budgets, so the whole connection is not
independent of network-object volume. The close projection has a 4,096-object,
approximately 4 MiB retained-metadata, and 128-notice/approximately 4 MiB
queued budget; these are fail-closed projection limits, not whole-process RSS
ceilings. Its typed `CloseNotice` stream carries Page, Context and Browser
GUID/generation identities, followed by a terminal `Disconnected` notice;
child notices are ordered before ancestor notices. The Python native bridge
returns `(kind, guid, generation)` from one asynchronous
`NativeRemoteBrowser.next_close_notice()` reader. The Python facade uses one
ordered watcher and generation-keyed wrapper registries, with a bounded
creation fence for notices that overtake wrapper publication. A server-origin
Page close can therefore notify `page.on("close")` while its Context and
Browser remain live; Context close emits child Page close first. Browser
`disconnected` is associated with terminal disconnection, not merely a
Browser close notice. The older connect-session profile retains its previous
whole-session-only watcher behavior. A fresh ABI127 release wheel (SHA-256
`8b524b5b424ff2b5dd3a8f871af3a6dbae961fddfdaf895b6000ecc5ff919cb9`)
passed 84 focused installed-wheel tests. A Tailscale-bound synthetic WebSocket
fixture pauses between Page/Context `close` and `__dispose__`, confirms the
native live inventories exclude closed objects, and checks Page-only,
Context-only, then terminal Browser callbacks; it passed ten independent
reruns. This is synthetic-server evidence, not a real Playwright Server or
unchanged upstream test pass, and promotes no official ledger row.
An independent close-notices-only ABI127 wheel (SHA-256
`02a13c04b0b1b9bee4e9e5fa9f0dbdb3a98bda8a61f53c05198f5af11885e4a4`)
omits the `frame-title` feature and native `NativeRemotePage.title` method;
its 51 close/ABI/fake-connect tests pass. This is feature separation, not proof of a
minimal full-package dependency footprint.

The independent ABI126 `playwright-server-frame-title` feature already has a
fresh installed-wheel synthetic WebSocket proof. Its actor checks the adopted
main Frame and complete Frame→Page→Context→Browser generation/parent chain
both before `Frame.title` is sent and when its correlated reply is accepted;
Page `close` tombstones the handle without removing it before `__dispose__`.
The selected wheel and release-test counts are recorded in the
[conformance checkpoint](../upstream-playwright-conformance.md), not evidence
for the newer ABI127 close-notice wheel.

The `pwright-playwright-server-session` extension now retains
pre-`Root.initialize` lifecycle messages and ordinary events in wire order,
then replays them through `receive_next` before reading newer frames. A
pre-reply Browser `close` or disposal prevents a connected lease from being
published. The replay queue is capped at 1,024 messages and 1 MiB of original frame
bytes; either limit closes the transport and rejects the pending initialize.
Parsed JSON values and GUID graph entries add heap overhead, so the wire-byte
limit is not an RSS measurement or a strict total-heap ceiling.
Default and `live-tokio` release session tests passed 19/19; the earlier
supervisor all-feature release checkpoint passed 40/40 with strict Clippy. The
supervisor drains bootstrap ingress before returning its handle, polls the
connection continuously, correlates at most 128 commands, and keeps ordered
events in a 128-entry queue with an approximate 4 MiB serialized-payload
budget by default. The original Node and pre-ABI127 Python profiles without
public remote event listeners select the explicit drop policy, which validates
each frame, projects object and terminal
state, and does not queue unsupported ordinary events. `next_event()` then
fails fast instead of suggesting events are available. It reports the count of
queued pre-reply ingress so a language adapter
can project objects before returning a public Browser. A terminal-state watcher
can await disconnect without retaining the supervisor or socket owner. A
wire-ordered GUID initializer projection is visible synchronously through
`object()` and `snapshot()` before the related command reply is published.
It caps objects at 16,384 and retained serialized initializer envelopes at
approximately 4 MiB; that is not a total RSS limit. Queue
pressure fails closed; an aborted actor reports `Stopped` instead
of a stale connected state. Parsed JSON overhead and unread reply values are
outside that event budget, so this is not a total RSS bound. Two explicitly
selected Tailscale-bound live WebSocket fixtures pass: the raw lease checks a
real upgrade, `Root.initialize`, GUID validation, ordered pre-reply replay and
close-frame handling; the supervisor checks immediately available bootstrap
events, command/reply correlation, later event ordering and clean close. Both
use a synthetic server, not a real Playwright Server or official conformance
test. The pinned server constructs dispatchers before its initialize reply and
can emit existing-context events while constructing a shared Browser, so a
future language binding must consume the supervisor's replay and populate
`Browser.contexts` **before** returning the public Browser. The current Node
binding reads the graph and fails closed if it encounters an externally
created Context without an owned public wrapper; it does not yet materialize
such Contexts. No unchanged official successful-connect test exercises this
source change.

A new independent, default-off
`pwright-playwright-server-endpoint-discovery` Rust crate handles the upstream
HTTP(S) `<path>/json` lookup and converts its `wsEndpointPath` to a checked
WS(S) endpoint. The default feature uses an injected fetcher, and the
`live-reqwest` fetcher must be selected explicitly. It sends only the supplied
User-Agent on the discovery GET, retains the original URL query, caps the
retained response body at 64 KiB and the deadline at 120 seconds, and redacts
query/body details from errors. Default and live-feature release suites pass
12/12 and 15/15 respectively, with selected strict Clippy and no Reqwest/Tokio
in the crate's default dependency closure. The live HTTP fetcher follows any
3xx response with a valid `Location`, at most five hops under one cumulative
deadline, retaining the original endpoint as the WebSocket base. Three explicitly
selected live HTTP fixture tests pass for the direct case, a cross-origin 304
redirect, and the redirect limit; each bound only to this Mac's Tailscale IP
and used its full MagicDNS name. The redirect fixture also proves the original
query is not sent in a synthetic `Referer` header. Unlike upstream's generic
HTTP helper, the chain is bounded; invalid/oversized redirect targets,
credentials and fragments are rejected. CI exercises default and live-feature
release tests, all-target
strict Clippy and the default dependency closure, but excludes the explicitly
selected Tailscale fixtures. There is no official conformance credit for this
component. A separate, default-off `pwright-playwright-server-connect` crate
explicitly composes direct WS(S) or HTTP(S) discovery, caller-validated
WebSocket headers, `Root.initialize` bootstrap, and the supervisor. Its generic
connector and fetcher are injected; real Reqwest/Tokio I/O requires the `live`
feature. Default and all-feature release suites pass 9/9 each, and one
explicitly selected synthetic Tailscale-bound HTTP-to-WebSocket fixture passes.
Its default dependency closure excludes Reqwest and Tungstenite. This is
component evidence, not a public language-binding pass or a real Playwright
Server test. An independent default-off
`pwright-playwright-server-browser-ops` crate passes 19/19 release tests for
real `Browser.newContext`, `BrowserContext.newPage`, and empty-option
`BrowserContext.close` commands. It verifies the returned GUID's type, parent
and creation sequence, rechecks parent generations after creation replies,
and retains ancestor lineage in handles. Close checks the generation before
send and accepts a correlated empty reply even after server-side disposal;
an in-flight command could still have a wire-side effect after GUID reuse
without a server-side compare-and-close or trusted never-reuse contract. It does not validate the
full required Context/Page initializer schemas;
malformed successful results do not yet invalidate the supervisor.

The opt-in Node `playwright-server-connect-session` profile composes live
HTTP discovery, checked WebSocket headers, `Root.initialize`, and supervised
connection ownership into a public Browser. It selects the separate Rust
browser-ops crate for empty-option `Browser.newContext` and
`BrowserContext.newPage`, with graph-backed liveness and GUID-reuse rejection.
Its release build, strict Clippy, 18 wrapper tests, native-class boundary,
TypeScript, and four selected Tailscale-bound synthetic native tests pass; the
latter cover 17 sequential retained-wrapper connections, disposal, outer URL
control normalization, a 160-event burst, whole-session terminal Page→Context→Browser
close ordering, and a single overall timeout. Arbitrary non-terminal child
close events remain unsupported. A separate selected real-addon
Tailscale fixture passes 2/2 for explicit Context close and implicit-Context
`Browser.newPage()`/`Page.close()`, with Page-before-Context close order while
Browser stays connected. Context-owned Page.close,
rich options, navigation/evaluation, and externally
created object wrappers remain unsupported; `timeout: 0` is conservatively
capped at 120 seconds. Python ABI125 separately returns a remote lifecycle
Browser, with version, connected state, close and one `disconnected` notice,
plus empty-option Context/Page creation. Its graph-backed inventories fail
closed on unknown or recycled generations. The fresh selected wheel passes 49/49
focused Python tests on each CPython 3.9.6 and 3.12.14, including four Tailscale-bound synthetic
HTTP-to-WebSocket variants (one with a 160-event burst). A prior Rust addon
checkpoint passed 41/41 release tests; that ABI125 source passed strict release
Clippy and wheel build, but direct addon unit testing could not link on this Mac
without `libpython3.9`. A newer ABI125 wheel with empty-option Context close
and implicit-Context `Browser.new_page()` passes a focused subset of 65/65 on
each Python version with Tailscale fixtures enabled (six compiled-extension
WebSocket cases); Context-owned Page.close and arbitrary externally initiated
child-close notifications were unsupported in that ABI125 wheel.
On whole-session termination, Python emits Page `close`, Context `close`, then
Browser `disconnected` once. Direct sync `_LoopThread` callers without a
caller dispatcher receive queued callbacks only at the next caller-thread API
boundary; a fully idle caller does not receive prompt notification. That
ABI125 wheel did not support arbitrary non-terminal child close.
Neither
binding has been exercised against a real Playwright Server. A pinned,
fixture-adapted Core lifecycle diagnostic passes two unchanged assertions in
each oracle/candidate lane, but it is not an unchanged official-server pass or
ledger credit; see the
[attestation](../../.work/official-conformance/results/core-connect-lifecycle-synthetic-v1-green-v2/attestation.json).
The separate browser-free Core connect-timeout v1 diagnostic initially
reported candidate 18/36 against oracle 36/36: a nested supervised WebSocket
upgrade timeout reached the Node host mapper as a generic backend error. The
typed mapping fix preserves the redacted internal error contract, and a fresh
[v2 attestation](../../.work/official-conformance/results/core-server-connect-timeouts-strict-v2/core-server-connect-timeouts-strict-v1-fixed-v2-20261012/attestation.json)
verifies 36/36 expected outcomes in each lane. Two literal fixture hosts were
adapted; this is implementation-internal and not official ledger credit.
The separate [Core HTTP-redirect timeout v1 RED](../../.work/official-conformance/results/core-server-connect-redirect-timeout-strict-v1/core-server-connect-redirect-timeout-strict-v1-second-20261012/attestation.json)
records oracle 9/9 and candidate 0/9 because a reserved `Connection` header
was rejected before HTTP discovery could time out. The independently verified
[v2 GREEN](../../.work/official-conformance/results/core-server-connect-redirect-timeout-strict-v2/core-server-connect-redirect-timeout-strict-v2-fixed-20261012/attestation.json)
records 9/9 in each lane. Rust still validates header syntax, size and count
before discovery, then enforces the reserved-name rule before the actual
WebSocket upgrade; direct WS(S) rejects immediately. The selected real-addon
Tailscale fixture passes 3/3 and seven verifier mutation controls pass.
Only the fixture host/bind is adapted, so this internal diagnostic is not
official ledger credit or real Playwright Server proof.
The historical [Core remote-Page waiter RED](../../.work/official-conformance/results/core-connect-waiters-synthetic-v1-red-v5/attestation.json)
records oracle 3/3 and candidate 0/3 on unchanged upstream `run-server`
assertions after replacing only the server launcher. Both sides create
Context/Page; the candidate fails at its first missing `waitForRequest` or
`waitForEvent`. No candidate assertion or `waitForResponse` call is reached;
this proves neither event delivery nor close ordering and earns no ledger credit.
The existing `playwright-server-connect-session` Node binding still drops
ordinary events. A separate selected Page-events profile now exists; passing
the teardown tests still requires real request/response/download delivery and
terminal ordering, not shutdown-only waiter stubs.
The default-off
[`pwright-playwright-server-page-events`](../../crates/pwright-playwright-server-page-events/src/lib.rs)
leaf is the bounded Rust projection for that profile. It consumes
validated `Ingress` in wire order, retains only Page-attributed
Request/Response/Download metadata and terminal notices, and closes on
invalid attribution or capacity overflow. Limits are 4,096 objects,
approximately 4 MiB retained metadata, and 128 notices/approximately 4 MiB
queued; 16 release tests and strict Clippy pass. The session's opt-in
`ProjectPageNotices` policy projects inline and drops unrelated validated
events, with 50/50 selected release tests including a 160-event burst and
ordered close synthesis. Rust browser-ops sends and acknowledges both Page
network subscriptions before Node Page publication (22/22 selected tests).
The separate `playwright-server-connect-page-events` Node addon profile builds
and loads; its JS adapter passes 25/25 focused tests, including early
Context/Page close and terminal-predicate regressions. An actor-side target
generation check blocks an already-observed GUID recycle before a command is
sent, but cannot rule out an unseen remote race. The independently verified
[v5 GREEN diagnostic](../../.work/official-conformance/results/core-connect-page-events-synthetic-v2-green-20261020-v5/attestation.json)
passes the same three unchanged Core teardown assertions 3/3 in both oracle
and real-addon candidate lanes. A separate positive probe in each lane
delivers Request→Response→Download to matching waiters and Page listeners,
with Request identity preserved. The server launcher remains a browser-free
synthetic replacement: this is not an unadapted official pass, real Playwright
Server/browser or full public-API parity proof, and promotes no ledger row.
The original `playwright-server-connect-session` profile still drops ordinary
events; the later Python close-only feature is independent of the Node
Page-event profile and does not add network-event delivery.
An independent fixture-adapted Python lifecycle diagnostic passes four
unchanged async/sync assertion bodies for each of Chromium, Firefox and WebKit
parameters (12/12 oracle and 12/12 earlier ABI125 wheel candidate). It replaces only the server
launcher with a browser-free protocol fixture; the
[attestation](../../.work/official-conformance/results/python-connect-lifecycle-synthetic-v1-green-v1/attestation.json)
is not real-browser, real-Playwright-Server, unadapted official or ledger evidence.
The pinned Python sync Chromium child-close assertion body passes 1/1 in both
oracle and selected-wheel lanes in an independently verified
[attestation](../../.work/official-conformance/results/python-connect-close-events-synthetic-v1-green-v1/attestation.json).
Its server-launch fixture is replaced, so it is equally browser-free and
non-promotable.
The Core line-382 child-close case is preserved as a paired
[RED](../../.work/official-conformance/results/core-connect-child-close-synthetic-v1-red-v1/attestation.json)
oracle 1/1 and candidate 0/1 timeout, then
[GREEN](../../.work/official-conformance/results/core-connect-child-close-synthetic-v1-green-v1/attestation.json)
1/1 each after the Node terminal-emission change. Only the server-launch
fixture was replaced; neither capture promotes an official ledger row.

Historical ABI123 observer checkpoint: the working tree introduced a default-off
`playwright-server-initialize` observer and Python async/sync
`BrowserType.connect` entry points. The observer sends a bounded
`Root.initialize` message and reports a remote WebSocket close; it does **not**
create a remote GUID object graph or return a Browser. Official post-fix
verification passes only the three unchanged early-close cases, 3/3 in each
isolated Python candidate/oracle lane with six strict protocol receipts. The
immutable paired ledger is
`.work/official-conformance/outcomes/python-connect-close-post-fix-strict-paired-v2/`.
This remains separate from Chromium-only `connect_over_cdp`.

An ABI123 source change maps typed WebSocket connection failures to the
official `WebSocket error: ` public error prefix. The selected release wheel
passes 290/290 contract/connect-facade unit tests on each fresh CPython 3.9.6
and 3.12.14 environment. The six async/sync official closed-server cases
pass 6/6 in each candidate/oracle lane with independently verified receipts,
but use a versioned browser-free replacement for the upstream server-launch
fixture. The six rows per lane are promoted in the immutable
[paired successor](../../.work/official-conformance/outcomes/python-connect-refused-strict-paired-v1/promotion-receipt.json),
whose receipt explicitly marks `publicApiParityEvidence=false`. This proves only
the refused-connection public error behavior; see the
[strict evidence scope](pwright-python-connect-refused-strict.md). ABI122
early-close evidence below is historical and remains unchanged. A separate, browser-free
`pwright-playwright-server-client-core` now models bounded command IDs, replies,
GUID ownership, events, and terminal cleanup, but is not yet wired to either
language binding or a live transport.

The subsequent source-only initialization hardening no longer treats every
`id=1` frame as success: it requires a `result.playwright.guid`, preserves a
typed remote error, and rejects an ID-only reply. Its seven release unit tests
pass, and the selected Python addon passes release `cargo check`. The frozen
ABI123 wheel and six official case captures above predate this change; this
hardening has not earned additional official conformance credit.

A separate default-off `pwright-playwright-server-session` composition now
connects the bounded WebSocket transport and protocol core. It yields a
connection-owned Browser lease only after checking the `Root.initialize`
reply, required Playwright/BrowserType/Android/Electron GUIDs, and a real
`preLaunchedBrowser` with matching parent, name and version. It fails closed
on malformed/early-close/timeout paths and clears pending commands. The
current 19/19 base and 32/32 supervised release suites and pre-reply replay
behavior are described above; the earlier 14/14 checkpoint is historical.
At that component checkpoint there was no Python/Node adapter, Context/Page API,
or real BrowserServer E2E. A caller of the raw lease must drive
`receive_next` to notice post-bootstrap closure; the optional supervisor does
so continuously.

Node now has a separate opt-in `playwright-server-connect-ws-error` profile.
Its thin `BrowserType.connect` adapter calls a bounded Rust WebSocket/DNS
diagnostic and exposes the official-style `getaddrinfo` error markers while
omitting endpoint query parameters. Two unchanged pinned Core line-122
WebSocket-error assertions pass 2/2 in each independently verified v2
candidate/oracle run; the v1 RED run remains preserved. Those upstream tests
use internal fixtures, so they are diagnostic-only and not public API parity.
On any successful WebSocket upgrade the adapter still rejects as unsupported;
it never fabricates a remote Browser. This is separate from the session crate,
which is not yet bound to Node or Python.
A second default-off `playwright-server-connect-close-error` Node profile
selects the bounded `Root.initialize` Close observer, not the DNS-only path;
it also refuses to fabricate a Browser on successful initialization.

A third, separate default-off `playwright-server-connect-headers` Node profile
uses `pwright-playwright-connect-headers` to compose the pinned Playwright
User-Agent, `x-playwright-browser`, and empty `x-playwright-proxy` in Rust.
Caller headers take precedence regardless of name case. The same bounded
`WsRequest` validates explicit and synthesized fields, including the 64-header
limit and reserved WebSocket handshake names. The thin Node adapter supplies
only the caller's pairs, browser name, and runtime observations; a successful
upgrade still returns unsupported instead of fabricating a remote Browser.
The current header ABI passes runtime identity as one bounded three-string
tuple (`platform`, `arch`, `release`, at most 64/64/128 UTF-8 bytes); the
selected addon proves this five-argument native port with the profile-local
`playwrightServerConnectHeadersAbiShape() === 2` marker. The current loader
rejects the older header addon despite its matching port name.

The latest frozen post-refactor v2 diagnostic reruns the same four original
Core header cases, and both candidate and independent oracle again pass
**4/4**. The candidate artifact SHA-256 is
`0b2553d3137a77f162836d8fadeacbf1d5e47038b24ae97fe9d1c9519d068194`;
the strict [candidate](../../.work/official-conformance/results/core-server-connect-headers-strict-v1/post-refactor-v2/candidate-v1.strict.json)
(SHA-256 `e9d05121603d7fd8c30b2f30286d3afb78ccf8bb5a255a2c1be8fafff0f129ec`)
and [oracle](../../.work/official-conformance/results/core-server-connect-headers-strict-v1/post-refactor-v2/oracle-v1.strict.json)
(SHA-256 `dcbb6c8974da20ac308541d518f87b6e307f7301050c6cff51e4e92b9de02e85`)
sidecars pin the frozen snapshot and its ABI-shape proof. They verify four
ordered Tailscale-bound upgrade receipts per lane, including the asserted
explicit and default header values. The default UA in this host's oracle is
`Playwright/1.58.0 (arm64; macOS 26.6) node/26.7`. As in v1, only the two
fixture URL hosts at lines 238 and 253 were adapted; the socket receipts lack
per-spec IDs, so variant attribution is ordinal. This remains
`official-derived-fixture-host-adapted-internal`, with no unadapted official
pass, public API parity, or Core ledger promotion.
The receipts check selected assertions, not the complete header set or proxy
header parity.

The historical, frozen, browser-free Core header v1 diagnostic selects the
original `browsertype-connect.spec.ts` assertions at lines 235 (explicit `User-Agent`
and `foo`) and 250 (default `User-Agent`, `x-playwright-browser`, and `foo`),
each under `launchServer` and `run-server`: **4/4 passed in both candidate and
independent Playwright 1.58.0 oracle lanes**. The only source adaptation
replaces the two fixture `localhost` WebSocket URL hosts at lines 238 and 253
with this Mac's full Tailscale MagicDNS host. The candidate ran from a frozen
package snapshot with pre-ABI-arity-refactor selected native artifact SHA-256
`30d8478b0d74acfe556b5209633045434503738c4424483ab7ea7bb952f358aa`.
The strict [candidate](../../.work/official-conformance/results/core-server-connect-headers-strict-v1/final-frozen-v1/candidate-v1.strict.json)
(SHA-256 `9cd89efe6495a99404d75a613a8e41870b31c87c1badb7cd5e9aed9306b3730b`)
and [oracle](../../.work/official-conformance/results/core-server-connect-headers-strict-v1/final-frozen-v1/oracle-v1.strict.json)
(SHA-256 `32341cf62335ee5455a826a35993f0ff9c1ca3adcd0ee306420f107f51f31905`)
sidecars independently verify four ordered upgrade receipts per lane. Those
sanitized socket receipts have no per-spec ID, so assignment of a receipt to
one of the four specs relies on execution order and header-case counts, not
direct socket-to-spec attribution. This is
`official-derived-fixture-host-adapted-internal` diagnostic evidence, not
current-source parity, an unadapted official pass, public API parity, or a
Core ledger promotion.

The opt-in live WebSocket transport now preserves an HTTP upgrade refusal as
an explicit status plus at most 8 KiB of response body, while its `Debug`
representation redacts that body. A real in-memory WebSocket handshake against
an HTTP 401 response verifies that `Unauthorized body` reaches the caller. An
explicitly selected live connector test also passes against a fixture bound
only to this Mac's Tailscale IP, addressed via full MagicDNS; release transport
tests and strict Clippy pass. The existing selected Node diagnostic profile
now exposes this error. In a versioned, host-adapted Core line-116 diagnostic,
two original HTTP 401 assertions pass in each candidate and independent
Playwright oracle lane; each lane's receipt records two real 401 sends, but
does not correlate each send to an individual spec ID. The frozen candidate
snapshot, strict sidecars and verifier tests are under
`.work/official-conformance/results/core-server-connect-http-401-strict-v5/`.
This is `official-derived-fixture-host-adapted-internal`, not an unchanged
official run, public API parity, ledger promotion or successful remote Browser.
After this frozen capture, the selected fork gained bounded split-read HTTP
rejection-body collection for valid `Content-Length`; both its raw buffer and
lossy UTF-8 output are capped at 8 KiB. Missing or malformed length still
returns the same-read tail without waiting for a keepalive connection. The
current source passes focused release, TRACE, TLS and live Tailscale 401
tests, but the old HTTP v5 sidecar does not attest these later bytes.

A separate host-adapted Core Close-error v4 diagnostic passed the original
line-130 assertions in both lanes, but its wire receipts exposed a real
candidate lifecycle defect: the server saw abnormal Close `1006` rather than
the oracle's `4123 / Oh my!`. That frozen v4 result remains lifecycle RED.
The v4 verifier also assumed separate worker IDs even though both cases ran
on worker 0, so neither v4 sidecar passes that verifier; its raw receipts and
hashes remain available for direct comparison.
The source fix explicitly flushes the WebSocket Close reply after receiving
the fixture's Close. In the subsequent v5 strict candidate and oracle lanes,
both original line-130 assertions pass 2/2 each; the independently checked
server receipts contain exactly two ordered upgrade, message and
Close `4123 / Oh my!` triplets in each lane. Both specs reused worker 0 and
port 8907, so the verifier correlates them by report order and ordered wire
events rather than inventing distinct workers. The receipt has no spec ID or
timestamp per triplet, making that per-spec association an ordinal inference.
The v5 result remains
`official-derived-fixture-host-adapted-internal`, with no unadapted official
pass, public API parity or Core ledger promotion. The new candidate artifact
has SHA-256 `39a8ecfdac8db8d0cf1eb0e100c3c17aaf59fdf9c4dbb4f9739602202f0843f1`.

This opt-in observer is experimental, not a production remote-browser client.
Its current source selects local pinned no-secret-log Tungstenite forks for the
live transport; the fork-path TRACE sentinel and native TLS tests pass. The
frozen ABI123 wheel predates this change, the forks are not yet published, and
full connector WSS and other process-wide stock-CDP logging remain outside
this narrow proof. See the [security boundary](pwright-ws-logging-security-design.md)
before using sensitive endpoint or protocol values.

## Observed upstream contract

The pinned official Python implementation enters through
`playwright-python/playwright/_impl/_browser_type.py:214-296`. It asks the
local driver's `LocalUtils.connect` to create a `JsonPipe`, builds a second
`Connection` over that pipe, marks it remote, waits for the remote `Playwright`
initializer, then returns its `preLaunchedBrowser`. The returned Browser owns
that connection and stops it on `Browser.close()`. Its transport-close handler
closes Page and Context wrappers before the Browser and rejects outstanding
calls with the close reason.

The pinned Core implementation at
`playwright-core/src/server/dispatchers/localUtilsDispatcher.ts:84-119`
resolves an HTTP endpoint when needed, opens a WebSocket, and bridges JSON
messages to/from the pipe. It adds a default User-Agent and proxy header;
Python adds `x-playwright-browser` after caller headers. The pinned Core Node
header assertions at lines 235 and 250 use the same local-utils default UA
and the browser-specific header while allowing an explicit caller UA to win.
Redirect handling in
`server/transport.ts:79-125` allows one HTTP redirect and removes
Authorization/access-key headers before following it. The Playwright Server
and per-WebSocket connection in `src/remote/playwrightServer.ts` and
`playwrightConnection.ts` perform version/protocol setup and cleanup.
Dispatcher messages use command IDs, GUIDs, methods, parameters and metadata;
object lifecycle messages include `__create__`, `__adopt__`, and
`__dispose__` (`server/dispatchers/dispatcher.ts:197-228`). Python's
`_connection.py:261-348` maintains the callback and object registries and
rejects pending operations on close.

The official Python
`tests/async/test_browsertype_connect.py::test_should_print_custom_ws_close_error`
is red in the preserved pre-fix strict ledger on all three engines: the public
`connect` method was absent, while the independent oracle passed. The
post-fix ABI122 observer now passes this narrow failure path through an actual
Playwright Server WebSocket `Root.initialize` message and remote Close frame;
the fixture observed and closed the real connection before the official
assertion passed. This does not establish successful remote connection.

## Composable Pwright design to implement

1. Keep the current default-off `playwright-server-initialize` observer as a
   separate, narrow Rust leaf. The protocol core covers bounded command ID
   correlation and GUID graph transitions; the separate discovery crate now
   covers direct and HTTP(S) endpoint resolution with a five-hop HTTP redirect
   limit. The session crate now has a default-off connection supervisor. A
   full `playwright-server-client` feature still needs compatible WebSocket
   redirect/header policy and language adapters. Keep it independent of
   Chromium/CDP.
2. Reuse `pwright-transport-ws` for bounded direct WebSocket I/O. The optional
   supervisor serializes requests, events and closure above its exclusive
   mutable transport, but requires cancellation-safe peer receives. Preserve
   upgrade response headers and close reason in later public error mapping;
   the transport itself does neither endpoint discovery nor Playwright JSON
   parsing. Per-command reply deadlines and large-event streaming remain open.
3. Expand the narrow Python async/sync and Node DNS-error-only entry points into
   real remote-browser adapters.
   Keep Python's header-merge order and Node's distinct order at their
   language boundaries. The remote Browser wrapper must own the new
   connection; a local CDP Browser must not accidentally inherit that close
   policy.
4. Build the full remote public object graph behind narrow opt-in pieces.
   Start with connection handshake and clean close, then Browser/Context/Page
   lifecycle, then events, network, selectors, handles and remaining public
   methods. No test may be marked passing for a stub that merely changes an
   exception type.

## First successful-connect milestone

The pinned Core `tests/library/browsertype-connect.spec.ts` has unchanged
`run-server` cases for `BrowserType.connect` that require a real remote Browser:
connected state (lines 321-327), a `disconnected` callback carrying that same
Browser (lines 304-312), and object-form endpoint plus
`newPage().evaluate('1 + 2')` (lines 598-609). The `launchServer` fixture variant
is a separate public capability, so passing only `run-server` must be reported
as partial coverage. The pinned Python async/sync
`test_browsertype_connect.py` files contain 90 three-engine expanded IDs: nine
failure-path IDs already pass in the strict candidate ledger, while 81
successful-connection IDs remain unrun. These counts are inventory, not a
claim that the missing cases are currently supported.

The first Rust boundary now exists as a default-off connection supervisor that
continuously receives frames, correlates replies, bounds event and command
queues, rejects pending calls on terminal close, and replays the startup object
graph before publishing its handle. Language bindings still need distinct remote
Browser wrappers, with connection-local `close`, `isConnected` and
`disconnected`, before Context/Page methods. Fake-duplex and synthetic live
WebSocket tests validate this machinery but cannot earn official conformance
credit; the unchanged candidate/oracle cases above are the first public gates.

Compatibility traps for that milestone: the current Node diagnostic rejects
`timeout: 0`, `slowMo`, and `exposeNetwork`, although the pinned public
`BrowserType.connect` accepts them; the real adapter must not inherit its
whitelist. The upstream Node connection adds `x-playwright-browser` and the
LocalUtils defaults for User-Agent and empty `x-playwright-proxy`, with caller
headers overriding those defaults. Python's current Browser wrapper assumes a
CDP-native session and lacks the remote `disconnected` event surface. The
current Pwright inbound WebSocket frame cap is 256 KiB versus the pinned
upstream's much larger 256 MiB allowance, so large-payload public cases need
an explicit bounded or streamed design before full parity is claimed.

## Verification gates

- Protocol fakes: exact header precedence, one-redirect stripping, version
  rejection, root initializer, GUID creation/adoption/disposal, command
  correlation, timeout, dropped receiver, close reason and pending-call
  cleanup. Use bounded payloads and deterministic injected timing.
- Official source-pinned browser-free tests: rerun the three-engine Python
  custom-close cases in independent candidate and oracle lanes, preserving
  the unchanged assertions and strict provenance. Then expand to all
  applicable Python and Core `connect` cases, including the separate Core
  Playwright Server default-User-Agent test.
- Packaging and ownership: selected/unselected ABI proof, no Node runtime in
  the Rust core, release dependency-closure comparison, Python 3.9/3.12 wheel
  checks, and memory/cleanup tests under many connections. Only promote
  genuine official outcomes into immutable ledgers.
