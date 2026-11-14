<!-- agent-updated: 2026-10-10T17:46:09Z -->

The Python Context-binding/media checkpoint adds no broad capability bundle.
The default addon closure is anchored to 108 Pwright packages; media and
`context-forced-colors` each preserve that exact closure. Page bindings and
`context-bindings` each have the exact 109-package Page-event/Handle closure,
with only the frame-lifecycle plugin beyond the base. These five anchored
closure comparisons were executed locally, rather than only comparing two
unanchored feature graphs. Standalone and combined addon release checks,
combined host regressions and strict Clippy gates are wired into the existing
Python boundary script/CI. Their execution evidence is recorded separately
in `../python-official-conformance.md`.

The optional Node `frame-navigation-burst-20` profile changes capacity policy,
not the package closure. It remains absent from Python's addon aliases and
from the full official Node Page artifact feature list; no default concurrency
bound is silently widened. Minimal-feature compile checks also cover the
selector-registration-only Handle creation helpers, keeping that optional
world-routing branch out of Context-binding-only builds.

The new `official-conformance-page-goto-burst-20` Node artifact selects the
same 38 addon features as the existing full official Page artifact plus only
`frame-navigation-burst-20`. It introduces no Cargo feature definition or
package dependency; the burst flag reaches the existing host Frame-navigation
capacity policy through the addon feature. The default, full official and
standalone burst selections remain separate. The selected release addon
SHA-256 is
`3dfb2ac8698de70f5b2416688b2d15de879251d9d7bf5c5549474ecb9fbd5c83`.
The selected native-load, 565 Node browserless units, strict types and Node
dependency-boundary gate pass. No official Page.goto row advances from these
browserless checks.

NODE-427 makes device emulation explicit in the Node graph:
`pwright-node-addon/device-emulation -> pwright-node-host/device-emulation`.
The standalone profile proves the leaf without unrelated capabilities;
`page-viewport` and `context-api-request` declare it as a prerequisite, while
the official and Context-request-proxy artifact profiles list it explicitly.
The default addon remains device-free. Exact-profile native identity checks
require both the compiled marker and `NativeSession.newContextWithHasTouch`.

NODE-399 adds one opt-in history path:
`pwright-cap-history-navigation -> pwright-backend-cdp-navigation ->
pwright-backend-cdp -> pwright-composition-chromium -> pwright-node-host`.
`pwright-node-addon/history-navigation` and
`pwright-python-addon/history-navigation` select that host feature independently.
The default addon graphs contain no `pwright-cap-history-navigation`; the
boundary matrix proves exact propagation across the five feature-bearing
packages for standalone and frame-navigation compositions. The capability is
an embedded-host contract in this checkpoint; it does not silently add a wire
codec, client or server transport.

NODE-370 adds the independently selectable
`pwright-node-addon/bypass-csp = pwright-node-host/bypass-csp` feature. It adds
no package and selects no handles, console events, browser provider or protocol
transport. The official profile composes this leaf with its existing features;
the standalone profile proves the same 251-package addon closure as default,
with only the addon/host feature flags changed. JavaScript adds only validation
and capability selection. Rust retains the fixed Context policy registry and
owns session installation/teardown.

The checkpoint also repairs a feature-boundary defect: `NativeFrameLocator`
and console setter callbacks now live in handles-only and console-only N-API
impl blocks. Default and `bypass-csp` builds no longer require their generated
callbacks. The raw-class inventory now includes all 14 opaque classes and the
handles registry includes `NativeFrameLocator`; Dialog remains its independent
conditional class/tag. NODE-369 adds no source or graph change.

NODE-368 adds the independently selectable addon `page-routes` feature and
selects the existing `pwright-node-host/page-routes` leaf in the official
profile. It adds no package: the Rust host still owns string matching, bounded
call slots and terminal continue/fallback/abort settlement. The Node package
adds one callback-identity registry and one native watcher, not a Fetch/CDP
observer, matcher engine, request-history owner or network executor. Regex,
predicate, override, fulfill and fetch support remain outside this selection.

NODE-367 adds the independently selectable addon `page-dialogs` feature as
`page-events + pwright-node-host/page-dialogs` and selects it only in the
official profile. It adds no package, browser observer, callback registry or
queue. The addon publishes a thin `NativeDialog` over the existing Rust action
owner; Node supplies public event identity and automatic no-listener policy.

NODE-366 adds no Cargo feature, Pwright package or native-profile dependency
edge. Client-redirect lifecycle adoption and original-response preservation
remain in the existing navigation backend/host selection. The new
`navigation-error.cjs` module is presentation-only and owns no browser state,
timer, protocol client or retry loop.

NODE-365 adds no Cargo feature, Pwright package or native-profile dependency
edge. The existing `console-events` selection gains two thin addon methods
that forward bounded Page/BrowserContext identities to the already selected
Rust host subscription owner. JavaScript retains only listener identity and
mirrors the zero-to-one and one-to-zero transitions; it gains no CDP observer,
publication queue, callback registry or browser-state owner. Artifact loading
now proves the two methods as well as the existing feature marker. The owned
Context automation override reuses the existing session initializer and CDP
domain, so it adds no package edge and does not affect external/default
Contexts.

NODE-364 adds no Cargo feature, Pwright package or native-profile dependency
edge. Under the existing `handles` selection, the addon now publishes the
already present `HostFrameLocator` as a thin `NativeFrameLocator`; derived
Locators retain the iframe-owner query graph in Rust rather than pinning a
child Frame in JavaScript. The `_fetchLog` compatibility alias, matcher
one-shot timing and CSS diagnostic mapping likewise reuse existing owners and
add no Node HTTP client, selector engine, polling loop or browser provider.
The complete unchanged official boolean matcher file passes 86/86 in both
candidate and oracle lanes.

NODE-362 adds the `ApiTunnelTlsUpgrader` port to the existing transport package
and its implementation to the already-selected Rustls package. HTTP CONNECT
and SOCKS5 accept that port by injection; neither gains a package edge to
Rustls or another TLS stack. The two Node artifact profiles are exact unions:
`api-request-client-certificates + api-request-http-proxy` and
`api-request-client-certificates + api-request-socks5-proxy`. An executable
closure comparison proves each combined graph equals the union of its two
component graphs. No browser, CDP, Node HTTP client, Hyper, reqwest or second
socket/TLS owner is added.

NODE-361 adds no package, feature, ABI or runtime dependency edge. It executes
the already proven `api-request-client-certificates` closure through the six
official Bidi projects. The additional 48 candidate and 48 oracle outcomes are
coverage evidence; the exact optional certificate edge remains unchanged.

NODE-360 adds no package, feature, ABI or runtime dependency edge. It executes
the already proven HTTP CONNECT and SOCKS5 leaves through the official Bidi
project matrix. The additional 18 candidate and 18 oracle outcomes are
coverage evidence only; the exact dependency closures proven in NODE-357/359
remain unchanged.

NODE-359 adds one optional package edge from
`pwright-node-addon/api-request-socks5-proxy` to
`pwright-api-request-connector-socks5`. The new leaf depends inward only on
the existing transport connector port plus async-trait, URL parsing and
cancellation. It wraps the already selected direct connector and adds no CDP,
browser, Node HTTP client, Hyper, reqwest or second TLS implementation. The
base request, certificate and HTTP CONNECT profiles do not select it. The
executable dependency proof shows exactly this one Pwright-package delta.

NODE-358 adds no package or feature edge. The existing optional CONNECT leaf
now permits an `https:` proxy URL and delegates that outer TLS connection to
the already selected `pwright-api-request-connector-rustls`. Its exact
dependency delta over `api-request` remains the single
`pwright-api-request-connector-http-proxy` package; HTTPS target tunneling,
SOCKS and bypass remain later leaves or compositions.

NODE-357 adds one optional package edge from
`pwright-node-addon/api-request-http-proxy` to
`pwright-api-request-connector-http-proxy`. That leaf depends inward only on
the existing transport connector port plus async-trait, Base64, httparse, URL
parsing, cancellation and zeroization. It wraps the already selected direct
connector; it adds no browser, CDP, Node HTTP client, Hyper or reqwest edge.
The base `api-request` and certificate profiles do not select it. The current
monolithic addon still has its pre-existing unconditional host closure, so
this is a feature boundary rather than a minimal-binary-size claim.

NODE-356 changes no ABI, Cargo feature, Pwright package or dependency edge.
The existing `page-routes + route-redirects + network-events` Rust path still
owns effective fallback matching and native settlement. Python adds only
public header lookup and bounded identity retention: at most 64 effective
Request snapshots, matching the Rust live route-call capacity, retired on
terminal request events. Moving sync callbacks to a worker keeps the existing
single bounded bridge loop and introduces no second protocol or browser owner.

NODE-355 advances Python to ABI 96 with two exact optional feature unions.
At the host boundary, `context-api-request = context-cookies +
device-emulation + context-http-credentials + ignore-https-errors`. At the
addon boundary it also selects the existing `api-request` feature and its
bounded Rust transport. `route-fetch = context-api-request + page-routes +
route-fulfill-api-response`. The executable proof remains exactly 108 Pwright
packages for the base and each union; neither feature adds Node, a bundled
browser, a Python protocol client or a second HTTP implementation. Rust owns
cookie and settings ports, typed request preparation, response lifetime and
route settlement. Python owns only async/sync facade and callback identities.

NODE-257 advances the Python ABI to 35 for the exported native `Locator`
owner, but adds no Cargo feature, Pwright package or dependency edge. A root
selector is admitted into the existing bounded Rust query registry; chained
`locator()` uses typed `QueryCombination::Within`, and first/last/nth derive
typed recipe nodes. Python retains only a facade reference and optional
description metadata. Reads and count expectations execute through the native
owner, so no selector grammar, chaining syntax, polling or browser state moves
into Python. The executable dependency proof remains 108 base Pwright
packages.

NODE-256 adds no native ABI, Cargo feature, Pwright package or dependency edge.
`Locator.description` is immutable language-facade metadata: describing a
Locator returns a new Python wrapper over the same Page and selector and issues
no browser/native call. Typed Locator chaining remains a separate Rust-owned
capability and is not encoded in Python for this checkpoint.

NODE-255 adds no Cargo feature, Pwright package or dependency edge. Python ABI
34 adds one native `locator_expect_count` export over the already selected
`pwright-python-addon -> pwright-node-host::HostLocator` path. Rust owns the
typed collection-size read, total deadline, retry classification and bounded
sleep. Python async/sync assertions and Node `_expect('to.have.count')` retain
no polling state and issue no browser script.
Pause-specific timeout guard symbols are now conditionally compiled with
`page-pause`; strict feature-cold Clippy therefore remains clean when a binding
selects `timeout-settings` alone.

NODE-254 adds no ABI, Cargo feature, package or dependency edge. Async/sync
Locator methods adapt directly to the already selected Rust locator read,
action and evaluation routes. Collection text reads use existing typed
`LocatorRead::{AllInnerTexts,AllTextContents}` operations rather than Python-
authored browser scripts. The `:text()` repair extends the bounded Rust query
recipe and selector-runtime closure already present in the base graph.

NODE-253 adds the optional feature edge
`pwright-python-addon/layout-selectors ->
pwright-node-host/layout-selectors + locator-evaluation`. The executable
dependency proof stays at the same 108-Pwright-package feature-cold closure:
the parser/compiler is a bounded host leaf and adds no Pwright package, Node
driver, bundled browser, Python protocol implementation or browser provider.
The selected wheel reuses the existing ABI 33 browser-program count export;
only the compiled plan kind changes. Rust owns the 256-part/256 KiB program
admission and geometry scoring, while Python retains no grammar or layout
state.

NODE-252 adds no Cargo feature or package edge. Python ABI 33 extends the
already selected path
`pwright-python-addon/selector-registration ->
pwright-node-host/selector-registration + locator-evaluation`. Rust turns the
bounded immutable registry entry into a bounded selector program, installs
referenced engine source in the selected execution world and evaluates it
through the existing Chromium runtime owner. The registry admits 8 MiB of
accounted name/source payload plus fixed 64-entry structural overhead; emitted
programs are capped at 256 parts and 2 MiB. Python gains no engine registry,
selector parser, JavaScript runtime, Node driver, provider or package edge.

NODE-251 adds no Cargo feature or package edge. Python ABI 32 exports native
`validate_selector` through the already selected
`pwright-python-addon -> pwright-node-host::LocatorQuery` path. The immutable
bounded query recipe surfaces deferred unknown-engine errors before the
browser command, then releases its temporary lease. Python does not gain a
selector parser, custom-engine runtime, Node driver or browser provider.

Python ABI 31 adds the optional feature edge
`pwright-python-addon/selector-registration ->
pwright-node-host/selector-registration`. The executable dependency proof
keeps the same 108-package feature-cold Pwright closure: the registry is a
bounded host leaf and adds no Pwright package, Node driver, browser provider or
Python protocol implementation. Rust owns 64 engine entries, 64-byte names,
1 MiB per source and 8 MiB aggregate bytes; the thin Python facade retains no
registry state. Custom-engine browser execution is a later capability and is
not implied by this edge.

NODE-249 adds no ABI, Cargo feature, package or dependency edge. It executes
the async facade over the same ABI 30
`pwright-python-addon -> pwright-node-host ->
pwright-composition-chromium-locator-evaluation` path already proven by the
sync row; the separate upstream result is binding evidence, not a new
capability implementation.

NODE-248 adds no Cargo feature or package edge. Python ABI 30 exports
`eval_on_selector_all` through the already selected
`pwright-python-addon -> pwright-node-host ->
pwright-composition-chromium-locator-evaluation` path; Rust's typed selector
model already owns intersection and collection evaluation. Raising the event
item bound from 2,048 to 8,192 while independently retaining the existing
512 MiB payload ceiling is a resource-profile change, not an unbounded queue,
dependency change, Node driver, Python selector engine or recovery retry loop.
When `concurrent-close` is also selected, its 256-topology-object declaration
is combined by maximum with the 1,088 live objects required by bounded network
Request/Response ownership; neither capability may erase the other's budget.

NODE-247 adds no feature or package edge. The sync listener uses the same
`pwright-python-addon/response-events -> pwright-node-host/response-events ->
network-events` closure as the async listener. Reconnect now holds live
network adapters while the topology snapshot is incomplete and reconciles
them against the completed authoritative Page set. The host connection's
bounded event replay window changes from 1,024 to 2,048 entries to cover the
declared 16-Page/128-session/256-world profile; this is a resource-budget
change, not an unbounded queue, dependency change, or retry loop.

Python ABI 29 adds the optional feature edge
`pwright-python-addon/response-events -> pwright-node-host/response-events ->
network-events`. The executable proof shows exactly the same Pwright package
closure as `network-events`; no Node driver, bundled browser, Python protocol
client or new Pwright package is selected. Rust owns the 16-Context interest
set, 256-notice FIFO and shared 16 MiB network byte budget. When
`request-history` is also selected, the two leaves share the response tag with
explicit interest coordination instead of disabling each other.
Sticky topology event loss is recovered inside the existing host/topology
closure by one same-connection authoritative reconstruction and one epoch
advance. Materialized Page membership is a portable-identity relation and is
therefore independent of the transient command-session route; this repair adds
no dependency edge or retry loop.

Python ABI 28 adds the optional feature edge
`pwright-python-addon/device-emulation -> pwright-node-host/device-emulation`.
The executable proof keeps the same 108-package feature-cold Pwright closure:
the leaf adds no Pwright package, Node driver, bundled browser or Python
protocol client. Rust embeds the immutable 143-entry catalog and owns 16
Context slots plus 64 KiB of retained settings; Python only converts official
camel-case descriptor keys to its public snake-case shape and forwards the
bounded tuple at Context creation.

Python ABI 27 adds the optional feature edge
`pwright-python-addon/request-history -> pwright-node-host/request-history`.
The executable proof shows exactly the same Pwright package closure as
`network-events`; history reuses normalized Chromium lifecycle events and adds
no package, Node driver or Python protocol client. Rust owns 16 Page slots,
100 visible items per Page, 256 retained owners per Page, 4,096 owners globally
and 64 MiB of event bytes. The Python FFI observation lane is bounded at 256
while the unrelated command lane stays at 32.

Python ABI 26 adds the optional feature edge
`pwright-python-addon/console-history -> pwright-node-host/console-history`.
It has the same Pwright package closure as `page-events`: the feature-cold
108-package base plus `pwright-plugin-chromium-frame-lifecycle`. The history
leaf reuses Chromium console observation/parsing but does not select
`console-events`, a publication queue, a Python callback registry, or a Node
driver. Rust owns 16 Page slots, 3,200 items and 16 MiB of retained console
bytes; Python receives only bounded snapshots.

Python ABI 25 adds no Pwright package edge. The existing optional
`pwright-python-addon/page-errors -> pwright-node-host/page-errors` path now
offers a native snapshot read over Rust-owned bounded history. The leaf owns
16 Page slots, 200 errors per Page, 3,200 global items and 16 MiB of retained
error bytes; Python retains only returned public `Error` values. The
feature-cold dependency proof remains at 108 Pwright packages.

Python ABI 24 adds no Pwright package edge. The optional
`pwright-python-addon/request-gc -> pwright-node-host/request-gc` feature uses
the already-present host and CDP connection. Rust owns Page/session lookup,
command admission and `HeapProfiler.collectGarbage`; Python owns only async and
sync method shapes and Page-ID forwarding.

NODE-240's Page-route retirement fix adds no Pwright package edge. The existing
`page-routes` host leaf retains at most 64 terminal call IDs, includes their
bytes in `BackendStateBytes`, and consumes each receipt exactly once after Page
retirement. Python keeps no route-settlement state.

Python ABI 23 adds no Pwright package edge. The optional
`pwright-python-addon/page-pause -> pwright-node-host/page-pause` feature reuses
the existing timeout-settings, topology and lifecycle closure. Rust owns the
scoped Page/Context timeout reset guard and typed internal-text parsing; Python
adds no timer, browser process, protocol client, selector engine or persistent
registry. The executable proof keeps the feature-cold closure at 108 Pwright
packages.

Python ABI 22 adds one optional edge:
`pwright-python-addon/persistent-context -> pwright-node-host/owned-launch`.
Relative to the 108-package feature-cold Python closure, the executable proof
requires exactly five additional Pwright packages:
`pwright-cap-browser-lifecycle`, `pwright-codec-browser-lifecycle`,
`pwright-composition-chromium-launch`, `pwright-provider-chromium`, and
`pwright-server-browser-lifecycle`. The facade adds no Node driver, browser
binary, downloader or Python-owned launch engine. The provider owns executable
selection and profile initialization; the host owns the one noncloneable
connection/process lease and bounded shutdown sequence.

Python ABI 19 adds one optional edge:
`pwright-python-addon/drag-and-drop -> pwright-node-host/drag-and-drop`. The host
feature composes the existing `drag-runtime` and `locator-bounding-box` leaves;
relative to the 108-package feature-cold Python closure, the executable proof
shows exactly one added Pwright package, `pwright-backend-cdp-drag`. The Python
package adds no Node driver, browser binary, protocol client or language-owned
input implementation.

Python ABI 18 adds no Pwright package edge. `Request.sizes()` reuses the
already-selected `network-events` path through `pwright-node-host`,
`pwright-composition-chromium-network-events`,
`pwright-backend-cdp-network-events` and `pwright-network-model`; PyO3 adds
only an opaque owner class and one session method. The exact dependency proof
still records 108 feature-cold Pwright packages and the ABI 17 Page-route delta
is unchanged.

Python ABI 17 adds only feature edges from
`pwright-python-addon/page-routes` to the existing host and Chromium Frame
observer/session-initialization path. The executable proof keeps the
feature-cold base at 108 Pwright packages: `page-events` adds exactly
`pwright-plugin-chromium-frame-lifecycle`, and `page-routes` adds no package
beyond that closure. PyO3 and the Python facade add no CDP client, browser,
server or routing implementation.

Python ABI 16 adds only feature edges from
`pwright-python-addon/page-errors` to the existing host and Page-event
publication path. Relative to the feature-cold 108-package closure,
`page-events` adds exactly the existing
`pwright-plugin-chromium-frame-lifecycle` package; `page-errors` adds no package
beyond that `page-events` closure. The observer uses the existing CDP event,
topology and lifecycle-fence packages and adds no Python runtime dependency.

Python ABI 15 adds only the `pwright-python-addon/page-events` feature edge to
the existing `pwright-node-host/page-events` lifecycle publication leaf. The
final selected profile also has `network-events`; the host therefore composes
one observed close service with network retirement instead of rejecting the
combination or installing parallel observers. PyO3 and the Python facade add
no CDP, lifecycle-registry or browser-process dependency.

Python ABI 14 adds only the `pwright-python-addon/page-bindings` feature edge
to the existing host binding owner. It adds no Python execution engine or
browser-owned registry.

Python ABI 13 adds feature edges from `pwright-python-addon/handles` and
`wait-for-function` to their existing `pwright-node-host` leaves. Because the
host package edge is already present, the executable dependency proof shows no
new Pwright package above the 108-package base. It adds no Python runtime
dependency and no language-owned browser or polling engine.

Python ABI 12 adds only a feature edge from
`pwright-python-addon/init-scripts` to the already-present
`pwright-node-host/init-scripts` leaf. It adds no package to the ABI 11
dependency closure and no Python runtime dependency. Page and BrowserContext
facades call two PyO3 methods backed by the same Rust registration store and
Chromium session-initialization plugin. The dependency proof therefore keeps
the feature-cold base and API-request/certificate deltas unchanged.

NODE-223 adds one optional Pwright package edge from the Node addon to
`pwright-api-request-client-certificates`. That leaf implements the existing
Rustls identity-provider port and owns PEM/PFX decoding; the base request owner,
HTTP codec, transport and Node facade do not depend on OpenSSL. The executable
profile-delta proof requires exactly that one Pwright package plus
`foreign-types`, `foreign-types-shared`, `openssl`, `openssl-macros` and
`openssl-sys`, with no package removal. The profile remains browser/CDP cold.
Python's ABI 11 certificate profile has the same exact delta: one additional
Pwright leaf plus `foreign-types`, `foreign-types-shared`, `openssl`,
`openssl-macros` and `openssl-sys`. The repaired wheel vendors the two OpenSSL
dynamic libraries and rewrites the extension to load them relative to the
wheel rather than from Homebrew.

Python ABI 10 added one optional addon edge set from
`pwright-python-addon/api-request` to the same browser-independent request
owner and Basic, cookie, transport, Rustls and decompression leaves selected by
Node. PyO3 converts tuples, bytes, cookies and awaitables only; it does not add
an HTTP client, CDP, browser or Python networking dependency. The wheel profile
appends `api-request`; ABI 11 appends the separately selectable certificate
leaf, while the base addon remains feature-cold. The executable
dependency proof keeps the base closure at 108 Pwright packages and requires
the selected profile to add exactly the eight API-request packages with no
base-package removal.

NODE-221 adds optional addon edges from `pwright-node-addon/api-request` to the
base request owner plus the selected Basic, cookie, transport, Rustls and
decompression leaves. The Node facade converts public values and performs
storage-state file I/O; it does not own an HTTP client, cookie jar, retry loop,
socket, TLS state or browser. Resolver output crosses as bounded IP values;
Rust keeps the original URL host authoritative for Host/SNI. Origin state is
retained as bounded JSON by the native context and cookies remain in the Rust
store. The `api-request` profile is browser-feature cold, but the current
monolithic addon still has an unconditional package edge to
`pwright-node-host`; splitting that package closure remains separate work and
no feature-minimal binary-size claim is made.

NODE-220 adds only the existing workspace `futures-util` edge to
`pwright-api-request-connector-tokio`. One call-owned `FuturesUnordered` races
TCP attempts; it spawns no task and is dropped with every losing future when a
winner or cancellation is observed. Rustls already depends inward on this TCP
leaf and automatically reuses the policy. No transport, HTTP codec, proxy,
browser, CDP, Node or Python edge is added.

NODE-219 changes no package or dependency edge. The final-head event remains in
`pwright-api-request-http1`; `pwright-api-request-transport` alone owns the
pending connection, decoder and fixed read buffer while it selects discard or
completion. Cookie and authorization behavior still enter through base ports.
No browser, CDP, Node or Python edge is added.

NODE-218 adds `pwright-api-request-auth-basic -> pwright-api-request` plus
Base64 and zeroization. The base owns only an authorization-provider port;
transport depends only on that port in normal builds and uses the Basic leaf as
a dev-only integration proof. The leaf has no HTTP codec, connector, TLS,
proxy, CDP, browser, Node or Python edge, so deployments without Basic
credentials do not select its encoding or retained credential state.

NODE-217 adds `pwright-api-request-cookie -> pwright-api-request` with
`httpdate` and URL parsing only. The base crate declares the store port and
structured cookie values; it does not select this implementation. Transport
depends only on the port in normal builds; its cookie-leaf edge is dev-only for
integration proof. The cookie leaf has no connector, TLS, decompressor, CDP,
browser, Node or Python edge. A future BrowserContext adapter depends inward on
the same port rather than adding Chromium ownership to the global store.

NODE-216 adds three optional leaves. `pwright-api-request-connector-tokio`
depends on the connection contracts plus Tokio networking only.
`pwright-api-request-connector-rustls` depends on that TCP leaf, Tokio Rustls
and WebPKI roots; selecting plain TCP does not select TLS. The separate
`pwright-api-request-decompress` leaf depends on the transport decoder port,
pure-Rust `flate2` and Brotli; selecting HTTP transport does not select
compression codecs. None has a CDP, browser, Node or Python edge. `rcgen` is a
Rustls-leaf dev dependency only for its in-memory handshake proof.

NODE-215 adds `pwright-api-request-transport -> pwright-api-request-http1 ->
pwright-api-request`. The policy package also uses async traits, Tokio only for
the cancellable retry delay, `tokio-util` cancellation and URL parsing. It has
no concrete DNS, socket, TLS, proxy, decompressor, cookie, CDP, browser, Node
or Python dependency. Those implementations enter through `ApiConnector` and
`ApiConnection`, so selecting raw HTTP policy does not select a network stack.

NODE-214 adds one independently selectable `pwright-api-request-http1` package.
Its direct normal edges are only `pwright-api-request`, `httparse`, `thiserror`
and URL parsing. It owns request/response wire bytes but no socket, executor,
DNS, TLS, proxy, decompressor, cookie jar, CDP, browser, Node or Python edge.
Keeping the raw HTTP/1 codec outside the base owner lets a distribution omit
network execution, and keeping connector/TLS policy out of the codec lets
future runtime stacks reuse exact raw-header semantics without inheriting each
other.

NODE-213 adds only `mime_guess` to `pwright-api-request` for Playwright-shaped
multipart file content types. Query/form encoding reuses the existing URL
dependency. The crate still has no reqwest/Hyper, socket runtime, CDP,
Chromium, Node or Python edge. Header overlay, body serialization and
deterministic multipart-boundary allocation happen before the injected
transport port and add no task, queue or global owner.

NODE-212 adds one independently selectable `pwright-api-request` package. Its
normal closure contains `async-trait`, `thiserror`, Tokio synchronization,
`tokio-util` cancellation and URL parsing; it has no edge to reqwest/Hyper,
CDP, Chromium composition, browser ownership, Node or Python. Language bindings
will depend inward on this crate only when their API-request profile is
selected. A future HTTP backend implements `ApiRequestTransport` in a separate
optional crate, and BrowserContext cookie sharing enters through a separate
adapter rather than adding Chromium to the base owner. The current crate owns
one semaphore, cancellation tree and bounded response registry; it creates no
task or browser process.

NODE-211 adds no package, Cargo feature, protocol method, browser owner, task,
queue, timer or persistent registry. The existing
`pwright-node-addon -> pwright-node-host ->
pwright-composition-chromium-locator-evaluation` edge carries one Rust-selected
bounded preview program only after an expectation mismatch and only inside the
original deadline. The addon projects fixed result fields; Node formats public
matcher text and has no DOM/CDP edge. The official runner remains blocked by
its independent empty-browser safety gate until the restored unrelated page is
gone.

NODE-210 adds no package, Cargo feature, protocol method, browser owner, task,
queue or timer. Checked, editable and enabled expectations reuse the existing
`pwright-node-addon -> pwright-node-host -> pwright-cap-locator ->
pwright-backend-cdp-locator` reads. Indeterminate and focused reuse the
existing host Locator-evaluation path with Rust-selected bounded programs;
Node performs only public option/result conversion. The official oracle
preload and the loopback `/json/list` empty-browser guard are test
infrastructure, not production dependencies. The selected conformance profile
continues to compose existing leaves only.

NODE-209 adds no package, Cargo feature, protocol method, browser owner, task,
queue or timer. `pwright-runtime` gains a default-noop, synchronous/no-I/O
terminal-error projection after successful typed cancellation. The existing
exact route remains `pwright-node-addon -> pwright-node-host ->
pwright-composition-chromium -> pwright-backend-cdp-locator`. That Locator
service owns one fixed last-actionability enum and consumes it once after an
original-deadline timeout; the composition only forwards the hook for the
matching route. Node maps fixed bounded diagnostic keys to public error text
and gains no browser/protocol dependency.

NODE-208 adds the direct optional edges
`pwright-node-addon/locator-bounding-box ->
pwright-node-host/locator-bounding-box` and
`pwright-node-addon/locator-highlight ->
pwright-node-host/locator-highlight -> locator-bounding-box`. Both reuse the
existing Locator evaluation, exact Page/Frame topology and CDP runtime path;
neither adds a package, protocol method, browser owner, task, queue or registry.
Rust selects both browser programs and owns strict resolution, geometry,
same-origin Frame offset traversal and overlay lifetime. Node keeps only
public descriptions, option/result conversion and error display.

NODE-207 adds no package, Cargo feature, protocol method, browser owner, task,
queue, timer or persistent table. Locator state, property and regular-expression
text expectations reuse the existing
`pwright-node-addon -> pwright-node-host -> pwright-composition-chromium-locator-evaluation`
path and exact Page/Frame/selector registries. The addon converts one bounded
expected value before async dispatch; Rust selects the browser program, owns
serial polling, deadline/context-replacement semantics and result matching.
Node keeps only public Locator descriptions and language-level error display.

NODE-206 adds the direct optional edges
`pwright-node-addon/response-body -> pwright-node-host/response-body ->
pwright-cdp`. The leaf reuses the selected Frame-navigation result and exact
Page/Frame registries; it adds no browser owner, task, timer, queue, event
listener or persistent state table. Rust retains the completed document's
bounded request identity, resolves the Page-scoped CDP session, chooses
`Network.getResponseBody`, decodes base64 and owns the bounded bytes through
FFI conversion. Node performs no protocol or browser routing and retains only
one cached copied Buffer for `body`, `text` and `json` language semantics.

NODE-205 adds no package, Cargo feature, protocol method, browser owner, task,
queue or retry loop. Both legacy and v2 click scripts use the existing
`pwright-backend-cdp-locator -> pwright-backend-cdp-selector-runtime` edge for
enabled-state retargeting. The original matched element still owns stability,
geometry and hit testing. `FrameDetached` is only a typed Rust host error
projection variant and crosses the existing addon error port with the same
`PWRIGHT_TARGET_CLOSED` class. Node adds no browser logic for either change.

NODE-204 adds no Rust package, Cargo feature, protocol method, browser owner,
timer, retry loop or state table. Page and Frame action facades now supply only
their public API name to the existing Node Locator adapter; the typed action,
timeout and browser work keep the existing
`pwright-node-addon -> pwright-node-host -> pwright-cap-locator ->
pwright-backend-cdp-locator` path. Rich actionability timeout progress remains
missing and is not synthesized in Node.

NODE-203 adds no package, Cargo feature, Node API surface, browser owner, task
or queue. The already selected recursive-attachment profile now admits an
exact five-owner paused-popup startup: caller Context settings, lifecycle
observer setup, `Page.enable`, recursive auto-attach and debugger resume are
polled together, with the first four branches queued before resume and every
reply retained as a publication fence. The later Frame-tree/runtime/utility
registration join remains a separate three-command phase. The existing
selector-runtime edge advances to version 8 and gains descendant text plus
submit/button input-value semantics; quoted text shorthand and `>>` become
bounded typed recipes in the existing host parser. The selected profile
reserves the corresponding five pending CDP command owners and, relative to
the serial profile, exactly `16 * max_response_bytes` of declared response
expansion. This is bounded admission, not preallocated RSS.

The Node conformance artifact
`official-conformance-page-bindings` selects exactly `set-content`,
`page-events`, `console-events`, `page-bindings`, `handles`,
`frame-navigation`, `init-scripts`, `timeout-settings`, `wait-for-function`,
`wait-for-url`, `wait-for-timeout`, `large-evaluation-results`, `emulate-media`,
`request-gc`, `response-body`, `extra-headers`, `api-request`, and
`context-api-request`. The Context request edge reuses the same Rust request
stack while adding the live Chromium cookie/settings bridge required by
`page.request`; it is not a second transport. `emulate-media` is one direct
addon-to-host edge and adds no production package; its fixed state and CDP
work remain in the host.
`extra-headers` reuses its existing optional addon/host/network composition and
does not add a second header path. Navigation waiting uses the already selected
`frame-navigation -> frame-lifecycle` edge, and Page click/type/focus add no
Cargo feature because they reuse typed Locator actions. The profile still adds
no file-input, drag, or concurrent-close dependency.

`pwright-node-addon/wait-for-url -> pwright-node-host/url-wait ->
frame-lifecycle`. The host leaf already existed for the Python binding; Node
adds no protocol/backend package. Rust owns the event wait and accepts an exact
Page-owned Frame id. N-API contributes only a bounded thread-safe matcher port
so JavaScript RegExp and URL-predicate semantics remain language-correct.

`pwright-node-addon/request-gc -> pwright-node-host/request-gc`. The host leaf
uses the already selected CDP client and Page/session registry; it adds no
package edge, persistent state, task, timer or queue. Rust owns the exact
Chromium `HeapProfiler.collectGarbage` command and empty payload. Node owns only
the public Page method facade.

Python ABI 9 selected
`wait-for-url+set-content+emulate-media+timeout-settings+network-events`.
`timeout-settings` forwards from `pwright-python-addon` to the same-named host
leaf and adds no package edge. `network-events` forwards through the addon and
host to the existing Chromium network composition; the host's direct optional
edges are `pwright-backend-cdp`, `pwright-cap-network-events`,
`pwright-network-model`, and `pwright-runtime-control`. The Python language
adapter adds no browser/protocol package and owns only typed value conversion
and predicate invocation. The base addon's exact dependency closure remains
separately checked.

The historical bridge test clients now share
`pwright-fake::CdpCallLog` instead of maintaining two call-recording paths.
Their compatible persistent optional overrides share `CdpResponseSlot` without
moving defaults, strict policy, queues, errors, or async gates into that type.
The stateful fake's 11 configurable Runtime entry points also share
`CdpResponseScript` for ordered errors, one-shot responses, and persistent
fallbacks. One lock owns the complete script so configuration and consumption
cannot observe partial cross-queue state; method defaults/gates and the canned
mock's strict policy remain separate. The canned mock now rejects every absent
configurable response slot in strict mode while retaining its fixed built-in
responses and no-op methods.
This support remains selected: `pwright-bridge` has a four-package ordinary
Pwright closure (`bridge`, `cdp`, `core`, `js`), while
`pwright-bridge/test-utils` adds exactly `pwright-fake` as the fifth package.
`scripts/check-bridge-test-utils-dependencies.sh` checks both exact closures and
both release builds, preventing test infrastructure from entering production
by accident. Canned-response and stateful DOM behavior remain separate.

NODE-004 begins with the command-free session-initialization leaf, the
synchronous Frame event/metadata projection leaf, and the bounded Frame
lifecycle/navigation-wait leaf. The first two exact normal Pwright closures
contain five packages each; the lifecycle closure contains ten packages. The
aggregate compatibility-reexports all three contracts, but Node settings,
observer, and lifecycle adapters consume the plugins directly, reducing
aggregate source coupling from 55 to 39 host files. These transitional package splits make the current default
host/addon closures 236/251; they are ownership-boundary steps, not yet a
binary-size or dependency-count reduction. Dedicated gates prevent the direct
host implementations from returning to the aggregate.

NODE-190 adds
`pwright-node-addon/retained-serialized-payload-import ->
retained-call-argument-serialization +
pwright-node-host/retained-serialized-payload-import + dep:serde_json` and
`pwright-node-host/retained-serialized-payload-import ->
retained-target-prevalidation`. `serde_json` was already in the workspace and
becomes an optional direct addon dependency only for this leaf. No backend,
composition, CDP, runtime or new package edge is selected.

Exact standalone/combined fingerprints prove that removing the host and addon
import flags yields the corresponding NODE-189 graph. Fixed artifact paths and
the compiled marker reject older/mislabeled binaries; every prior profile
remains unselected. [Canonical NODE-190 boundary](pwright-node-handles.md#node-190-bounded-rust-ownership-for-the-serialized-payload-graph).

NODE-189 adds only
`pwright-node-addon/retained-call-argument-serialization ->
retained-input-instrumentation`. It adds no host, backend, composition, browser
or transport package/feature edge. Standalone and combined Cargo-tree
fingerprints must equal the corresponding instrumentation graphs after removing
this one addon flag.

NODE-188: `pwright-node-addon/retained-input-instrumentation` selects exactly
its existing `retained-target-prevalidation` prerequisite. There is no matching
host, backend or composition feature and no package/dependency edge. Exact
standalone/combined fingerprints prove that removing this one addon flag yields
the prior prevalidation graph. New fixed artifact paths and compiled identity
markers reject older or mislabeled binaries; all older profiles remain exact.
[Canonical NODE-188 boundary](pwright-node-handles.md#node-188-awaited-input-action-instrumentation-boundary).

NODE-187: New default-off `retained-target-prevalidation` flags exist only on
the addon, host and file-input composition. Addon selects its existing marking
profile plus the host leaf; host selects marking plus the composition leaf;
composition selects the existing `retained-target-marking` and
`retained-preflight` primitives. The backend therefore gains only its existing
`retained-preflight` flag. No package or dependency edge is added.

Standalone/combined exact fingerprints prove that removing those five resolved
flags produces the prior marking graph. Older artifact paths and compiled
identity markers remain exact; the two new artifact paths reject both older
binaries and prevalidation binaries mislabeled as marking profiles.
[Canonical NODE-187 boundary](pwright-node-handles.md#node-187-same-owner-retained-dom-prevalidation).

NODE-186: No package, dependency edge, Cargo feature or Node artifact path is
added. The retained-owner result, `Projected` state, ten-root selected maximum
and N-API projection method compile only under the existing
`retained-target-marking` feature. Earlier profiles preserve their prior graph,
six-root maximum and Materialized stop behavior. Exact standalone/combined
boundary fingerprints, default addon check and strict Clippy all pass.
[Canonical NODE-186 boundary](pwright-node-handles.md#node-186-retained-marked-owner-and-same-ticket-carrier-projection).

NODE-185: No Cargo package, feature flag, Node artifact path or resolved graph
changes. The call-ID counter, retained record field and N-API methods compile
only under the existing addon `retained-target-marking` selection. The JS
progress/dispatcher code is present but reaches these methods only when that
selected artifact exposes them; older profiles retain their two-gate
Materialized behavior. Exact boundary fingerprints pass for standalone and
combined graphs, and strict Clippy passes both existing selections.
[Canonical NODE-185 boundary](pwright-node-handles.md#node-185-session-call-identity-and-one-node-deadline).

NODE-184: `pwright-node-addon/retained-target-marking` explicitly selects only
its existing `retained-file-input` prerequisite and
`pwright-node-host/retained-target-marking`. Resolved graphs add the same-named
feature only on addon, host, file-input composition and file-input backend, with
no package addition. Standalone and all existing combined selections are checked
by exact normalized fingerprints. Existing default, handles, file-input and
retained-file-input artifact profiles remain unchanged; new standalone/combined
marker artifacts have independent fixed paths and compiled identity checks.
[Canonical NODE-184 boundary](pwright-node-handles.md#node-184-same-admission-node-target-marking-handoff).

NODE-183: The existing Rust host crate adds a default-off
`retained-target-marking` feature selecting its `retained-file-input` capability
and the existing file-input composition marker feature. No new package,
dependency cycle, addon feature or Node artifact profile is introduced. Default
and older selections do not acquire this cache or its budgets. Standalone and
combined host tests/strict Clippy, full composition tests/strict Clippy and the
older retained host regression pass. Default host tests/strict Clippy,
independent standalone/combined marker dependency proofs, runtime-JS/addon
boundary checks and formatting also pass; exact evidence is canonical.

The selected host admits 64 current world entries plus eight retiring
predecessors and eight calls. Persistent marker owners receive 72 additional
shared utility-snapshot slots, with independent single-waiter retirement
receipts rather than borrowing per-call Handle/work ownership. Cache storage,
native state/call allowances and cleanup reservations remain separately charged;
these wrapper bounds do not measure arbitrary DOM retention or browser RSS.
The exact-Frame utility accessor itself performs no browser work or readiness
wait. Node typed progress, deadlines and public retained upload remain unwired.
[Canonical NODE-183 contract and verification](pwright-node-handles.md#node-183-host-owned-utility-world-marker-cache).
Earlier checkpoints retain their historical dependency and verification scope.

NODE-182: Existing file-input backend/composition crates add default-off
`retained-target-marking` flags. The backend selects the Locator backend's
`retained-reference` feature for settlement-owner validation; composition selects
the marker backend and Locator composition's retained-reference owner. No new
normal dependency package, implicit executor, host/addon feature selection,
browser launch or Node API is introduced. `pwright-runtime-tokio` is a new
composition dev-dependency only, used by the isolated native ownership fixture.

Dependency gates distinguish the standalone marker selection from marker plus
retained-preflight/large-commands and preserve default/older selections. Final
standalone/combined resolved-feature proofs, tests and strict Clippy pass;
default regressions and addon boundary checks also pass. CI includes backend/composition
release tests and strict all-target Clippy for both the standalone marker and
combined marker/preflight/large-command selections.

The owned-world primitive does not wire a world registry, scheduler, original
deadline/call ID or typed native progress into Node preparation. Its separate
resource/cleanup ownership does not enlarge existing host/addon budgets.
Completed evidence and remaining gates belong to the
[canonical NODE-182 checkpoint](pwright-node-handles.md#node-182-persistent-native-target-marking);
historical NODE-181 and earlier notes below retain their original scope.

NODE-181: Selected addon preparation now captures the actual NativeHandle's
inert Weak state binding and propagates it through existing records/snapshots.
No strong native owner, root, capacity, budget, package or feature is added.
The enlarged actual registry layout remains checked against 16 KiB. The same
record/snapshot overlap sum is compile-checked against both the existing 1 MiB
entry allowance and independent 16 KiB synchronous phase scratch, covering
temporaries after reentrant entry revocation without increasing either budget.
Weak tombstone backing remains outside active-lease reclamation claims.

The preparation still holds a unit FFI result and stops at Materialized; there
is no Weak upgrade, native phase or marker. Final frozen-source live runs pass
20/20 standalone, 20/20 combined and 20/20 standalone repeat: twenty distinct
cases, sixty passes through Materialized/abort, with owned cleanup fully retired
and no failures/cancellations/skips. This is not native Weak/GC proof.
Completed offline, Rust, strict Clippy and dependency gates route to the canonical
record rather than extending the browser claim.
Operation deadlines before materialization, connection-wide protocol IDs and
persistent exact-world remote-marker ownership remain next design work.
[Canonical contract and verification](pwright-node-handles.md#node-181-original-native-wrapper-binding).

NODE-180: The existing retained-file-input host selection adds an eight-slot
weak invocation registry, with 4 KiB BackendStateBytes and one fixed scope/lease
at extension(169,2). No package or feature is added; final dependency evidence
belongs to the canonical checkpoint and is pending here. Default-profile
shutdown also moves cancellation/tracker wakeups outside the session admission
lock, using a state barrier; Handle owner close similarly wakes outside its lock.

Host registration/check bind exact session and weak source provenance, not
native liveness. The intended registration point is the future first native
handoff; the existing 32 language preparations are unchanged and do not register.
No addon typed-progress promotion, original NativeHandle weak binding, marking,
public retained upload or browser proof is added. Stale Weak backing is outside
active-lease reclamation claims.
[Canonical contract and verification](pwright-node-handles.md#node-180-session-bound-revocation-before-native-drains).

NODE-179: Same-entry park/resume and weak per-entry revocation remain under
the existing retained-file-input host selection. The selected addon adds the
Rust raw-carrier projection diagnostic and frozen language primitives, not a
new package, feature, artifact profile, raw class, driver or browser server.
Existing preparation root and lane limits are unchanged because production
ticket integration is still absent. Additional selected entry bookkeeping is
not a measured RSS bound. Default-off native prevalidation is still unselected
by every host/addon profile; the exact dependency gates remain authoritative.
[Canonical contract and current verification](pwright-node-handles.md#node-179-same-admission-ffi-handoff-and-raw-carrier-projection).

NODE-178: Existing backend/composition file-input crates add default-off
retained-preflight features. The backend adds only its own feature; composition
adds both preflight flags and the two locator-resolution retained-reference
flags. No normal package, transport/runtime feature, Node, filesystem or executor
leaf is added. The file-input gate compares complete resolved normal feature
fingerprints with defaults and large commands; the host/addon gate rejects
retained-preflight across existing selections. The native precheck owner does
not imply Node integration or a rebuilt addon artifact.
[Canonical contract and verification](pwright-node-handles.md#node-178-owned-native-file-input-prevalidation).

NODE-177: Registration gating stays inside the existing selected retained-file-input
host/addon boundary: exact owner ticket and creation Page, not world/Handle/Frame
runtime state. Two Rust phase gates compose with a thin setImmediate/empty-await
language driver; no package, feature, profile, dependency, class, root, budget or
native pin is added. The driver returns undefined at internal Materialized,
not a public upload result. Later target/DOM/utility-serializer ordering must be
designed before typed source conversion and native submission are connected.
Offline evidence and remaining browser gates belong to the canonical record.
[Canonical contract and verification](pwright-node-handles.md#node-177-deferred-registration-gates-before-materialization).

NODE-176: Server validation/materialization remain inside retained-file-input.
The safe host re-exports its existing pinned MIME provider, with exact-extension
lookup and a table-derived stack bound. No package, feature, profile, class,
JS browser driver or transport dependency is added.
[Canonical contract and current evidence](pwright-node-handles.md#node-176-server-direction-validation-and-raw-payload-materialization).

NODE-175 (2026-09-17): Synchronous raw-buffer channel validation remains
inside the existing selected retained-file-input addon boundary. No feature,
artifact profile, dependency package, raw class, driver or server is added.
Rust owns schema order and validation policy; frozen JavaScript ports provide
individual language primitives. ArgumentsReady becomes ChannelReady by
replacing root six, without extra root/FFI scratch/metadata budgets or promotion.

The schema covers payloads (name, optional mimeType, buffer), localDirectory,
directoryStream, localPaths, streams and timeout, preserving optional fields,
observable map/raw results, raw boxed valueOf results, inherited-setter
assignment and Buffer identity. There is no byte-copy/serializer dependency.
Three no-capture/ZST native callback specializations use static typed dispatch
and current Env/path/ports, not retained scoped N-API values or registry/native
authority. This avoids new Rust callback-Box allocation and its construction-
failure leak; N-API/V8 callback backing and arbitrary escaped JS graphs remain
outside administrative RSS bounds. Normal non-isUnderTest mode is selected.
No genuine WritableStream producer/brand
exists: its first `_object` read is observable and validation fails closed.
Valid stream channels remain a separate missing capability.

Public error prefix/stack, deferred server bytes/path preparation, float
deadlines, typed move-only FfiPreparation promotion/native authority and public
retained upload remain open. Offline verification is recorded in the canonical
checkpoint; actual-ticket/browser/public gates remain open. Shared CDP is
blocked without restart permission and no new browser pass is claimed.
[Canonical contract and exact evidence](pwright-node-handles.md) own verification;
earlier dependency statements remain historical.

NODE-174 (2026-09-17): Canonical session Context/Page timeout indexes and
immutable Handle creation-Page bindings remain inside the existing selected
retained-file-input boundary. No feature, profile, dependency package, raw
class, browser driver or server is added; older profiles retain their original
Handle-creation path. Published membership plus exact Weak<Owners> provenance
bind production metadata. Rust propagates it through every derived Handle
producer and preparation; argument assembly no longer takes late settings.

The JS addition captures a frozen raw debug tuple at import and forwards
creation Page ID, with no policy query or JS-held settings. Rust selects the
canonical Page/Context chain after FFI admission and defers a lookup error
through normal source/argument materialization before host work. Idle wrappers
allocate no policy binding during pre-commit lifecycle preparation. Rust owns
undefined-only environment precedence,
inspector exclusions and policy validation. Metadata slots are shared by cells
and fresh scoped bindings (1024 at 512 bytes), with 16 KiB administration and
sixteen 16 KiB session indexes: 784 KiB total. Scoped clones retain neither the
index nor HostSession/native-use/task authority; existing Handle/FFI admission
covers their embedded storage. This increases resources, not graph selection.

Public default setters/arbitrary JS state, channel/error conversion, deferred
Buffer/path materialization, native deadline/submission integration and public
retained upload remain open. [Canonical contract and exact verification](pwright-node-handles.md#node-174-canonical-creation-page-timeout-binding)
own remaining gates; this note claims no new browser pass. Earlier dependency
and association statements below are historical checkpoints.

NODE-173 (2026-09-17): The existing selected `retained-file-input` boundary
adds Rust argument assembly and live client-timeout metadata; no feature,
artifact profile, package, dependency, driver or server is added. Narrow JS
spread/member-read ports supply language behavior while Rust chooses converted
then options then timeout order, without another conversion-continuation await.
The safely published, module-local internal `NativeFileInputTimeoutSettings`
raises the selected raw-class inventory to ten; older profiles are unchanged.

The host metadata module is selected only by retained-file-input. Its independent
1024-cell budget reserves 512 bytes per cell plus 16 KiB factory administration
(528 KiB total, 1025 leases, one scope), with no Node roots, session/native
authority or task. Factory administration survives through the final cell
lease. This is added resource selection, not a new dependency; existing
six-root/32-preparation/512 KiB synchronous-phase limits stay unchanged.

Only typed numeric defaults and the normal Page-to-Context chain are modeled.
The caller still selects creation-Frame settings; automatic association,
production facade/default/debug wiring and public setters are absent.
Channel/error validation, deferred Buffer/path work, native deadline-domain
mapping, promotion/submission and public retained upload remain open.
[Canonical contract, dependency evidence and remaining gates](pwright-node-handles.md#node-173-rust-owned-argument-assembly-and-live-timeout-settings)
own exact verification. Earlier entries retain their historical selection scope.

NODE-172 (2026-09-17): The existing selected retained-file-input boundary adds
the Rust synchronous preflight kernel, weak registry phase guards and a frozen
table of single-expression JavaScript language ports. Rust retains sequencing,
classification and size policy; JavaScript supplies member calls/operators and
the explicit owner/conversion await boundaries. Context stays outside async
conversion, whose result is an ordinary `{ payloads }` carrier.

Registry `RootSnapshot` inspection is pure Rust (weak source metadata and an
`Rc<NodeReference>` clone). N-API reference lookup occurs after the borrow ends,
and the temporary Rc drops before user JavaScript. The same selected registry
now separately admits at most 32 synchronous construction/replacement phases
at 16 KiB each, a 512 KiB administrative increase with no new dependency or
native owner. Initial `creation_phase` and later phases reserve scratch before
N-API allocation and recheck liveness after callbacks; replacement rejects
revocation. Weak scope/registry guards retain scratch through callback
abort/finalization until synchronous return, not arbitrary Promise waits, with
no FFI Command/task authority. Existing 32-entry/16-scope capacity, 16 KiB table,
1 MiB entry allowance and shared 64 MiB FFI pool stay unchanged; the total
selected resource budget does not. Arbitrary V8 graphs, scoped handles and
opaque allocator backing are not byte-bounded by these reservations.

No feature, dependency package or artifact profile is added. The two NODE-171
profiles remain the explicit selection, and thirteen older profiles stay
unchanged. Stateless selected diagnostics use the same kernel without
session/native authority or persistent roots; their differential harness is
not actual-ticket N-API or browser verification. Paths remain classified then
explicitly unsupported; byte/path materialization, options/channel validation,
promotion/submission and public `ElementHandle.setInputFiles` are not exposed.
[Canonical contract, exact dependency evidence and remaining gates](pwright-node-handles.md#node-172-rust-owned-preflight-and-reentrant-language-phases)
own verification. Earlier entries retain their historical selection scope.

NODE-171 (2026-09-17): The addon adds a default-off `retained-file-input`
feature implying `handles`, `file-input` and
`pwright-node-host/retained-file-input`. Two new artifact profiles select it:
`retained-file-input` and `retained-file-input-combined`; the latter also
selects concurrent-close, drag-runtime, frame-navigation and extra-headers.
The thirteen pre-existing artifact profiles remain unchanged and must not
implicitly enable the new selection.

The boundary is the Node-thread preparation registry and module-local
`NativeFileInputPreparation`, reusing host FFI preparation and weak Handle
source identity. It adds no browser driver or JavaScript browser policy.
Registry admission roots receiver/files/options without conversion or native
work; public `ElementHandle.setInputFiles`, promotion and submission remain
unwired. [Canonical contract, exact dependency evidence and remaining gates](pwright-node-handles.md#node-171-session-bound-node-preparation-registry)
own verification; this routing record makes no browser-pass claim. Earlier
entries below retain their historical feature/artifact scope.

NODE-170 (2026-09-17): `FfiPreparation<T>` is gated by the existing host-only
`retained-file-input` feature. It reserves the existing FFI lane/byte allowance
without a running-task token; atomic start promotes the same reservation into
execution. No addon selection, public Node API, artifact, dependency package or
feature is added. Its weak exact-budget provenance binds the shared environment,
not an individual session. A Node preparation registry, session-scoped revocation,
type-tagged tickets and complete ingress still require separate integration.
[Canonical NODE-170 contract and verification](pwright-node-handles.md#node-170-revocable-ffi-preparation-before-execution)
records the pending Rust gates and offline-only oracle evidence; this source
checkpoint makes no browser or native candidate pass claim. Earlier entries
remain historical checkpoints.

NODE-169 (2026-09-17): The host-only retained-file-input convenience feature implies handles plus file-input and exactly three retained-reference leaf selections: Handle evaluation, Locator-resolution composition and its backend. It adds no normal packages versus handles,file-input. Existing host/addon profiles must remain unselected; no addon retained-upload feature is exposed.
[Canonical contract and evidence](pwright-node-handles.md#node-169-safe-host-retained-upload-and-pinned-ingress-oracle).

NODE-168 (2026-09-17): The locator-resolution backend and composition add
default-off retained-reference features. Their normal closures remain the same
five/seven Pwright packages and the selected entrypoint introduces no Handle,
upload, Node or browser dependency. The dependency gate compares complete
normal closures with and without selection and rejects implicit activation.
Runtime attempt storage is separately opted in via with_retained_reference;
no Node artifact silently enables the feature. [Canonical contract and current evidence](pwright-node-handles.md#node-168-group-owned-retained-adoption-and-deferred-upload-ingress).

NODE-167 (2026-09-17): Current DOM ownerFrame uses the existing handles profile
and optional handle-value leaf API. Direct raw object-ID DOM description adds
one explicit CdpClient method with real/Fake implementations and a fail-closed
custom-client default, not a new dependency, transport or driver. Temporary
RemoteHandle admission is selected at the host boundary; default profiles stay
Handle-free. Combined retained-upload artifacts remain unimplemented.
[Canonical contract and evidence](pwright-node-handles.md).

NODE-166 (2026-09-17): Node property batches reuse the existing handles feature
and native-creation leaf selection. Raw property settlement, batch staging,
owned host publication and Node conversion add no dependency package or feature
edge, browser launcher or process transport. Default profiles remain Handle-
free; selected resource changes are explicit rather than hidden in the base.
[Canonical contract and verification](pwright-node-handles.md).

NODE-165 (2026-09-17): Explicit raw retained-result acquisition and
release-to-settlement methods extend the existing CdpClient boundary, with
real and fake implementations and fail-closed defaults for custom clients.
The existing Handle backend selects them; there is no fallback to lossy
ordinary evaluation, and ordinary CDP methods are unchanged. Borrowed-ID
compensation remains in the existing backend and retains existing admission.
No package, feature edge, portable schema, Node API or JavaScript browser-policy
dependency is added. [Canonical verification](pwright-node-handles.md) records
the exact checked selections; this is not evidence that pending property
settlement/admission, unaddressable-ID group ownership, Node batching or the
full compatibility corpus is complete.

NODE-164 (2026-09-17): The addon dependency key changes from napi to napi_runtime, still selecting package napi 3.12.2 with the same features. A crate-wide lexical facade supplies module-local class tags while reexporting the actual N-API traits; no dependency-graph package is added. Portable property snapshot/publication changes stay in the existing Handle leaf. The priority class-isolation implementation gap is superseded; [exact boundary/live verification and remaining Node batch gates](pwright-node-handles.md) are recorded canonically and must not be inferred from historical passes.

NODE-163: Handle producer/publication work stays inside the existing handles profile and native-creation leaf selection. No package or feature edge is added. The Node boundary checker still reports 249 packages for selected handles and rejects browser driver, CDP transport or process-launcher imports from production JS.

NODE-162: Independent default-off handle-value exposes native-only Handle value programs. The Node handles profile explicitly selects it alongside frame-value and native-creation; exact dependency fingerprints now permit five lower-level differences including lifecycle/native-release and runtime/disposal-reservations. No package or Node driver dependency is added.

NODE-161: The empty frame-value feature is independently selectable in the Handle evaluation leaf and adds no package. Host handles now permits exactly four lower-level differences: frame-value, native-creation, native-release and disposal-reservations. Default host/addon graphs and other named Node profiles remain feature-cold; the boundary checker verifies these exact closures.

NODE-160: The Node addon adds only an explicit handles -> node-host/handles feature edge and fixed native/handles/pwright.node artifact. Defaults and other named profiles remain unselected. Existing exact host dependency proofs and addon marker checks now include this independent public selection; no Node Playwright/CDP/child-process dependency is introduced.

NODE-159: Host handles now explicitly selects evaluation/native-creation alongside lifecycle/native-release and runtime/disposal-reservations. Exact ordinary/upload fingerprints permit only these three feature differences, with no new package; Node addon handles remains unselected.

NODE-158: Evaluation leaf native-creation is an empty opt-in feature with no default selection, new dependency package or transitive feature. Its independently verified staged API is not selected by the host/addon and does not install a runtime or scheduler.

NODE-157: Host handles explicitly adds lifecycle/native-release and runtime/disposal-reservations, with no new package. The evaluation leaf directly reuses the already-transitive CDP classifier. The aggregate profile delegates route budgeting to its leaf. Node addon handles stays disabled; default dependencies/features remain cold.

NODE-156: Lifecycle native-release adds only the existing runtime disposal-reservations feature, checked by scripts/check-v2-native-handle-release-dependencies.sh. Node host/addon matrices explicitly reject selecting it before coordinator integration; no native artifact is rebuilt.

NODE-155: Queue-free admission and recoverable disposal preparation stay inside the existing disposal-reservations feature. No Cargo edge, feature membership or addon selection changes. In-process Handles must not enable a control outbox/concurrent-close just to obtain disposal admission.

NODE-154: Pending-result ownership stays inside the selected safe-host handles module and reuses existing leaves/runtime/Tokio dependencies. No Cargo edges or feature memberships change. The Node addon still does not select handles; no native artifact is rebuilt.

NODE-152: The optional safe-host handles feature adds explicit edges to the existing Handle leaves but adds no package or downstream feature: exact graph fingerprints match ordinary host versus handles and file-input versus file-input+handles. Host matrix checks now include handles alone and combined selections; addon matrices assert handles remains unselected. Native route installation is separate from the still-unimplemented Node result/publication owner.
[Native Handle installation and remaining ownership gates](pwright-node-handles.md#node-152-opt-in-safe-host-native-installation).

NODE-149: The selected file-input addon closure is exactly ordinary addon union selected safe host. Addon standalone/combined feature matrices verify six upload leaves and large-commands only when selected.
[Selected Node upload boundary and verification](pwright-file-input.md#node-149-selected-node-buffer-ingress-and-public-locator-uploads).

NODE-148: The safe-host file-input feature selects the union of the existing six upload leaves, admitted filesystem executor and large-command transport. Exact closure and host feature-matrix checks cover standalone/combined selections. Ordinary host and every existing addon profile remain upload-free.
[Selected safe-host upload contract](pwright-file-input.md#node-148-selected-safe-host-upload-integration).

NODE-147: The file-input backend large-commands feature forwards only pwright-cdp/large-commands and adds no package dependency. The exact closure proof checks both selections; ordinary Node host/addon still exclude all six file-input leaves.
[Large outbound admission and remaining gates](pwright-file-input.md#node-147-independently-admitted-large-outbound-commands).

NODE-146: The upload continuation leaf selects the union of the buffered backend and Locator-resolution composition closures. It adds no filesystem, executor-runtime or Node production dependency and creates no runtime. Ordinary host/addon still exclude all six file-input leaves. Tokio test-util is dev-only for paused-clock timeout evidence.
[Upload continuation and remaining gates](pwright-file-input.md#node-146-single-use-upload-continuations).

NODE-145: The opt-in Chromium file-input backend adds model/source/base64 to the existing Locator-resolution CDP closure. It has no normal filesystem/executor/Node dependency; those compose only in the live test. Ordinary host/addon exclude all five new file-input leaves. The adapted DOM helper retains its Apache-2.0 notice/license; this crate declares MIT AND Apache-2.0.
[Backend selection and licensing](pwright-file-input.md#node-145-chromium-payload-mutation).

NODE-144: The file-input Tokio leaf adds only tokio and pin-project-lite to the core/model/source/filesystem closure (plus Unix libc). Normal dependencies exclude Node, CDP and network/IO-driver dependencies. All four file-input leaves remain absent from ordinary host/addon selections; CI checks the expanded exact closure.
[Optional executor contract](pwright-file-input.md#node-144-admitted-blocking-executor).

NODE-143: Optional pwright-file-input-fs selects only core, model, source and Unix libc. Its pinned MIME table is static data with the original MIT license, not a production Playwright dependency. All three file-input leaves stay absent from ordinary host/addon closures; the dedicated dependency proof now checks the filesystem leaf too.
[Filesystem selection boundary](pwright-file-input.md#node-143-filesystem-preparation-and-pinned-mime).

NODE-142: pwright-file-input-source selects exactly pwright-core and pwright-file-input-model as normal dependencies. The model stays dependency-free; ordinary Node host/addon exclude both. The dependency proof and CI now cover the source leaf without enabling filesystem runtime, Tokio, N-API, CDP or Locator transitively.
[Content ownership boundary](pwright-file-input.md#node-142-ledger-admitted-content-ownership).

NODE-141: pwright-file-input-model is a new zero-dependency leaf, not a dependency of ordinary Node host/addon or Locator. A dedicated proof and CI selection protect that boundary. Existing nine host/ten addon closure checks remain passing; no upload feature is wired yet.
[File-input policy, evidence and remaining work](pwright-file-input.md).

NODE-134: Outbox storage selection stays inside existing runtime disposal-reservations; core gains explicit queue preallocation methods without changing lazy constructors. No new package or transitive feature is added. Cold runtime lacks the storage-lease field, and Node host/addon profiles do not select this feature.
[Preallocated runtime outbox contract and evidence](pwright-close-cascade.md#node-134-preallocated-runtime-control-outbox-storage).

NODE-130: The independent close-route-retirement feature has no prerequisites or dependency additions. Its exact normal package/version/feature proof compares cold versus selected roots. It does not implicitly select native confirmation, ancestor authority, scheduler, Node integration or unrelated capabilities.
[Deletion-only close route contract and evidence](pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-129: native-close-scheduler selects only the existing native-close-authority feature and its native-confirmation prerequisite. The parameterized exact-normal-closure proof covers both selections. No production package is added and Node host/addon feature selections remain unchanged.
[Bounded native scheduler contract and evidence](pwright-close-cascade.md#node-129-bounded-native-close-scheduler).

NODE-128: Optional native-close-authority selects existing native-confirmation and a direct optional common CDP classifier. The proof finds no new normal packages or other transitive-feature changes. Peer dependencies are dev-only; ordinary and Node selections stay unchanged. CI adds explicit selected tests/lint/proof.
[Native ancestor authority contract and evidence](pwright-close-cascade.md#node-128-exact-native-context-ancestor-authority).

NODE-127: Native command/receipt phases reuse the existing native-confirmation feature. Added futures-util/tokio-tungstenite entries are dev-only protocol-peer dependencies; the selected proof shows no normal package/version/transitive-feature delta. CI explicitly checks selected phases, strict lint and dependency closure.
[Native close phase contract and evidence](pwright-close-cascade.md#node-127-native-command-and-destruction-wait-phases).

NODE-126: The existing composition observer trait gains a default opt-out deferred capacity method, implemented by the selected host observer. No Cargo dependency/feature selection or ordinary service fields change; no executor, subscription or native command is added.
[Deferred publication contract and evidence](pwright-close-cascade.md#node-126-deferred-close-publication-capacity).

NODE-125: Adds the empty disposal-reservations feature to executor-independent pwright-runtime. Selected and selected-plus-lifetime-handoff dependency proofs show no normal package/version/transitive-feature delta. No host/addon selects it, and Node boundary checks remain green. CI includes selected/combined unit tests, the public API example and strict checks; no new worker or transport dependency.
[Runtime reservation contract and evidence](pwright-close-cascade.md#node-125-opt-in-runtime-disposal-and-outbox-reservations).

NODE-124: Adds the empty opt-in transition-reservations feature on the existing CDP topology leaf. Its module contains storage/ownership only, with no commands, executor, subscription or worker. A new CI proof compares complete normal transitive packages, versions and features against feature-off topology and finds no delta. Neither host nor addon selects it; the original topology-only example remains green.
[Topology reservation contract and evidence](pwright-close-cascade.md#node-124-opt-in-topology-transition-reservations).

NODE-122: Composition adds an explicit direct edge to its existing page-lifecycle leaf. That leaf exposes additive confirmation contracts and opt-in `native-confirmation` implementation (`serde_json` only; no new executor). The host selects this feature but ordinary construction does not install the source or its additional cursor/storage. Seven safe-host and eight addon resolved dependency selections pass unchanged package-closure checks; the JS boundary check still rejects browser drivers/transports/process launchers.
[Selected confirmation](pwright-page-close.md#node-122-selected-native-destruction-confirmation).

NODE-119: Native ready/linkage integration adds no dependency, subscription, transport or worker. Metadata retains exact masks and charged acknowledgement tokens; JavaScript remains a wrapper/EventEmitter adapter. Seven safe-host and eight addon resolved dependency selections remain the required proof.
[Native ready-source contract and evidence](pwright-node-lifecycle-events.md#node-119-native-ready-and-popup-source).

NODE-118: Ready/popup conversion adds no dependency and preserves the safe-host boundary. The addon consumes metadata-only descriptors; injected JS factories only construct language wrappers. Native source selection is unchanged, so this step does not claim production creation-event parity.
[Creation-conversion contract and evidence](pwright-node-lifecycle-events.md#node-118-precommit-creation-wrapper-conversion).

NODE-117: External-close integration reuses the borrowed committed-driver hook and existing publication queue, adding no dependency or JS browser policy. Safe-host/default and all combined dependency selections remain proved; only the Node lifecycle composition selects this source by default.
[External-close source contract and evidence](pwright-node-lifecycle-events.md#node-117-pre-admitted-external-close-source).

NODE-116: Observed attachment factories compose the existing native close decorator inside the existing Continuable close services. Safe-host service selection is a typed ordinary/observed choice. No dependency or native protocol change is added; Page/Context close sources remain selected composition, not global dispatch.
[Explicit-close source contract and evidence](pwright-node-lifecycle-events.md#node-116-explicit-close-source-integration).

NODE-115: Terminal dispatch adds no dependency, transport, worker or process. The safe host remains Node-free; addon-owned non-Send ports carry environment-scoped references, while workers carry native fences only. Terminal selection is at the Node composition boundary.
[Terminal dispatcher contract and evidence](pwright-node-lifecycle-events.md#node-115-environment-thread-terminal-dispatch).

# Pwright v2 dependency topology

NODE-114: Terminal event production is composed within the existing host metadata owner using existing types and ledgers. No dependency, Cargo feature or protocol tag is added; the experimental Rust entry point is not selected by the Node addon.
[Terminal source contract and evidence](pwright-node-lifecycle-events.md#node-114-pre-admitted-native-terminal-source).

NODE-113: The host group primitive uses existing core, metadata and std types only. No Cargo dependency, feature or wire tag changes. It is not selected by production host producers or Node delivery yet.
[Group contract and evidence](pwright-node-lifecycle-events.md#node-113-bounded-dependent-publication-groups).

NODE-112: The composition adds a native close observer/decorator using existing core
OperationError, effects and std ownership. No dependency, feature or wire tag is
added. The ordinary services select no observer; callers compose the decorator
inside their existing continuation owner.
[NODE-112 close receipt contract and verification](pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: The existing composition observer adds a default committed-effects method and
reexports a borrowed view. No crate, feature, wire tag, subscription or dependency
edge is added. Legacy observers retain their snapshot behavior.
[NODE-111 source contract and verification](pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-110 reuses existing napi/sys to report synchronous listener exceptions from
the addon. No crate/feature or core dependency edge changes. The Node boundary
adds only catch-and-forward behavior; browser state and one-shot disconnect
publication remain Rust-owned.
[NODE-110 contracts, fix and verification](pwright-node-lifecycle-events.md#node-110-listener-failure-boundaries-and-scoped-exception-reporting).

NODE-109 adds a private host drain module with public typed ownership/progress
APIs, using existing std and thiserror dependencies. No crate, feature, transport,
runtime subscription, event cursor or JS dependency is introduced. Node dispatch
is not wired to this primitive.
[NODE-109 contract and verification](pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-108 adds a private CDP TRACE helper and test-only lifecycle receipt module,
reusing tracing and serde already present. No crate/feature, dependency edge,
event cursor or runtime subscription is added. All 7 host / 8 addon dependency
selections remain verified; pending-session retirement is the existing Node
profile selection, not a newly introduced native behavior.
[NODE-108 receipts and verification](pwright-chromium-topology-events.md#node-108-bounded-native-lifecycle-identity-receipts).

NODE-107 reuses ResourceLedger::shares_budget_with for revision domain identity
and adds a Rust-only rebasing module plus Context metadata comparison. There is
no new crate/feature, native event subscription, foreign port or JS logic.
[NODE-107 contracts and evidence](pwright-node-lifecycle-events.md#node-107-validated-cross-revision-visibility-rebasing).

NODE-106 adds only a Rust host metadata submodule and public view/member/error /
iterator types. No dependency edge, feature selection, native subscriber, NAPI
callback or JavaScript state machine is added. The existing selected collection
read path consumes full views. All 7 host / 8 addon dependency selections pass.
[Implementation boundary](pwright-node-lifecycle-events.md#node-106-immutable-single-revision-visibility-foundation).

NODE-105 identifies existing pwright-cap-page-events, CDP opener effects and
ChromiumTopologyPageEventPublisher as reuse points for Node lifecycle dispatch.
No dependency, feature or subscriber is installed by this test/design checkpoint.
Do not pull in a Page-load cursor or unrelated event transport solely for topology
notices; missing Page/Context close event registration still needs an explicit audit.
[Proposed seams and verification](pwright-node-lifecycle-events.md).

NODE-103 adds host frame_trees::collections and collection_reads, composition
membership iterators and a Rust addon baseURL boundary helper. Selection remains
inside the existing Node composition; no Cargo dependency, feature, runtime,
domain or subscriber is added. Seven host/eight addon resolved selections and
strict combined Clippy pass. Default and drag-frame-extra-headers were rebuilt.
[Publication architecture and artifact evidence](pwright-frame-publication.md#node-103-rust-collection-producers-and-published-node-reads).
Historical NODE-102 artifact/unwired statements below no longer describe HEAD's worktree.

NODE-102 adds the internal frame_metadata::collections module and typed Rust
reservation/revision ports, reusing FrameTreeFactory and the existing queue.
No Cargo feature, dependency, subscriber or task is added; seven host/eight addon
resolved selections and strict combined Clippy pass. Native artifacts remain
NODE-100 binaries, and collection producers/getters have not been switched.
[Implementation and evidence](pwright-frame-publication.md#node-102-bounded-rust-collection-revision-storage).

NODE-100 adds no dependency or feature: the existing Node host metadata and
native Session binding own Browser disconnect publication. Resolved checks pass
for seven host and eight addon selections; only default and combined artifacts
were rebuilt. The test-only transport Worker is not a production driver.
[Contract and artifact identities](pwright-browser-close.md#node-100-rust-owned-browser-disconnect-publication).

NODE-099 adds the internal host module `frame_metadata::close` and a synchronous
getter on the existing native Frame-tree binding. It adds no Cargo feature or
dependency and selects no extra browser domain, cursor or task. The Node host's
existing metadata composition owns admission/publication; other API profiles
remain independently selected. Rebuild each artifact before using the getter;
this checkpoint verifies default/combined, not every stale local binary.
[Contract and resolved-boundary evidence](pwright-page-close.md#node-099-rust-owned-page-close-publication).

NODE-095 adds no dependency or Cargo feature. Pending-session retirement is an
explicit CDP production-profile option selected by the Node host, with an ID
budget and existing command leases; it installs no additional event subscriber.
[Contract and evidence](pwright-chromium-topology-events.md#node-095-native-detach-before-an-initialization-response).

NODE-092 changes existing CdpSession cleanup-result handling without new features,
production dependencies or runtime owners. The diagnostic lane reuses the pinned
test-only WebSocket relay. [Contract](pwright-chromium-topology-events.md#node-092-child-disappears-between-resume-and-detach).

NODE-091 reuses the optional header feature's futures-util AtomicWaker and the
existing topology observer. It adds no Cargo dependency, default-on capability,
extra event cursor or production Node driver. Frame callbacks delegate through
the composed observer only when selected. [Contract](pwright-extra-http-headers.md#node-091-native-session-closure-during-header-fanout).

NODE-090 adds Rust discovered/synchronized-session helpers and optional observer cleanup
callback without new production dependencies, tasks or event cursors. Header and
attachment tests share a loopback-only relay using the already pinned upstream
WebSocket test dependency; it is not shipped in the production package.
[Scope](pwright-chromium-topology-events.md#node-090-missing-session-retirement-before-page-publication).

NODE-089 changes selected Rust header dispatch without adding production
packages, tasks, event cursors or Node CDP imports. The diagnostic receipt relay
uses WebSockets from the already-pinned Playwright test installation, only under
test/. It is not a shipped runtime dependency.
[Scope](pwright-extra-http-headers.md#node-089-native-receipt-gates-and-context-page-sequencing).

NODE-088 selects futures-util as an optional direct host dependency only for
extra-headers, reusing its already-transitive package for caller-polled queues.
No extra task, process or event cursor is added. Default remains header-free;
new job/native future bounds and the 419-owner pool are separately selected.
[Dependency and resource boundary](pwright-extra-http-headers.md#node-088-caller-polled-native-jobs-and-early-primary-rejection).

NODE-087 adds only a selected host preparation module and synchronous Rust ports;
no dependency, runtime task or CDP cursor is added. The selected host now chooses
nine metadata writers and a derived 291-owner header pool. Default addon compile
and all seven host/eight addon resolved-profile checks pass.
[Selection and bounds](pwright-extra-http-headers.md#node-087-synchronous-rust-preparation-and-frozen-dispatch).

NODE-086 adds an explicit overlapping-writer profile inside the existing optional
network header model, plus a feature-gated development example. Production
requirements derive pending storage from the selected count; no dependency or
native task is added. Default remains header-free; the host still selects one writer.
[Boundary and verification](pwright-extra-http-headers.md#node-086-pending-setter-race-and-bounded-overlapping-model).

NODE-085 uses the existing optional header model from the Page.goto path and,
when independently selected, the public Frame.goto path. The raw native Frame
port stays lower-level. No dependencies, cursors or tasks are added; only the
selected header body allowance grows for bounded navigation preparation.
[Integration and evidence](pwright-extra-http-headers.md#node-085-navigation-referrer-reconciliation).

NODE-084 extends only the selected `extra-headers` host/addon path for Context
creation. No dependency or process is added; header-free default still compiles.
Creation reuses existing topology lifecycle, settings and FFI resource owners.
[Contract and gates](pwright-extra-http-headers.md#node-084-context-creation-headers).

NODE-083 separates `HeaderSyntaxError` from desired-state admission errors and
adds bounded Page-local failure handling inside the existing selected host.
No dependency, event cursor or process is added. Default remains header-free;
header/drag/frame selection still composes through the same optional feature.
[Behavior and verification](pwright-extra-http-headers.md#node-083-deferred-syntax-and-page-local-initialization-failure).

NODE-082 forwards optional addon `extra-headers` to the existing selected Rust
host. No dependency is added; the feature remains absent from default/drag/frame
profiles unless requested. Packaging enumerates all eight combinations of drag,
frame-navigation and headers; compiled flags must match exactly. Dependency
checks cover all eight addon selections, and all-three-feature Clippy passes.
Only header-only is newly built/live-tested, not every combined artifact.
[Selection evidence](pwright-extra-http-headers.md#node-082-public-selected-setters-and-language-phase-differential).

NODE-081 adds a direct optional network-model edge to the Rust host and forwards
only `extra-headers`. No new production package is introduced into its existing
closure. Seven host boundary selections (including header-only and combined
drag/navigation/headers) and four existing addon selections pass. Addons do not
enable header policy, and production JS still has no CDP/driver/process imports.
[Selection contract](pwright-extra-http-headers.md#node-081-selected-rust-host-and-native-session-reconciliation).

NODE-080 adds transactions under the existing default-off network-model
extra-headers feature. It uses only core and std; no host, transport, task or
Node production dependency is added. The new bounded setter oracle shares the
existing dev-only serde dependencies.
[Boundary](pwright-extra-http-headers.md#node-080-bounded-desired-settings-transactions).

NODE-079 changes the existing composition initializer interface through a
backward-compatible default method and borrowed target descriptor. No Cargo
dependency/feature or Node runtime import is added; selecting headers in the
actual host remains a separate pending change.
[Port boundary](pwright-extra-http-headers.md#node-079-identity-aware-native-initialization).

NODE-078 adds `pwright-network-model/extra-headers`, default off. Normal
closures with and without selection are exactly network-model plus core.
Serde/JSON are dev-only dependencies for the bounded oracle example; no driver,
transport or host feature is selected. CI adds explicit optional tests/Clippy.
[Boundary](pwright-extra-http-headers.md#rust-ownership-contract).

NODE-076 adds addon frame-navigation selection and four fixed package profiles.
The expanded boundary gate verifies five host and four addon selections,
including combined drag/runtime and Frame navigation. Host default/frame normal
closures are 234 total packages; drag adds one. Addon default/frame closures are
249; drag adds one. Navigation does not enable drag or lifetime-handoff; only
drag-runtime enables that runtime extension. The optional direct capability
edge adds no package to the existing transitive closure. CI explicitly checks
selected host units and single/combined addon Clippy. Runtime JS still imports
no upstream driver, CDP transport or process launcher.
[Packaging boundary](pwright-frame-navigation.md#node-076-public-node-frame-navigation).

NODE-075 forwards host `frame-navigation` to the existing capability crate's
1.2 feature and adds a bounded Rust result projector. No package, JS automation
logic, task or CDP cursor is added. Default and selected host normal closures
match at 105 Pwright packages. Default/readiness-only hosts do not install
result slots; no generated server or Node addon selection changes here.
[Selection](pwright-frame-navigation.md#node-075-typed-rust-host-result-ownership).

NODE-074 adds default-off `frame-navigation` selection to navigation capability,
codec, client and server crates. No normal dependency package is added. Codec
selection also enables its existing client dev-dependency's Frame extension for
the stream test. Root APIs/codecs/server stay 1.1; the explicit `v1_2` chain
requires four services. The default navigation-only proof remains 15 packages.
The Rust host and Node addon do not yet select this typed chain.
Selected/default server normal closures were compared directly: both contain
the same nine Pwright packages. A separate default-feature test run passes 32
tests; selected feature unification does not replace the root 1.1 binding.
[Selection](pwright-frame-navigation.md#node-074-opt-in-typed-navigation-12-contract).

NODE-073 installs retained native navigation routes and the executor only under
the existing Rust host `frame-navigation` feature. Default and readiness-only
hosts do not select its owners. No package, JS runtime logic or new event cursor
is introduced; generated/public Node Frame.goto remains separate work.
[Selection boundary](pwright-frame-navigation.md#node-073-rust-host-dispatch-scheduling-and-cancellation).

NODE-072 adds host `frame-navigation = ["frame-lifecycle"]`, separate from
readiness-only selection. Fixed Page slots and per-operation response budgets
are selected only for navigation. Registration uses existing Rust/Notify
primitives, with no new package, queue, task or runtime JS dependency. The
retained default/drag addon binaries are unchanged.
[Selection](pwright-frame-navigation.md#node-072-ordered-operation-registration-and-actual-rust-host-observation).

NODE-071 places the navigation waiter under the existing default-off navigation
leaf `frame-lifecycle` feature. It adds no package or dependency, runtime JS
import, native client or background executor. Its offline bridge is a selected
example; the pinned JS oracle stays in tests, not the Node runtime package.
[Selection and evidence](pwright-frame-navigation.md#node-071-bounded-navigation-matching-and-response-retention).

NODE-070 adds the default-off Rust host `frame-lifecycle` feature forwarding to
the existing composition feature. Registry code stays under that selected
module; no new package or addon feature/API is added. Default and selected normal
host closures have identical sets of 105 Pwright packages. Both retained addon
hashes are unchanged; the actual host selection is verified separately in Rust.
[Host selection](pwright-frame-navigation.md#node-070-bounded-multi-page-registry-and-opt-in-rust-host-selection).

NODE-069 adds a default-off composition `frame-lifecycle` feature and optional
direct edge to the existing navigation leaf's same-named feature. The generic
observer port has no new dependency; only the selected concrete owner compiles
the lifecycle implementation. Neither Node host nor addon selects it yet.
Default and selected normal closures have the same 90 Pwright package names;
the selected inverse feature graph explicitly enables navigation/frame-lifecycle.
The navigation-only closure remains 15 packages. Both Node source profiles pass;
retained addon hashes are unchanged.
[Selection contract](pwright-frame-navigation.md#node-069-opt-in-production-ordered-lifecycle-observation).

NODE-067 adds no Cargo dependency or default selection. Native lifecycle
normalization lives under the navigation leaf's existing default-off
`frame-lifecycle` feature and consumes a borrowed Rust route port. No public
network capability, evaluation world or Node driver is pulled into this owner.
Neither addon has been rebuilt or exposed a new API.
[Native adapter boundary](pwright-frame-navigation.md#node-067-bounded-native-lifecycle-normalization).

NODE-066 adds no Cargo edge. Its optional composition route table delegates
through the aggregate backend to the existing isolated Frame initiation adapter.
Installation requires navigation but not evaluation/world readiness or public
network events. It does not select the separate frame-lifecycle feature, change
Node package exports, or rebuild either addon.
[Selection boundary](pwright-frame-navigation.md#node-066-retained-routes-in-the-production-topology-transaction).

NODE-065 uses the default-off navigation `frame-lifecycle` feature. Selected and
default normal dependency sets are identical; navigation-only remains 15 Pwright
packages. No public network-event, Locator, Node or launch dependency is added.
The host does not select the feature yet; no browser/event driver is installed.
[Scope and proof](pwright-frame-navigation.md#node-065-optional-normalized-lifecycle-reducer).

NODE-064 adds no Cargo edge or runtime JS import. Borrowed navigation routing
stays in the topology leaf; explicit Frame initiation stays in the navigation
leaf. Exact normal closures pass at five topology-only and 15 navigation-only
Pwright packages. No new command is advertised, and Node artifacts are unchanged.
[Selection and pending full-operation design](pwright-frame-navigation.md#resource-and-dependency-contract).

NODE-063 public lookup adds no Cargo edge or runtime JS driver/CDP/launcher
import. Selection is Rust; RegExp, URL values and synchronous callbacks are
scoped N-API operations. Verified closures remain host 234/235/235 and addon
249/250. Selected addon 1b627f19 is rebuilt; historical default bytes are unchanged.
[Artifact and evidence](pwright-frame-lookup.md#node-063-evidence-and-remaining-work).

NODE-062 adds no Cargo dependency, process, task or JS driver import. Node-host
Context metadata is explicit and navigation/glob share a scoped N-API URL-value
helper. Final selected addon 9c36434e now passes Context, Page and glob gates;
historical default bytes are unchanged. Closures remain 234/235 host and 249/250
addon packages for unselected/selected profiles.
[Artifact and verification evidence](pwright-context-creation.md#verification-status-and-remaining-gates).

NODE-060 selects Node WHATWG URL as a foreign value port, with all glob policy
in Rust. It adds no Cargo edge, task or runtime JS driver/CDP/launcher import.
The diagnostic Rust parser remains separate. The selected native addon and its
MIT/Apache notices were rebuilt and verified; historical default bytes are unchanged.
[Selection and artifact evidence](pwright-frame-lookup.md#node-060-actual-n-api-url-value-conversion-port).

NODE-059 adds no Cargo edge, runtime JS import, task or installed capability.
Explicitly called Rust glob helpers share the Frame ledger; only the offline
example currently selects the existing `url` parser. Production parser resource
admission remains separate. Apache attribution now includes preprocessing.
[Selection and evidence](pwright-frame-lookup.md#node-059-bounded-glob-preprocessing-and-borrowed-url-resolution).

NODE-058 adds no Cargo dependency edge, installed runtime capability or task.
Rust lookup/glob helpers allocate only when explicitly called, sharing Frame
ledger admission. The offline oracle bridge is test infrastructure, not a Node
runtime driver. Host license metadata now includes the Apache-2.0 adapted compiler.
[Selection, proof and attribution](pwright-frame-lookup.md#node-058-rust-traversal-and-bounded-resolved-glob-compiler).

NODE-056 adds no Cargo edge, process or task. The existing URL observer gains
a default-compatible navigation callback; only the Node host selects separate
name storage. Host closure counts remain 234/235/235 and addon counts 249/250.
Borrowed native conversion introduces no runtime JS driver or transport import.
[Selection and evidence](pwright-frame-publication.md#node-056-bounded-navigation-names-and-public-framename).

NODE-055 adds no Cargo edge, task, ledger or process launcher. Descriptor reads
and native tree wrappers share the existing metadata budget. The CDP client
adds an optional user-gesture evaluation port; backend profiles default off,
while the Node value evaluator explicitly selects it without changing resource
requirements. The JS layer only performs conversion/cache identity work.
[Contract and evidence](pwright-frame-publication.md#node-055-admitted-page-descriptors-and-cache-aware-enumeration).

NODE-054 transfers a pre-reserved root binding through the existing host/addon
Page descriptor. It introduces no Cargo edge, feature, ledger, task or launcher;
resource-error creation ordering remains in Rust. Normal closures and the
runtime-JS import boundary remain unchanged.
[Admission and evidence](pwright-frame-publication.md#node-054-creation-time-root-binding-admission).

NODE-053 adds metadata-only N-API Frame bindings and JS wrapper identity caching
with no new Cargo edge, driver, transport, launcher or task. Read/binding leases
share the existing host Frame ledger; retained native metadata excludes the
FFI execution registry. Host closures remain 234/235, addon 249/250.
[Admission and verification](pwright-frame-publication.md#node-053-public-node-frame-collections-and-retained-wrappers).

NODE-052 adds an optional borrowed ownership callback and installs its bounded
Frame index only in the Node host. Native child-release and paused-world fixes
add no Cargo edges, background task or browser launcher. Resolved host closures
remain 234/235; addon closures remain 249/250. Only the explicitly selected
recursive attachment profile overlaps utility-world creation with target resume.
[Contract and verification](pwright-frame-publication.md#node-052-live-host-projection-and-paused-oopif-attachment).

NODE-050 forwards the optional host `drag-runtime` feature through the addon.
Default/selected addon closures are 249/250 normal packages including addon;
host default/source/runtime remain 234/235/235. `pnpm test:boundary` verifies
each resolved profile. The JS selector/build metadata adds no browser transport,
driver or launcher to the runtime facade.
[Selection](pwright-chromium-drag.md#explicit-local-node-artifact-selection).

NODE-049 adds the empty, default-off `pwright-runtime/lifetime-handoff` feature.
Only host `drag-runtime` selects it; the Locator leaf exposes an injected phase
port without depending on runtime or drag. No Cargo package edge is introduced.
`pnpm test:boundary` verifies resolved runtime features separately for all three
host profiles: default/source/runtime remain 234/235/235 packages including host.
Only the last selects lifetime handoff; input/Locator leaf proofs pass at 5/10.
[Contract](pwright-chromium-drag.md#phase-transition-and-cancellation-identity).

NODE-047 adds explicit Frame selector / Page keyboard binding without a Locator
or input dependency on drag. Default/source/runtime host closures remain
234/235/235; only the opt-in host selects the shared controller.
[Boundary](pwright-chromium-drag.md#node-047-locator-keyboard-selects-the-owning-page).

NODE-046 adds an optional transaction-local keydown port in the input leaf, then
explicit Keyboard backend/Page selection. Only the already-optional drag
Mouse dispatcher implements cancellation. Host closures stay 234/235/235 and
drag 3/5; Keyboard does not depend on drag.
[Ownership](pwright-chromium-drag.md#node-046-selection-and-resource-boundary).

NODE-045 promotes the already-transitive `pwright-input-model` to a direct
normal host dependency for shared click bounds. Dependency closures are unchanged
at 234/235/235 (host) and 3/5 (drag); no Node-API or browser runtime enters the leaf.
[Details](pwright-chromium-drag.md#node-045-maximum-click-batches-and-matching-host-admission).

NODE-044 adds `drag-runtime` on top of `drag-source` without another dependency.
Both are opt-in; the default host and addon selection remain unchanged. Generic
extension event-cursor accounting lives in the existing Chromium composition.
[Selected resource profile](pwright-chromium-drag.md#selected-resource-profile).

NODE-043 adds an opt-in host `drag-source`: fresh Page utility snapshots now
remain owned through movement results, retirement and quarantine. Exact world
destruction drains started listener requests without issuing new cleanup there.
Default/selected host closures are **234/235**; only the drag leaf is added.
HostSession Mouse/close/Escape selection and public differential checks remain
pending; the addon and public **36/41** baseline are unchanged.
[Production source and evidence](pwright-chromium-drag.md#node-043-retained-production-page-drag-source).

NODE-042 adds immediate, budgeted snapshots of a Page's existing utility worlds,
including child Frames/OOPIFs without waiting for a main world. Utility-only
native destruction receipts retain snapshot admission and remain distinct from
route invalidation and owned-session retirement. Production drag-source/host
integration is still pending; no addon rebuild or public **36/41** change.
[Snapshot ownership and verification](pwright-chromium-drag.md#node-042-existing-page-utility-world-snapshots).

NODE-041 implements the explicit `mouse-dispatch` feature and Page-owned cleanup
driver: **82** unit tests and **4** selected-Mouse native cases pass, alongside
the prior **10** native cases. Default/feature closures are **3/5** Pwright
packages. Host Frame-source/lifecycle and Keyboard Escape integration remain
pending; no addon rebuild or public **36/41** change.
[Selected dispatcher and evidence](pwright-chromium-drag.md#node-041-selected-mouse-dispatcher-and-page-cleanup-driver).

NODE-040 adds Page-owned drag state and retained native enter/over/drop/cancel.
**70** unit tests and **10** native scenarios pass, including actual drop and
cancel with dragend checks. The optional leaf remains **3** Pwright packages;
Mouse dispatch/forClick/down suppression, Keyboard Escape and Node host selection
are still pending. The addon/public **36/41** evidence is unchanged.
[Page state and verification](pwright-chromium-drag.md#node-040-page-drag-state-and-native-event-owners).

NODE-039 adds intercepted-movement composition in the same **3-package** optional
drag leaf. Lazy native movement is injected, not a dependency on Mouse/input.
Native Mouse/Mouse/runtime Mouse remain **8/16/17**, host **234**. Production
Mouse does not select this helper yet.
[Owner and allocation boundary](pwright-chromium-drag.md#node-039-composed-intercepted-movement).

NODE-038 adds fixed-session capture and its `CdpSession` adapter within the
existing optional drag leaf. No dependency is added: drag/native Mouse/default
Mouse/runtime Mouse closures remain **3/8/16/17**. Neither Mouse nor the Node
host selects it. [Resource exclusions](pwright-chromium-drag.md#node-038-retained-capture-lifecycle).

NODE-037 adds typed drag-data retention without dependencies: drag remains
**3** Pwright packages, native Mouse **8**, Mouse default/runtime **16/17**,
and host **234** normal packages. All four boundary scripts pass. The leaf is
still unselected in production.
[Admission accounting](pwright-chromium-drag.md#node-037-admitted-intercepted-data).

NODE-036 adds `pwright-backend-cdp-drag` as a separately installable utility-world
listener leaf, with exactly **3** normal Pwright packages (itself/CDP/core).
`check-v2-drag-backend.mjs` rejects Mouse, Locator, runtime and Node installation.
It is not selected by the host or existing Mouse package; their **234** and
**8/16/17** closures remain unchanged. [Scope/accounting](pwright-chromium-drag.md).

NODE-035 threads one optional dispatcher through native/Page bindings and adds
a standalone button owner in the existing input leaf. Dependency proofs still
report **5/8/16/17** Pwright packages for input-only/native/Mouse/controlled Mouse,
and host **234** normal packages. Outer operation and Page-binding measurements
include the new owner/Arc; provider allocation trees remain separately budgeted.
The Node host does not yet select a drag provider.
[Selection scope](pwright-raw-mouse.md#page-scoped-dispatcher-selection-node-035).

NODE-034's optional Mouse dispatch traits live in the existing input leaf and
add no packages. Rechecked closures remain input-only **5**, native **8**,
Mouse **16/17**, host **234** normal packages. Larger fixed dispatch slots enter
the existing payload measurement; injected provider allocation trees require
their own admission, not an implied share of the host's direct-path allowance.
No Chromium drag provider is installed yet.
[Ownership scope](pwright-raw-mouse.md#optional-mouse-dispatch-ownership-node-034).

NODE-033 adds a session-local 32-slot first-poll ordering component inside the
safe Node host. It introduces no new package/runtime dependency: host remains
**234** normal packages and Mouse stays **16/17**. N-API explicitly selects it for
raw Mouse/Keyboard; portable input/native leaves do not depend on Node scheduling.
Its measured fixed array has its own production scope **120/22**.

Native Mouse starters add an independent **8-Pwright-package** normal closure,
enforced by `scripts/check-v2-mouse-backend.mjs`. They reuse the command-free CDP
input owner and Mouse capability without runtime, Locator, Node or a driver.
The optional Page-route composition now has a **16-Pwright-package** normal
closure, checked by `scripts/check-v2-mouse-composition.mjs`. It owns bounded
records and caller-polled retirement receipts without a controlled runtime,
full Chromium composition, Locator or Node host. Explicit `runtime` adds exactly
`pwright-runtime` (**17** packages) and a single-call receipt-aware adapter, no
executor or full host. The safe host now selects bounded independent retirement
workers with host storage/provider allowances and an explicit shutdown join.
The final foreign/nested-runtime allocation audit remains a separate FFI gate.

Optional Mouse portable packages have exact normal Pwright closures of
**3 capability / 6 client / 7 codec / 10 server**, with no Tokio, Node, generator
or native backend dependencies. `scripts/check-v2-mouse-contracts.mjs` enforces
these independent boundaries. The host source now has **234** packages and
input-only remains **5**; Mouse is selected by the host, not the input leaf.
[Contract details](pwright-raw-mouse.md#optional-mouse-10-contracts).

Latest host source installation: **234** normal packages, up from 229 by selecting
Mouse capability/backend/codec/server/composition with its runtime feature.
The new `e9d9e543...109535` addon now includes this source and exposes all six
Mouse methods, with **26/26** public comparisons and **3/3** profile/Worker tests.
Source dependency changes alone remain insufficient evidence for native behavior.
The optional leaf still proves default/runtime **16/17**. No N-API, legacy bridge,
server/CLI/script or Playwright driver dependency enters the safe Rust host.

The new optional Keyboard contract layers have exact normal Pwright closures
of **3 capability / 6 client / 7 codec / 10 server** packages. None includes
Locator, a native backend, Tokio, Node or the schema generator. A dedicated CI
check verifies these sets and generated freshness. The raw input example remains
5 packages including itself and the unmodified safe host stays at 224 normal
packages. The separate `pwright-backend-cdp-keyboard` closure is exactly **8**:
itself, backend-common, backend-input, browser-backend, cap-keyboard, CDP, core,
and input-model. Its separate CI proof excludes Locator/runtime/Node; native
Keyboard is not installed into the aggregate host/FFI yet. The new optional
single-Page `pwright-composition-chromium-keyboard` adds wire/server dependencies
for an exact **16**-package normal closure, verified by its own CI script without
the full Chromium root, runtime, Locator or Node. Its new explicit `runtime`
feature adds exactly `pwright-runtime`, for **17** Pwright packages and no other
additional package. This enables target-aware controlled dispatch without
installing the aggregate host. Both selections are dependency-proofed in CI.
Independent production Page-state admission lives in the existing input leaf.
Its direct core dependency was already transitive; the input example and host
closures remain **5/224**, and Locator composition remains **18**. No new package
is required by the Node host's independent Page input selection.
[Detailed contract proof](pwright-raw-keyboard.md).

Opt-in interleaved input adds no dependency: the existing input leaf owns fixed
Page state, Locator's production profile selects a policy, and Chromium/Node
inject it at Page creation. Current proofs remain 5 input-only / 18 standalone
Locator-composition Pwright packages and 224 safe-host normal packages. No
default profile acquires a Node driver, browser process or new task.

Optional native-destruction receipts add no Cargo edge: the isolated resolver
accepts a caller-owned proof trait, Chromium paired access implements its native
identity source, and the Node host explicitly composes them. The 224-package
host and independent 5/7/15 package closures are unchanged; default profiles do
not install the receipt registry. See [the opt-in boundary](pwright-locator-evaluation.md#optional-native-destruction-receipts-2026-09-07).

Current host-owned Locator evaluation selects independent resolution/evaluation
compositions, bringing three previously omitted normal packages into the broad
host (two compositions plus resolution backend): **224** normal packages.
Browser-model and Handle-evaluation capability become direct host imports already
present transitively. No Node-API, driver or launcher enters the safe host.
Independent 5/7/15 closures and all 30 umbrella profiles still pass. Later 221
counts below are historical checkpoints. See
[the installation](pwright-locator-evaluation.md#host-owned-evaluation-continuation-2026-09-07).

Optional CDP event admission adds no Cargo edge: the raw client defines a
stateless callback contract, Chromium composition supplies the Frame-world
classifier, and Node explicitly selects it through existing dependencies.
Default/legacy profiles stay unfiltered. Exact 5/7/15 leaf closures, all 30
umbrella profiles and the 221-package Node boundary pass unchanged. A rebuilt
addon is required for the reader/profile behavior; dependency proofs alone do
not imply its failing full browser gates pass. See
[the event-admission checkpoint](../node-ffi-compatibility-design.md#optional-world-event-admission-checkpoint-2026-09-07).

Controlled runtime extension dispatch adds no Cargo edge. Its real preparation
integration imports browser-model only as another dev dependency of the existing
evaluation leaf. Normal leaf closures remain 5/7/15 packages; the Node host
remains 221 and does not install preparation. All 30 umbrella proofs pass. A
shared-runtime Node rebuild is required despite unchanged dependency membership;
its failing attachment-pressure gate is recorded separately from these proofs.

The optional paired selection/adoption phase lives in the existing Locator
evaluation leaf. It adds direct selector-model/runtime edges already present
in that leaf's exact fifteen-package normal closure. Its real production-port
integration imports the aggregate Chromium root and codec/server fixtures only
as dev-dependencies; no normal cycle or aggregate dependency is introduced.
Resolution retains five/seven-package closures. Node's normal boundary remains
221 packages and does not include these uninstalled preparation/evaluation
leaves. All 30 umbrella profiles and the exact leaf proofs pass.

The CI workflow now invokes both Locator-resolution and element-evaluation
proof scripts. They pass locally for exact five/seven/fifteen-package closures;
no remote CI run is claimed because this work has not been pushed.

Production paired-world access adds no Cargo edge: it is an explicitly
installed access port inside the existing aggregate composition, not a reason
for the independent resolution/evaluation leaves to depend on that aggregate.
The normal 5/7/15-package leaf closures, 221-package Node boundary and all 30
umbrella profiles pass unchanged. Node's profile does not install the accessor
or link the independent Locator evaluation leaf yet.

Main-world Locator element evaluation is a separate opt-in 15-package normal
closure, proved by `scripts/check-v2-locator-evaluation-dependencies.sh`. It
combines group-owned resolution/cleanup with the value and Handle-evaluation
backends but does not select a provider, aggregate root, portable Handle
lifecycle, codec/server family or Node. Ordinary Locator and resolution-only
consumers do not gain this dependency. The Node host still has 221 normal
packages and does not select the new leaf; shared CDP/adoption extensions are
opt-in methods, not default service installation.

The new opt-in Locator-resolution backend has an exact five-package normal
closure: itself, selector runtime/model, CDP and core. Its independent
continuation composition adds only itself and runtime, for seven packages.
Its optional strict waiter directly reuses selector validation/model and CDP
route types already in that closure. It injects route readiness, pacing and
termination instead of adding topology/provider/evaluation or a timer runtime.
`scripts/check-v2-locator-resolution-dependencies.sh` enforces both closures.
Neither is installed by ordinary Locator reads/actions or the current Node
host. Retained element resolution therefore adds no Handle/evaluation/provider
or aggregate dependency to a count/text consumer. The shared CDP trait gains
two group-owned primitives, not a default service installation. See
[Locator evaluation](pwright-locator-evaluation.md) for the ownership contract.

Node semantic Locator bindings reuse the already selected selector model and
command-free selector runtime; no crate edge is added. The runtime-7
accessible-name priority correction and constant-derived wrapper guards are
shared Rust-backend behavior, not a JS driver or a new public evaluation
dependency. The safe host's aggregate nature and future narrow native
distribution gate remain unchanged.

## Decision

Runtime capability selection is not enough for the v2 product boundary. A
consumer that installs only one capability must also avoid compiling unrelated
capability, backend, client, provider, tracing, Artifact, and browser-install
crates.

The enforced profiles include eight attach-only Chromium CDP server
compositions plus independently installable backend capability and topology
foundations:

```text
consumer
  -> pwright-backend-cdp-topology
       -> pwright-browser-model + pwright-cdp + pwright-core

consumer
  -> pwright-backend-cdp-context-creation
       -> pwright-backend-cdp-common
       -> pwright-cap-context-creation + pwright-cdp + pwright-core

consumer
  -> pwright-backend-cdp-context-lifecycle
       -> pwright-backend-cdp-common + pwright-backend-cdp-topology
       -> pwright-cap-context-lifecycle + pwright-cdp + pwright-core

consumer
  -> pwright-composition-chromium-context-lifecycle
       -> pwright-backend-cdp-context-lifecycle + pwright-backend-cdp-topology
       -> pwright-cap-context-lifecycle + pwright-codec-context-lifecycle
       -> pwright-runtime + pwright-server-context-lifecycle
       -> pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-composition-chromium-context-creation
       -> pwright-backend-cdp-context-creation
       -> pwright-composition-chromium-context-lifecycle
       -> pwright-cap-context-creation + pwright-codec-context-creation
       -> pwright-runtime + pwright-server-context-creation
       -> pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-backend-cdp-page-lifecycle
       -> pwright-backend-cdp-common
       -> pwright-cap-page-lifecycle + pwright-cdp + pwright-core

consumer
  -> pwright-backend-cdp-page-creation
       -> pwright-backend-cdp-common + pwright-backend-cdp-page-lifecycle
       -> pwright-cap-page-creation + pwright-cdp + pwright-core

consumer
  -> pwright-backend-cdp-handle-lifecycle
       -> pwright-backend-cdp-common
       -> pwright-cap-handle-lifecycle + pwright-cdp + pwright-core

consumer
  -> pwright-composition-chromium-handle-lifecycle
       -> pwright-backend-cdp-handle-lifecycle
       -> pwright-cap-handle-lifecycle + pwright-codec-handle-lifecycle
       -> pwright-runtime + pwright-server-handle-lifecycle
       -> pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-backend-cdp-handle-evaluation
       -> pwright-backend-cdp-evaluation + pwright-backend-cdp-common
       -> pwright-cap-handle-evaluation + pwright-cap-evaluation
       -> pwright-browser-model + pwright-cdp + pwright-core

consumer
  -> pwright-backend-cdp-page-events
       -> pwright-backend-cdp-topology
       -> pwright-cap-page-events + pwright-browser-model
       -> pwright-cdp + pwright-core

consumer
  -> pwright-composition-chromium-page-events
       -> pwright-backend-cdp-page-events + pwright-backend-cdp-topology
       -> pwright-cap-page-events + pwright-codec-page-events
       -> pwright-connection-control + pwright-runtime-control
       -> pwright-runtime + pwright-server-events + pwright-server-wire
       -> pwright-browser-model + pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-backend-cdp-network-events
       -> pwright-cap-network-events + pwright-network-model
       -> pwright-browser-model + pwright-cdp + pwright-core

consumer
  -> pwright-composition-chromium-network-events
       -> pwright-backend-cdp-network-events
       -> pwright-cap-network-events + pwright-codec-network-events
       -> pwright-network-model + pwright-runtime-control + pwright-runtime
       -> pwright-server-events + pwright-server-wire
       -> pwright-browser-model + pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-backend-cdp-input
       -> pwright-input-model + pwright-cdp + pwright-core

consumer
  -> pwright-composition-chromium-navigation
       -> pwright-backend-cdp-navigation
          -> pwright-backend-cdp-common
       -> pwright-cap-navigation
       -> pwright-codec-navigation
       -> pwright-server-navigation
       -> pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-composition-chromium-evaluation
       -> pwright-backend-cdp-evaluation
          -> pwright-backend-cdp-common
       -> pwright-cap-evaluation
       -> pwright-codec-evaluation
       -> pwright-server-evaluation
       -> pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-composition-chromium-dom-content
       -> pwright-backend-cdp-dom-content
          -> pwright-backend-cdp-common
       -> pwright-cap-dom-content
       -> pwright-codec-dom-content
       -> pwright-server-dom-content
       -> pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-composition-chromium-content
       -> pwright-backend-cdp-content
          -> pwright-backend-cdp-common
       -> pwright-cap-content + pwright-screenshot-model
       -> pwright-codec-content + pwright-codec-screenshot
       -> pwright-server-content
       -> pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-backend-cdp-selector-geometry
       -> pwright-backend-cdp-selector-runtime
       -> pwright-backend-cdp-locator-screenshot geometry contract
       -> pwright-cdp + pwright-core + pwright-selector-model
  -> pwright-composition-chromium-locator-screenshot
       -> pwright-backend-cdp-locator-screenshot
          -> pwright-backend-cdp-content + pwright-backend-cdp-common
       -> pwright-cap-locator-screenshot + selector/screenshot models
       -> pwright-codec-locator-screenshot + shared selector/screenshot codecs
       -> pwright-server-locator-screenshot
       -> pwright-cdp + pwright-core + pwright-transport
```

The navigation leaf accepts an existing `Arc<dyn CdpClient>` and one pre-provisioned Page
`ObjectId`. It creates the bounded CDP navigation service, a production-budgeted
wire protocol, and the generated two-command server handler chain. It starts no
task and does not select a runtime or transport. Browser discovery, browser
launch, managed installation, Node.js, Locator, tracing, and Artifact transfer
are absent.

The evaluation leaf binds one pre-provisioned Frame. A main Frame uses its
target-local default execution context; a child Frame can bind an explicit
CDP `uniqueContextId` whose retained route is included in the production
resource claim. It exposes only by-value evaluation: no navigation, Handle,
client, provider, installer, runtime, or browser acquisition crate is selected.

The DOM-content leaf also binds exactly one pre-provisioned Frame, but requires
that Frame's bounded utility-world CDP `uniqueContextId`. It evaluates only the
library-owned doctype-plus-`outerHTML` serializer and exposes only
`Frame.content`; public evaluation, screenshot, navigation, client, provider,
installer, runtime, and browser acquisition crates are absent. The retained
utility route is part of the production resource claim.

The content leaf binds exactly one pre-provisioned Page and exposes only
`Page.screenshot`. It supports the already-frozen non-mutating 1.0 option set,
returns one budgeted binary attachment, and validates the exact Page identity
before CDP. DOM access, public evaluation, Locator screenshot, navigation,
client, provider, installer, runtime, and browser acquisition crates are
absent.

The Locator-screenshot leaf binds one Page/Frame object to an injected
selector-to-geometry resolver and a target-local CDP capture client. The
resolver returns only validated viewport geometry; no DOM/CDP handle crosses
it. The first-party Chromium plugin is a separate dependency from the generic
capture composition and shares its command-free compiler/runtime with the full
Locator backend. Consequently the executable exact 23-package profile contains
neither `pwright-cap-locator` nor `pwright-backend-cdp`. Selector plugins
declare and claim their resources separately, and applications combine those
requirements concurrently with the capture profile before constructing the
shared ledger.

`pwright-backend-cdp-common` contains only command-free CDP failure
classification shared by navigation, evaluation, DOM content, and Page
screenshot. It owns no capability, service, route, resource ledger, runtime, or
provider authority.

`pwright-backend-cdp-topology` is the separately installable bounded
Browser→BrowserContext→Page→Frame/World state machine. It normalizes generated
CDP topology events, rejects stale/future epochs, emits portable lifecycle
effects, preserves popup/crash/reconnect semantics, and reserves exact
native-string state plus a transactional shadow. It performs no CDP command or
I/O and selects no capability, codec, client, server, runtime, provider,
launcher, installer, browser process, or aggregate.

The four creation/lifecycle leaves preserve Playwright-aligned native
boundaries without owning composition transactions. Context creation creates
one no-options, dispose-on-detach context and compensates an invalid returned
identity. Context lifecycle permits only owned contexts to reach
`Target.disposeBrowserContext`. Page lifecycle separates ordinary
`Target.closeTarget` plus bounded absence confirmation from
`Page.close` run-before-unload semantics. Page creation creates
`about:blank`, attaches its exact target, reads its main Frame, and delegates
all failure compensation to the Page lifecycle leaf. This explicit
page-creation → page-lifecycle edge has no reverse edge.

Handle lifecycle is a six-package leaf that retains one bounded native object
ID and issues exactly one `Runtime.releaseObject`. Retained Handle creation,
Handle-local value/Handle evaluation, `jsonValue`, `getProperty`, and bounded
`getProperties` remain together in the ten-package Handle-evaluation leaf
because they share private invocation, response parsing, and cleanup. The leaf
also contains the low-level Chromium ElementHandle-adoption primitive because
it reuses the same bounded native representation and parser. Adoption adds two
CDP methods but no package edge; public composition remains gated. The leaf
reuses the value-evaluation adapter; that dependency is one-way. Neither Handle
leaf selects navigation, Locator, content/screenshot, network events, topology,
Context/Page creation or lifecycle, the aggregate backend, codecs,
client/server, runtime, provider, installer, Node.js, or browser acquisition.

The Page-event leaf activates Page lifecycle delivery for one topology-routed
Page and normalizes only current main-Frame `load`, `PageReady`, and
`PopupLinked`. The network-event leaf maps one bounded Page/session HTTP lane
into atomic portable lifecycle/event/retirement batches, including redirects,
without retaining response bodies. Both own caller-driven normalization only:
no CDP receiver, queue, reader task, runtime, transport, browser session, or
process. Page events has the intentional topology edge; network events does
not. Their normal Pwright closures contain seven packages each.

`pwright-composition-chromium-page-events` binds that normalization to three
independently budgeted event queues, the ordered discovery fence, one
caller-polled load cursor, and an injected transport. Its exact 15-package
closure adds only the Page-event codec, generic event/server-wire machinery,
runtime control needed by the creation fence, and transport boundary. It has
no aggregate Chromium backend, command capability, client, provider, Tokio,
launcher, installer, Node.js, or browser acquisition edge. The aggregate
complete recipe consumes and compatibility-reexports this leaf.

`pwright-composition-chromium-network-events` owns the single cross-event
queue, monotonic sequence, routed codec, production resources, creation fence,
and retained receive/publish/acknowledge backend. Its generic batch-driver port
depends on the normalized CDP batch contract but not a receiver; its object-safe
scope selector lets the aggregate preserve dynamic Page/BrowserContext listener
semantics without moving topology or listener mutation into the leaf. The exact
16-package standalone closure includes runtime/control and server-wire for this
causal turn, but has no aggregate Chromium backend, client, provider, Tokio
executor, launcher, installer, Node.js, or browser acquisition. The complete
recipe consumes it, and the aggregate implements both ports while retaining its
old publisher/backend as compatibility APIs.

The Page-input leaf keeps keyboard layout resolution and the serialized
keyboard/mouse transaction state together because each requires the other's
modifier representation. Their shared 16-key chord bound is independently
owned by command-free `pwright-input-model`, so the portable Locator contract
and CDP backend agree without either depending on the other. The leaf accepts a
caller-owned CDP client only when beginning a transaction and owns no selector,
Locator command, capability, runtime, transport, task, browser session, or
process. Its normal Pwright closure contains exactly five packages.

The sequential-typing primitive adds only a borrowed input model and injected
delay-future iteration within this same leaf. `serde`, `serde_json` and `url`
are added only as test dependencies for the pinned live oracle; no production
dependency edge changes. The five-package normal closure is reverified. See
[typing evidence and public binding](pwright-sequential-typing.md). The typed
Locator/host/FFI integration adds no normal dependency edge; the independent
Locator composition proof remains 18 Pwright packages and the safe Node host
boundary remains 224 normal packages.

The complete Locator backend is the independently installable
`pwright-backend-cdp-locator` leaf. It composes the portable Locator contract,
command-free selector runtime, and Page-input foundation over a caller-owned
target CDP client. Screenshot capture remains a separate capability: the
compatibility aggregate alone adapts the Locator leaf's bounded internal
geometry result to the screenshot resolver contract. The Locator leaf owns no
screenshot, aggregate backend, runtime, transport, server, provider, launcher,
browser session, or process. Its normal Pwright closure contains exactly ten
packages, including the proof consumer. The additional command-free
`pwright-backend-cdp-common` edge shares the existing native failure classifier:
a closed CDP session must not become a generic Locator backend error. It adds
no browser capability, task, queue or runtime.

`pwright-composition-chromium-locator` adds only the generated Locator server
and bounded wire codec around that backend. Each installation retains exactly
one protocol table—Locator 1.22 or 2.5—and one explicit Page or Frame route.
The caller injects the target CDP client and Page-owned input coordinator;
unique-context Frame installations additionally hold the declared route lease.
It owns no runtime, transport task, screenshot/navigation/evaluation
capability, provider, launcher, browser session, or process. Its standalone
normal Pwright closure contains exactly eighteen packages, including the proof
consumer and the same command-free failure-classification leaf.

`pwright-composition-chromium-context-lifecycle` closes the gap between the
native Context-lifecycle backend and a usable generated server leaf. It stages
the exact owned-context topology transition, child-first runtime disposal, and
one caller-owned continuation slot before issuing
`Target.disposeBrowserContext`. A single injected retirement plugin can
prepare Page-route, Handle, network, or composite cleanup without adding those
crates to the leaf; its commit token is infallible and publishes only after the
native close and core ownership commits. The standalone normal Pwright closure
contains exactly seventeen packages and excludes the aggregate backend and
composition, clients, providers, launchers, installers, Tokio, Node.js, and
browser acquisition.

`pwright-composition-chromium-context-creation` pairs the generated
`Browser.newContext` handler with that required lifecycle installation. It
allocates portable identity, pre-admits an injected publication plugin,
prepares runtime ownership, creates the native Context, and commits the exact
topology/runtime/plugin publication. Any post-CDP publication failure disposes
the returned native identity through the lifecycle backend. Creation and close
have separate caller-owned continuation capacity. Its exact 22-package normal
closure contains no aggregate backend/composition, client, provider, launcher,
installer, Tokio, Node.js, or browser acquisition.

`pwright-composition-chromium-page-lifecycle` is the parallel Page-only leaf.
It composes the generated two-command server, bounded binary codec, native
close confirmation, production topology, and controlled runtime in exactly
seventeen Pwright packages. A target-session resolver supplies only the
Page-local client needed by before-unload, and one retirement plugin owns any
optional Page-route, Handle, or network cleanup. Its exact dependency proof
excludes the aggregate backend/composition, clients, providers, launchers,
installers, Tokio, Node.js, and browser acquisition.

`pwright-composition-chromium-page-creation` explicitly depends on that leaf
and exposes the generated creation and lifecycle handlers together. Its
pre-CDP publication reservation can provisionally bind the returned native
target/session/main-frame route, while drop remains rollback and final commit
is infallible. Runtime publishes Page and Frame as one batch; every later
failure closes the exact native target. The standalone normal closure contains
exactly twenty-two Pwright packages and excludes aggregates, clients,
providers, launchers, installers, Tokio, Node.js, and browser acquisition.

`pwright-composition-chromium-handle-lifecycle` owns the bounded shared route
registry consumed by optional Handle producers. A producer pre-admits one
route and its maximum retained bytes, then publishes exact ownership, an
opaque typed backend payload, and an injected object-local disposer. The leaf
therefore installs generated `Handle.dispose`, runtime retirement, and
caller-owned cancellation continuation without depending on Handle or value
evaluation. Its standalone normal closure contains exactly fifteen Pwright
packages and excludes topology, Context/Page capabilities, aggregate crates,
clients, providers, launchers, installers, Tokio, Node.js, and browser bytes.

`pwright-composition-chromium-handle-evaluation` depends on that lifecycle leaf
and adds bounded Frame execution-world routes plus all six generated 1.2
handlers. Its exact 24-package normal closure excludes aggregate crates,
topology, Context/Page commands, clients, providers, launchers, installers,
Tokio, Node.js, and browser acquisition. Lifecycle remains independently
selectable and value-only evaluation still has no reverse Handle dependency.

`pwright-plugin-ordered-handle-delivery` is the independent remote policy above
those services. Its exact 15-package closure contains only the Handle
capability/codec/server path, core runtime/control, and transport traits. It
contains no CDP/backend/provider, Chromium composition, client, launcher,
installer, Tokio adapter, Node.js, or browser bytes. Direct Handle evaluation
therefore stays a 24-package Chromium leaf, while remote consumers opt into the
ordering and Result-delivery fence explicitly.

The existing `pwright-backend-cdp` and `pwright-composition-chromium` remain
compatibility aggregates. The aggregate backend reexports the extracted
topology, Context/Page creation and lifecycle, Handle, Page/network event,
evaluation, DOM-content, and Page-screenshot adapters and wraps the extracted
navigation adapter for existing users. The composition aggregate depends
downward on the independent Handle-evaluation leaf: its complete recipe uses
the leaf registry and command services, while aggregate Page services provide
the live Frame/world resolver and parent/topology retirement bridge. Optional
ElementHandle adoption remains a separately budgeted backend choice. No
isolated leaf depends upward on either aggregate.

## Representative consumer

`examples/v2-topology-only/Cargo.toml`,
`examples/v2-context-creation-only/Cargo.toml`,
`examples/v2-context-lifecycle-only/Cargo.toml`,
`examples/v2-context-lifecycle-composition-only/Cargo.toml`,
`examples/v2-context-creation-composition-only/Cargo.toml`,
`examples/v2-page-creation-only/Cargo.toml`,
`examples/v2-page-creation-composition-only/Cargo.toml`,
`examples/v2-page-lifecycle-only/Cargo.toml`,
`examples/v2-page-lifecycle-composition-only/Cargo.toml`,
`examples/v2-handle-lifecycle-only/Cargo.toml`,
`examples/v2-handle-lifecycle-composition-only/Cargo.toml`,
`examples/v2-handle-evaluation-only/Cargo.toml`,
`examples/v2-handle-evaluation-composition-only/Cargo.toml`,
`examples/v2-ordered-handle-delivery-only/Cargo.toml`,
`examples/v2-input-only/Cargo.toml`,
`examples/v2-locator-only/Cargo.toml`,
`examples/v2-locator-composition-only/Cargo.toml`,
`examples/v2-page-events-only/Cargo.toml`,
`examples/v2-network-events-only/Cargo.toml`,
`examples/v2-navigation-only/Cargo.toml`,
`examples/v2-evaluation-only/Cargo.toml`, and
`examples/v2-dom-content-only/Cargo.toml`,
`examples/v2-content-only/Cargo.toml`, and
`examples/v2-locator-screenshot-only/Cargo.toml` are intentionally separate
workspaces so the root workspace cannot unify optional features into any
proof:

Each manifest has exactly one dependency:

```toml
# v2-topology-only
pwright-backend-cdp-topology = { path = "../../crates/pwright-backend-cdp-topology" }

# v2-context-creation-only
pwright-backend-cdp-context-creation = { path = "../../crates/pwright-backend-cdp-context-creation" }

# v2-context-lifecycle-only
pwright-backend-cdp-context-lifecycle = { path = "../../crates/pwright-backend-cdp-context-lifecycle" }

# v2-context-lifecycle-composition-only
pwright-composition-chromium-context-lifecycle = { path = "../../crates/pwright-composition-chromium-context-lifecycle" }

# v2-context-creation-composition-only
pwright-composition-chromium-context-creation = { path = "../../crates/pwright-composition-chromium-context-creation" }

# v2-page-creation-only
pwright-backend-cdp-page-creation = { path = "../../crates/pwright-backend-cdp-page-creation" }

# v2-page-lifecycle-only
pwright-backend-cdp-page-lifecycle = { path = "../../crates/pwright-backend-cdp-page-lifecycle" }

# v2-handle-lifecycle-only
pwright-backend-cdp-handle-lifecycle = { path = "../../crates/pwright-backend-cdp-handle-lifecycle" }

# v2-handle-evaluation-only
pwright-backend-cdp-handle-evaluation = { path = "../../crates/pwright-backend-cdp-handle-evaluation" }

# v2-input-only
pwright-backend-cdp-input = { path = "../../crates/pwright-backend-cdp-input" }

# v2-locator-only
pwright-backend-cdp-locator = { path = "../../crates/pwright-backend-cdp-locator" }

# v2-locator-composition-only
pwright-composition-chromium-locator = { path = "../../crates/pwright-composition-chromium-locator" }

# v2-page-events-only
pwright-backend-cdp-page-events = { path = "../../crates/pwright-backend-cdp-page-events" }

# v2-network-events-only
pwright-backend-cdp-network-events = { path = "../../crates/pwright-backend-cdp-network-events" }

# v2-navigation-only
pwright-composition-chromium-navigation = { path = "../../crates/pwright-composition-chromium-navigation" }

# v2-evaluation-only
pwright-composition-chromium-evaluation = { path = "../../crates/pwright-composition-chromium-evaluation" }

# v2-dom-content-only
pwright-composition-chromium-dom-content = { path = "../../crates/pwright-composition-chromium-dom-content" }

# v2-content-only
pwright-composition-chromium-content = { path = "../../crates/pwright-composition-chromium-content" }

# v2-locator-screenshot-only
pwright-backend-cdp-selector-geometry = { path = "../../crates/pwright-backend-cdp-selector-geometry" }
pwright-composition-chromium-locator-screenshot = { path = "../../crates/pwright-composition-chromium-locator-screenshot" }
```

Every isolated composition now links a dedicated backend directly:

```toml
pwright-backend-cdp-navigation = { workspace = true }
pwright-backend-cdp-evaluation = { workspace = true }
pwright-backend-cdp-dom-content = { workspace = true }
pwright-backend-cdp-content = { workspace = true }
pwright-backend-cdp-locator = { workspace = true }
pwright-backend-cdp-context-lifecycle = { workspace = true }
pwright-backend-cdp-context-creation = { workspace = true }
```

## Enforcement

`scripts/check-v2-topology-dependencies.sh`,
`scripts/check-v2-context-creation-dependencies.sh`,
`scripts/check-v2-context-lifecycle-dependencies.sh`,
`scripts/check-v2-context-lifecycle-composition-dependencies.sh`,
`scripts/check-v2-context-creation-composition-dependencies.sh`,
`scripts/check-v2-page-creation-dependencies.sh`,
`scripts/check-v2-page-lifecycle-dependencies.sh`,
`scripts/check-v2-handle-lifecycle-dependencies.sh`,
`scripts/check-v2-handle-evaluation-dependencies.sh`,
`scripts/check-v2-ordered-handle-delivery-dependencies.sh`,
`scripts/check-v2-input-dependencies.sh`,
`scripts/check-v2-locator-dependencies.sh`,
`scripts/check-v2-locator-composition-dependencies.sh`,
`scripts/check-v2-page-events-dependencies.sh`,
`scripts/check-v2-page-events-composition-dependencies.sh`,
`scripts/check-v2-network-events-dependencies.sh`,
`scripts/check-v2-network-events-composition-dependencies.sh`,
`scripts/check-v2-navigation-dependencies.sh`,
`scripts/check-v2-evaluation-dependencies.sh`, and
`scripts/check-v2-dom-content-dependencies.sh`,
`scripts/check-v2-content-dependencies.sh`, and
`scripts/check-v2-locator-screenshot-dependencies.sh` inspect only normal Cargo edges
from their representative consumers. Each allowlists every permitted Pwright
crate, so a future dependency fails rather than silently expanding the
distribution. CI runs all twenty-seven dependency proofs after workspace
checking. The topology
foundation contains exactly five Pwright packages. Context creation and Page
lifecycle contain 6 each; Context lifecycle and Page creation contain 8 each.
Handle lifecycle contains 6; Handle evaluation contains 10 because it reuses
the value-evaluation backend and its portable evaluation/browser contracts.
Page events and network events contain 7 each; only Page events includes the
topology foundation.
The complete Page-event composition contains 15 packages and retains no
aggregate, command, client, provider, Tokio, browser-launch, or install edge.
The Network-event lifecycle composition contains 16 packages. It intentionally
includes the normalized CDP batch contract and runtime/control creation fence,
but retains no CDP receiver, aggregate, client, provider, Tokio executor,
browser-launch, or install edge. Dynamic Page/BrowserContext subscription
selection and the single-cursor driver are injected ports; their state and
mutation remain aggregate-owned.
Page input contains 5: its consumer, input backend, input model, CDP, and core.
Locator contains 9: its consumer, Locator backend, input and selector-runtime
foundations, Locator contract, CDP, core, and the input/selector value models.
The complete Locator composition contains 17 packages and still excludes the
aggregate backend, runtime, provider, launcher, installer, and unrelated
capabilities.
The complete Context-lifecycle composition also contains 17 packages; runtime
and topology are intentional ownership dependencies, while Page routes,
Handles, network events, clients, providers, browser launch, and the aggregate
backend remain absent.
The complete paired Context-creation composition contains 22 packages. Its five
additional creation capability/codec/backend/server/composition packages reuse
the Context-lifecycle leaf as an explicit dependency instead of duplicating or
hiding close authority.
The current exact normal
Pwright closures contain 15 packages each for navigation,
evaluation, and DOM content, while Page screenshot contains 17 because its
portable binary result and attachment codec are separately reusable leaves.
Locator screenshot contains 23 because its independently installable
selector-runtime and first-party selector-geometry leaves join the separately
reusable selector/screenshot models and codecs, while public Locator commands
and the aggregate backend remain forbidden.

The topology leaf retains the 36 pure and production-state tests that formerly
lived inside the aggregate backend. They cover bounded identity maps, deferred
materialization, OOPIF swaps/detach, popup ordering, crash and reconnect
reconciliation, generated-event validation, complete portable runtime effects,
exact native-byte accounting, resource admission, and rollback/commit
transactions. Aggregate backend tests compile against its compatibility
reexports.

The 17 creation/lifecycle AAA tests stay with their new implementation owners:
4 for Context creation, 5 for Context lifecycle, 2 for Page creation, and 6 for
Page lifecycle. They cover exact CDP calls, ownership rejection before I/O,
native-ID bounds, invalid-response compensation, target-absence confirmation,
run-before-unload separation, and portable error classification.
Two dedicated Chrome 152 tests additionally pass through aggregate
compatibility reexports: one closes an explicit Page and preserves its parent
Context/browser; the other creates and closes a real Context and Page while
verifying main-Frame initialization and external-browser survival.
The independent Context-lifecycle composition has five additional AAA tests
for coordinated commit, complete rollback, unowned-context rejection, plugin
ordering, generated-handler construction, and cancellation-safe continuation.
Its dedicated Chrome 152 test closes one owned temporary Context through the
new leaf and then proves the external browser is still alive.
The independent paired Context-creation composition owns six additional AAA
tests for success, rollback, compensation, plugin admission, and both
continuation boundaries. Its dedicated Chrome proof creates and closes the
same Context through the paired services, observes native removal, and leaves
the externally owned browser running.

The two Handle composition leaves own 33 direct tests: 14 for lifecycle route
admission/disposal/retirement and 19 for retained Handle creation, local
operations, bounds, adoption, atomic cleanup, and failure classification. The
aggregate live Handle test now runs the complete leaf-backed recipe against
Chrome 152 and verifies typed lifecycle announcements, Handle-valued invocation,
native release, graceful close, and external-browser survival.

The Page/network event leaves own 10 AAA tests: 4 for lifecycle activation,
main-Frame load filtering, and topology-effect mapping; 6 for HTTP lifecycle
ordering, redirects, failure/session distinction, input bounds, teardown, and
production resource release.
Three aggregate-compatibility live tests pass against Chrome 152: Page load is
read through a caller-owned cursor; bounded HTTP request/response/finish is
observed without response-body ownership; and redirect, HTTP-error, and
transport-failure semantics remain distinct. Each test disposes only its
temporary context and verifies external Chrome remains live.

The extracted navigation backend descriptor exposes only
`navigation.begin` and `navigation.goto`, with no event features. Leaf and
aggregate-compatibility tests cover production-ledger installation, real
service delegation through a fake CDP client, cancellation, bounded event
state, error classification, wrong-Page rejection, and construction of the
generated handler chain. A dedicated Chrome 152 test calls the isolated
backend directly, completes a real data-URL navigation, disposes only its
temporary browser context, and verifies that the externally owned browser
remains available.

The extracted evaluation backend descriptor exposes only
`evaluation.evaluate-value`, with no event features. Composition tests cover
main-Frame and unique-context routing, held route resources, wrong-Frame
rejection before CDP, and generated-handler construction.

The extracted DOM-content backend descriptor exposes only `frame.content`.
Composition tests cover exact utility-context routing and serialization,
retained result and route accounting, rejection of a wrong Frame before CDP,
required route budgeting, and generated-handler construction.

The extracted Page-screenshot backend descriptor exposes only the native
`page.screenshot` feature. Composition tests cover exact Page routing, retained
source-result accounting, backend/codec limit agreement, and generated-handler
construction.

## Umbrella profiles

The public `pwright` crate is now a no-default-features re-export layer. It does
not replace independently usable leaf crates. Its first executable feature
axes are:

```text
navigation            portable command contract
evaluation            portable by-value evaluation contract
content               portable Page screenshot contract
context-lifecycle     portable BrowserContext close contract
context-creation      portable Context creation + required lifecycle contract
page-lifecycle        portable Page close/before-unload contract
page-creation         portable Page creation + required lifecycle contract
handle-lifecycle      portable JSHandle/ElementHandle disposal contract
handle-evaluation     portable retained-Handle creation and local operations
ordered-handle-delivery
                      remote Handle ordering and Result-abandonment policy
locator-screenshot    portable Locator screenshot contract
dom-content           portable read-only Frame DOM contract
client-navigation     client + navigation + Page extension traits
client-evaluation     client + evaluation + Frame/Page extension traits
client-content        client + Page screenshot extension trait
client-context-lifecycle
                      client + BrowserContext close extension trait
client-context-creation
                      client + Browser creation + required lifecycle client
client-page-lifecycle client + Page close/before-unload extension traits
client-page-creation  client + Page creation + required lifecycle client
client-handle-lifecycle
                      client + JSHandle/ElementHandle dispose extension trait
client-handle-evaluation
                      client + Handle evaluation + required lifecycle client
client-locator        client + Playwright-shaped Locator factories/actions
client-locator-screenshot
                      client + Locator screenshot extension trait
client-dom-content    client + DOM content + Frame/Page extension traits
cdp                   raw CDP client
chromium-navigation   CDP + navigation + minimal Chromium server leaf
chromium-evaluation   CDP + evaluation + minimal Chromium server leaf
chromium-content      CDP + Page screenshot + minimal Chromium server leaf
chromium-context-lifecycle
                      CDP + runtime/topology Context-close server leaf
chromium-context-creation
                      CDP + runtime/topology paired create/close server leaf
chromium-page-lifecycle
                      CDP + runtime/topology Page-close server leaf
chromium-page-creation
                      CDP + runtime/topology paired Page create/close server leaf
chromium-handle-lifecycle
                      CDP + runtime + bounded Handle-route/disposal server leaf
chromium-handle-evaluation
                      CDP + runtime + Handle 1.2 + required lifecycle server leaf
chromium-locator      CDP + Locator 1.22/2.5 + minimal Chromium server leaf
chromium-locator-screenshot
                      CDP + Locator screenshot orchestration + first-party selector plugin
chromium-selector-geometry
                      CDP + standalone first-party selector-to-geometry plugin
chromium-dom-content  CDP + DOM content + minimal Chromium server leaf
runtime-tokio         executor-neutral runtime + Tokio adapter
transport-tokio       transport contract + Tokio byte-stream adapter
tokio                 convenience selection of both Tokio adapters
```

`examples/v2-umbrella-profiles` contains thirty isolated packages: empty;
client Context/Page creation, Context/Page/Handle lifecycle, Handle evaluation,
Locator, content, Locator screenshot, DOM content, evaluation, navigation, and
Page/network events;
the backend-neutral ordered Handle-delivery plugin; plus their currently
packaged Chromium server counterparts, including Handle lifecycle and Handle
evaluation and Page/network events.
`scripts/check-v2-umbrella-dependencies.sh`
compares each package's complete normal Pwright dependency closure with an
exact allowlist and checks its feature edges. In particular:

- the empty umbrella has no Pwright dependency beyond itself;
- client Context creation/lifecycle, Locator, content, Locator screenshot, DOM content,
  evaluation, and navigation
  have no CDP, backend, codec, or server crate;
- Chromium Context lifecycle contains its intentional topology/runtime
  ownership boundaries and generated server but no Page-route, Handle,
  network, aggregate-backend, client, provider, launcher, or installer crate;
- Chromium Context creation contains the required lifecycle leaf and its
  topology/runtime boundaries but no aggregate backend, client, provider,
  launcher, installer, Tokio, Node.js, or browser acquisition;
- Chromium Page creation/lifecycle preserve the same paired dependency edge
  and omit Context, Handle, aggregate, client, provider, launcher, installer,
  Tokio, Node.js, and browser acquisition crates;
- Chromium Handle lifecycle owns runtime and its bounded route/disposal leaf,
  but contains no Handle evaluation, value evaluation, topology, Page/Context,
  aggregate, client, provider, launcher, installer, Tokio, Node.js, or browser
  acquisition crate;
- client Page events contains only its portable capability, codec, generic
  client event/wire machinery, core, and transport; Chromium Page events owns
  the exact 15-package server composition above and no aggregate, unrelated
  command, client, provider, Tokio, launcher, installer, Node.js, or browser
  acquisition crate;
- client Network events contains only its portable capability/model, codec,
  generic client event/wire machinery, core, and transport; Chromium Network
  events owns the exact 16-package lifecycle composition, including the
  normalized CDP batch contract and runtime/control fence, but no CDP receiver,
  aggregate, unrelated command, client, provider, Tokio executor, launcher,
  installer, Node.js, or browser acquisition crate;
- Chromium Locator contains its dedicated backend and generated server but no
  aggregate backend, client, runtime, provider, launcher, installer, or
  unrelated capability;
- Chromium Locator screenshot has neither public Locator commands nor the
  aggregate CDP backend; selector resolution stays an injected plugin;
- Chromium content has no client, DOM content, public evaluation, Locator,
  navigation, aggregate CDP backend, provider, launcher, or installer;
- Chromium DOM content has no client, evaluation, screenshot, navigation,
  aggregate CDP backend, provider, launcher, or installer;
- Chromium evaluation has no client, navigation, aggregate CDP backend,
  provider, launcher, or installer;
- Chromium navigation has no client, provider, launcher, installer, unrelated
  capability, or aggregate backend.

The umbrella intentionally has no `full` feature while the supported feature
surface is incomplete.

Each optional umbrella dependency has both a repository path and an exact
current-series `0.1.0` version requirement. Cargo can normalize and list the
package contents now. Upload verification cannot resolve the package until the
unpublished v2 leaf crates exist in the registry, so release order is leaves
first and umbrella last. Current users should use the Git dependency shown in
the umbrella README.

## Resource boundary

All eight minimal composition profiles combine their CDP adapter's bounded
operation-state peak with one simultaneous wire-encoding peak. Evaluation also
includes its optional retained unique-context route. DOM content includes the
persistent utility route, one potentially retained backend `DomContentBytes`
result, and one separately decoded public-protocol `DomContentBytes` result;
the codec therefore has no hidden private result ledger. Installation claims
everything from the caller's shared `ProductionResourceLedger`; an omitted or
undersized plan fails before a command can run.

Page screenshot similarly accounts the backend-owned `ScreenshotBytes` result,
one separately decoded public-protocol result, and the binary wire attachment
as simultaneously live owners. Backend and codec byte ceilings must match, and
installation rejects an outbound attachment profile that cannot carry the
declared maximum.

Locator screenshot adds the same capture/result/wire overlap but deliberately
does not absorb selector-plugin resources. The application must combine the
plugin's declared requirements concurrently with this composition profile.

Creation/lifecycle leaves retain only bounded native identities. Page close and
Page-creation compensation also cap target-absence confirmation attempts.
Their caller composition owns the larger transactional topology/runtime/route
resource plan; the native leaves do not create a second ledger or queue.
The Context-lifecycle composition claims only its wire resources directly;
the injected topology and runtime retain their own production claims, and an
optional retirement plugin must pre-admit its resources before native close.
The paired Context-creation profile combines both simultaneously retained
protocol tables. Its publication plugin must reserve fallible optional state
before native creation and expose only an infallible final commit.

## Current limitation

This checkpoint proves command-free topology, native creation/lifecycle, and
capability-level Cargo modularity, not the final minimum binary. `pwright-cdp`,
`pwright-server-wire`, and their transport dependencies remain shared
infrastructure and can be split further only after their own independently
useful boundaries are designed. Remaining capability-specific Chromium leaves
and umbrella features remain future DT-041/DT-043 work.
