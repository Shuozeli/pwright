<!-- agent-updated: 2026-09-05T22:05:06Z -->

# Pwright network lifecycle

> Status: implemented optional Page- and BrowserContext-scoped lifecycle
> foundations, including one single-cursor driver, transactional Context
> fan-out, explicit `BrowserContext.newPage` inheritance, a cross-event ordered
> remote stream, and a bounded client handler. New-Page automatic discovery is
> joined to the same transaction. Session replacement uses one pre-budgeted
> shadow adapter, preserves direct and inherited interest, cleans old active
> Request roots in the runtime rebind, and rolls back or recovers explicitly.
> Page-session detach retires topology, runtime, subscription state, and the
> network lane under one commit guard. The public Page and BrowserContext
> listener helpers share the same control protocol, and the driver now exposes
> one owned receive/publish/acknowledge turn. Terminal events now carry a
> complete immutable Request plus optional Response snapshot and retire their
> temporary runtime objects immediately after all selected event queues accept
> the batch. The v2 production connection can select a task-free,
> reclaim-after-all-read CDP fanout whose subscriber capacity is derived from
> installed components. The globally ordered remote queue and its retained
> causal lifecycle turn are now an independent 16-package composition leaf
> consumed by the complete recipe. This
> does not approve response-body capture, routing,
> WebSocket inspection, or HAR
> recording.

## Why this is a separate capability family

Playwright presents related network APIs, but they have different authority
and radically different retention costs:

| Concern | Authority | Retained data | Pwright capability |
| --- | --- | --- | --- |
| HTTP lifecycle observation | observe Request/Response transitions | bounded metadata and stable identities | `pwright.network-events` |
| Response body | read bytes after headers and before backend expiry | caller-requested bounded bytes or chunks | future `pwright.response-body` |
| Request routing | pause and decide a request exactly once | live Route decision state | future `pwright.network-routing` |
| WebSocket | observe or proxy an open-ended stream | bounded frame queues | future `pwright.websocket` |
| HAR | record or replay exchanges | bounded spill/storage state | future `pwright.har` plugin |

Installing lifecycle observation must not install any of the other four. In
particular, it must never read response bodies, base64-encode traffic, retain a
complete history, write files, or start a task by itself.

## Reference behavior

The local Playwright source establishes these semantic anchors:

- a normal exchange is `request` -> `response` -> `requestfinished`;
- a transport failure or abort ends with `requestfailed`, while HTTP 4xx/5xx
  still use the successful response/finished path;
- response means headers are available, not that the body is complete;
- every redirect creates a new Request/Response pair linked to the previous
  Request rather than mutating one object through the whole chain;
- Request dispatchers are created only for a selected listener, continuation
  of an already-reported redirect, or navigation identity;
- a Response dispatcher is a child of its Request dispatcher.

Source anchors are
`packages/playwright-core/src/server/dispatchers/browserContextDispatcher.ts`,
`packages/playwright-core/src/server/dispatchers/networkDispatchers.ts`, and
`packages/protocol/spec/network.yml` in the checked-out upstream Playwright
repository.

The local Playwright `1.63.0-next` checkout at commit
`de214f440b7e34937fe4886f046b78b757136087` also exposes a concrete retention
cost. Request and Response dispatchers are reused for terminal events and are
not disposed by `requestFinished` or `requestFailed`. Both use the generic
per-type dispatcher bucket, whose default threshold is 10,000; when a bucket
overflows, Playwright disposes the oldest 10%. A long-lived connection can
therefore retain roughly 10,000 Request plus 10,000 Response dispatchers and
their connection/object-map entries before this safety GC runs. This is a
plausible, source-verified contributor to memory growth, but has not yet been
isolated as the only or dominant cause.

## Portable object and event contract

`Request` and `Response` are semantic browser objects, not CDP request IDs or
JSON records. The base object catalog adds only these two kinds:

```text
BrowserContext | Page | Frame
              `-- Request
                    `-- Response
```

A Request may be parented to the Page when one is known, otherwise to its
Frame or BrowserContext. A Response is always parented to its Request. Native
request/session/loader identifiers stay inside a backend route table and never
cross the portable API.

The first event capability owns four Playwright-shaped events. Each may target
a Page or BrowserContext subscription and refers to the same Request/Response
object identities when both scopes are subscribed:

- `request` carries the Request initializer;
- `response` carries the Response initializer and its Request identity;
- `requestfinished` carries complete Request and optional Response value
  snapshots plus bounded completion timing;
- `requestfailed` carries complete Request and optional Response value
  snapshots, bounded failure text, and bounded completion timing.

Object `Create` lifecycle control must be physically written before an event
that exposes that object. Redirect links can name only an object already
created on the same connection. A terminal event contains the complete
immutable Request metadata and optional Response metadata in addition to its
completion or failure fields. It remains usable after the corresponding
temporary runtime objects expire.

## Retention and lifecycle decision

Network identity is opt-in and lifecycle-bounded. The backend creates temporary
portable Request/Response objects only when at least one selected network
listener or a separate operation such as navigation requires them. It keeps no
passive history when the capability is absent.

The implementation uses three distinct bounds:

1. active native request records, including redirect correlation;
2. live, in-flight portable Request/Response objects in the runtime ownership
   tree;
3. per-event queue items and aggregate metadata bytes.

The codec computes a conservative maximum terminal payload and maximum retained
metadata value from the same lifecycle limits. Composition rejects an outbound
frame limit or per-event byte budget that cannot hold that maximum, before CDP
attachment or browser traffic begins.

Backend-native correlation state is removed immediately at the terminal event.
The tracker retains a compact copy of the in-flight Request and optional
Response metadata under the active-request byte budget so it can construct a
self-contained terminal snapshot. Publication first admits every selected
semantic event; only after all queues accept the batch does it commit one
Request-root Dispose, with the runtime retiring any Response child first.

The base `pwright.network-events` contract therefore guarantees live handles
only during the in-flight lifecycle. A terminal event is a value snapshot, not
a promise that its historical IDs remain callable. Since the snapshot has no
live-object dependency, the control Dispose and terminal event may be
physically drained by independent outbound lanes without creating a dangling
payload. Queue rejection happens before retirement, is connection-terminal,
and leaves teardown responsible for the still-live graph rather than silently
losing an event.

Playwright-style post-terminal Request/Response handles belong in a separately
installed client-acknowledged lease capability. That capability must declare
hard object/byte limits, disconnect and abandoned-consumer cleanup, and
backpressure before exceeding capacity. A bounded recent-history ring without
acknowledgment does not prove that a queued or slow consumer still owns a live
handle, so it is not sufficient by itself.

The dedicated Chrome 152 workload now proves this default on one persistent
Page: three complete real Request/Response lifecycles produced steady
`LiveObject.current` values of `[4, 4, 4]` (Browser, Context, Page, Frame), with
zero native correlation and no terminal Request/Response descendants. The
observed runtime RSS was 8,388,608 to 10,059,776 bytes and the browser process
tree changed from 10 to 8 processes; these are single-run diagnostics, not
accepted memory budgets.

## Backpressure and failure policy

HTTP lifecycle is state, not lossy telemetry. Every portable event lane uses
`RejectNewest`. Any raw CDP cursor lag, portable queue rejection/eviction,
invalid transition, unknown redirect predecessor, or resource exhaustion makes
the network view incomplete and therefore terminates that lane with a typed
reconnect/resubscribe requirement. It never silently drops an event and
continues.

All strings and header collections are validated before copying. Production
profiles must declare at least:

- active request count;
- native ID, URL, method, resource-type, failure-text, and status-text bytes;
- request/response header count and aggregate name/value bytes;
- post-data metadata bytes when that optional field is selected;
- event queue capacity and maximum retained bytes per event;
- portable Request/Response object counts and backend-state bytes.

Response body bytes are not part of this profile. A future body capability
must reserve `ResponseBodyBytes` before allocation and use either a typed
bounded result or an explicitly backpressured chunk stream. Truncating a body
and pretending it is complete is forbidden.

## CDP backend rules

The Chromium adapter reuses the Page's existing flattened CDP session; it does
not attach a hidden second session per listener. The explicit Page-lane factory
subscribes and registers its cursor without enabling Network. The ordered
subscription activator performs the native transition only after that exact
Page/session lane is ready, and must not disable a domain still required by
navigation or another installed capability.

### Page subscription and Network-domain ownership

The next Page-scoped slice uses two separate pieces of state. They must not be
collapsed into a single listener count:

1. a **lane registration** proves that one caller-polled CDP cursor and one
   bounded normalizer already exist for the Page's exact current session;
2. a **logical interest mask** records which of `request`, `response`,
   `requestfinished`, and `requestfailed` the remote peer selected.

Lane registration happens before `Network.enable`. The first logical interest
is rejected when no matching lane exists, so a successful subscription can
never create a window in which native events are enabled but no cursor can
observe them. Duplicate Page lanes and stale-session lanes are errors rather
than extra CDP attachments. No reader task is installed: the application still
polls the registered lane as part of its composed server loop.

Subscription activation is one ordered transaction. On the first Page interest
the activator enables the exact route's Network domain, then commits the bit.
Additional event tags only update the bounded mask. On disable, the bit remains
committed until any required native transition succeeds. A missing route during
cleanup means the target session has already retired and local state can be
forgotten safely. Cancellation at a CDP await leaves the prior logical state in
place so the runtime-control compensation path can retry the exact edge.

Native domain ownership is deliberately more conservative than logical event
ownership. When network events are the only installed Network-domain user, the
last interest sends `Network.disable`. When navigation is installed, the first
implementation treats it as a connection-lifetime shared owner because the
current `goto` backend enables Network while waiting for lifecycle events but
does not yet expose an operation-scoped release lease. In that composition the
last listener clears publication interest but does not disable Network. A later
navigation lease can narrow this lifetime without changing the public event
contract.

The ordered remote publisher consults the committed Page/tag mask immediately
before queue admission. Local statically wired typed queues remain independent;
dynamic remote filtering cannot turn off an in-process consumer. Request and
Response lifecycle creation still follows the Page lane as a whole, because a
terminal-only listener needs the same bounded correlation as a request listener.
No completed request history is retained.

Dropping a lane is terminal for that lane, not an implicit async cleanup path.
The owner must stop the composed server and run normal control cleanup; `Drop`
never starts a task or performs unawaited CDP I/O. Page disposal and session
replacement remove the exact registration and cause a future enable to require
a newly prepared lane.

### BrowserContext fan-out design checkpoint

BrowserContext listeners cannot be implemented by copying one bit into today's
Page entries. Pages may appear while the listener is live, and a multi-Page CDP
activation may be interrupted after only some sessions accepted
`Network.enable`.

The implementation now promotes individual lanes into one optional,
caller-polled **network event driver** per attachment:

- it owns one selected CDP event cursor for the connection, not one cursor or reader
  task per Page;
- it owns a bounded Page -> exact session -> normalizer table, capped by the
  profile's Page-lane limit;
- it routes each native event by session to exactly one normalizer;
- it exposes a non-owning registration handle that can join Page publication
  without taking ownership of, or blocking, the event cursor.

The single cursor, exact-session table, capacity checks, explicit Page
registration/retirement, active-request draining, lag failure, and attachment
registry/ledger identity checks are implemented. Explicit
`BrowserContext.newPage` and new-Page automatic discovery now couple the
registration handle to publication. Discovery replaces an existing Page
session through a separately reserved shadow adapter. It marks the logical lane
busy, enables the new session when the preserved direct-or-Context interest
requires it, stages the exact route only for topology/runtime publication, and
then swaps adapters. Old active Request roots join the same runtime lifecycle
batch. Publication failure restores the old route and lane, disables and drops
the shadow, and detaches only the incoming session. Cancelled activation remains
discoverable through the same explicit recovery operation.

This lowers cursor overhead and gives current and future Pages one ownership
point. It does not merge topology ingestion with network ingestion: both remain
independent caller-polled plugins over separate cursors, so neither consumer can
steal the other's events.

The registry now separately retains bounded Page masks and BrowserContext
masks, including empty materialized Contexts. The production profile exposes
separate Page-lane and Context limits; its compatibility constructor defaults
the Context limit to the Page-lane limit. A Page's native-domain edge already
uses the union of its direct mask and owning Context mask, so direct listener
cleanup cannot disable a Context-owned domain.
For a Page event, effective native interest is the union of its direct mask and
its owning Context mask. Remote publication uses the original subscription
scope: a Page listener receives a frame sourced from the Page, a Context
listener receives a frame sourced from the BrowserContext, and when both are
selected the ordered stream emits two frames that refer to the same portable
Request/Response identities. The deterministic order is Page scope first,
Context scope second. Local typed queues remain one independently selected
in-process observation path. This dual-scope selection and physical wire order
are implemented and covered by an executable test.

A Context subscription activation is a bounded resumable batch transaction.
It snapshots only the Context's currently materialized Page entries, requires
an exact prepared adapter for every one, and retains one exact Page/session/
action plan before the first native await. The logical Context bit commits only
after every required session is enabled. A definite failure compensates every
attempted session in reverse order. Because enable/disable are idempotent, a
cancelled future leaves the bounded plan pending; the runtime-control cleanup
edge replays the inverse plan across every possibly enabled session. Disable
follows the inverse rule and retains the old bit until every required
transition succeeds. Other Page transitions for those lanes are rejected while
the plan is pending, and Page-route/topology synchronization fails closed until
the pending plan commits or its cleanup completes.

A topology update that introduces or replaces a Page must prepare its network
adapter before enabling Network. The explicit `BrowserContext.newPage` path now
does this: it registers a provisional exact-session lane, enables inherited
Context interest, and holds any event from that provisional session until the
Page/Frame runtime, topology, and route publication commits under the same
registration guard. A later publication failure disables Network, discards the
lane, and then closes the newly created target. A cancelled provisional
activation remains discoverable through the non-owning handle and has an exact
idempotent disable/retirement recovery operation. Automatic new-Page discovery
now uses the same transaction and detaches its session after rollback. Session
replacement preserves the Page's direct and inherited masks, overlaps one
pre-budgeted shadow adapter, enables before route publication, and disposes old
active Request roots with the topology rebind. Page retirement drains active native correlation, produces
child-first portable cleanup, removes the exact adapter/masks, and treats the
retired CDP domain as already gone. The topology event owner now holds the
network driver state guard across Page-session detach publication: a failed
topology/runtime transaction preserves the lane, while a successful parent
dispose removes the runtime subtree before the adapter and native correlation
are released.

Context masks use the network profile's independently configurable Context
limit; adapter entries are bounded by its Page-lane limit. One in-flight
Context transition retains at most that many Page IDs, compact session IDs,
and action flags, all included in the pre-reserved registry ceiling. Empty
Contexts retain only their fixed four-bit mask. Multi-session first/last edges,
definite failure compensation, and cancelled-enable cleanup now have executable
proofs. Explicit Page creation and automatic new-Page discovery now have the
positive participant plus publication-failure cleanup proofs; explicit creation
also has cancellation recovery proof. Session replacement has success with
active-request cleanup, failed-publication restoration, and cancelled-activation
recovery proofs. Parent retirement has success and transaction-failure
preservation proofs. One normalized batch must be explicitly acknowledged after
publication; until then replacement, retirement, and the next driver turn fail
closed. `receive_and_publish_once` owns normalization, runtime/control
publication, and exact-Page acknowledgment in one async turn.

The normalizer consumes only session-matching events:

- `Network.requestWillBeSent` creates a Request and handles redirect closure;
- `Network.responseReceived` creates the Response for that Request;
- `Network.loadingFinished` emits `requestfinished` and removes native state;
- `Network.loadingFailed` emits `requestfailed` and removes native state.

CDP's `requestId` reuse across redirects is an adapter detail. Each redirect
hop receives a fresh portable Request identity, and the old hop reaches its
response/finished terminal state before the new `request` event is published.
Extra-info events, service-worker attribution, cache provenance, security
details, and timing fields may extend the capability compatibly only after
their ordering and bounds are specified.

## First implementation slice

The first vertical slice is intentionally narrower than full public parity:

1. add Request/Response kinds to the portable object catalog;
2. define dependency-light bounded network metadata models and the four event
   contracts;
3. **Implemented:** a pure CDP lifecycle normalizer with explicit limits,
   stable object-ID injection, redirect-hop identity, terminal cleanup, and a
   pre-reserved production `BackendStateBytes` ceiling;
4. **Implemented locally:** connect one caller-polled attachment cursor and
   exact-session Page adapters to four independently optional bounded typed
   queues. The composition atomically
   admits each lifecycle batch to the runtime, physically writes through its
   final Request/Response `Create`, and only then admits semantic events;
5. **Implemented for this slice:** fake AAA coverage proves request -> response ->
   finished, HTTP 500, transport failure, redirect, metadata/active exhaustion,
   wrong-session filtering, active teardown, queue exhaustion, Create fencing,
   Page-parent runtime teardown, and one cross-event wire sequence. Dedicated
   Chrome 152 release proofs cover success, redirect-hop ordering, HTTP 500,
   truncated-response failure, owned-context cleanup, and external-browser
   survival without body reads.

The attach production profile reserves a configurable number of concurrent
Page lanes plus one replacement-only shadow adapter, one attachment cursor,
their subscription registry, one maximum provisional Page transaction, and runtime capacity for two portable
objects per possible active request. It registers an exact adapter behind the
single cursor before an ordered first-listener edge enables `Network` on the
authoritative topology route. `pwright-codec-network-events` encodes all four
public tags through one bounded canonical binary payload family, and
`ChromiumRemoteNetworkEventProfile` converts without drift into
`ChromiumNetworkEventDeliveryProfile`. The independent delivery installation
creates one negotiated queue and one monotonic sequence. Generic per-event
remote outbounds remain rejected because
independently scheduled queues could expose a Response before its Request.

The composition leaf owns codec, production queue, resource, transport write,
creation-fence, and retained receive/publish/ack boundaries. Its setup rejects
zero capacity, insufficient terminal payload/total-frame limits, and
insufficient decoded metadata retention. Standalone dependency proof admits
exactly 16 Pwright packages: the normalized CDP batch contract and
runtime/control fence are intentional, while the CDP receiver, aggregate
composition, providers, launchers, installers, Tokio executor, Node.js, and
browser acquisition remain excluded. The complete recipe obtains its sender,
outbound, lifecycle publisher, and retained backend from this leaf. Its
aggregate driver and dynamic Page/BrowserContext registry implement the two
injected ports without transferring their state or mutation ownership.

`pwright-client-network-events` provides one routed handler with reliable
`RejectNewest` overflow plus Page and BrowserContext helpers that turn the first
and last local listener into `SetSubscription` edges for the exact portable
owner. The base client claims announced
Request children from Page and Response children from Request. The application
still owns sending those returned control messages; a callback-owning facade is
withheld. The server-side owned network turn now joins the driver, lifecycle
publisher, ordered control lane, and exact acknowledgment. A selected driver
must be taken by the application installer. Taking it also attaches its
non-owning registration handle to topology independent of take order. The
all-capabilities recipe owns the negotiated publisher/outbound and a retained
supervisor backend turn; terminal notification drops the cursor and clears the
exact Page registrations. Unit teardown and dedicated-Chrome
install/drop/external-browser-survival proofs pass. Explicit Page and
BrowserContext close now retire driver adapters only after
topology/routes/runtime publication succeeds; failed native Page close preserves
the lane. The driver retains one budgeted parent Context ID per Page adapter,
allowing Context retirement to filter in place without a second Page-ID list or
a registry lock held across route publication.

The ignored release workload `live_network_memory_workload` performs a complete
warmup Context/Page/network/close lifecycle, proves the single Page-lane
capacity can be reused, then performs three real Chrome 152 data-URL network
lifecycles. It observes the exact navigation Request and Response, publishes
portable lifecycle behind the ordered Create fence, acknowledges every batch,
and samples only when active native correlation and the control outbox are both
zero. The first run exposed the old `6 -> 8 -> 10` live-object growth. After
terminal snapshots and post-admission Request-root retirement were implemented,
the same workload holds `LiveObject.current` at `[4, 4, 4]`: only Browser,
Context, Page, and Frame remain. The correcting run recorded 7 observations,
runtime RSS `8,388,608 -> 10,059,776`, browser RSS sum `1,609,121,792 ->
1,377,501,184`, and browser processes `10 -> 8`. These values prove the
workload and cleanup boundary, not a stable budget. A follow-up bounded series
now runs three independent attachments, each with three steady network
lifecycles. The composition derives two subscriber slots: one for the shared
network driver and one for the single admitted navigation. The production
fanout reclaims an event immediately after both current cursors advance. A
response barrier followed by nonblocking drain requires its queue to be empty
after each navigation and after Page/Context teardown.

Warmup process settlement accepts a new stable process count rather than
forcing first-use helpers back to the cold count. Measured teardown remains
strict: it must return to that cleaned warm count before a report enters the
series.

Two consecutive strict series held all six runs at `[4, 4, 4]`, returned every
logical resource class to the warm baseline, and settled Chrome's process
count. No `EventBytes` allowance is used. The process-stable second series
observed runtime RSS deltas of `+98,304..+1,425,408` bytes and browser RSS-sum
deltas of `+884,736..+5,357,568` bytes. These remain diagnostics, not reviewed
budgets. The legacy v1 event API still uses Tokio broadcast for compatibility;
the reclaiming delivery strategy is explicit production-profile composition.
A post-race-fix repeat again passed with 8 -> 8 browser processes,
`+65,536..+1,556,480` runtime bytes, `+933,888..+4,980,736` browser RSS-sum
bytes, and an empty fanout at every asserted boundary.
The warmup-settlement correction was then exercised by another three-run
series at 6 -> 6 processes: runtime deltas were
`+81,920..+1,490,944` bytes and browser RSS-sum deltas were
`+212,992..+4,849,664` bytes, with `[4, 4, 4]` steady objects throughout.
The full-recipe Chrome 152 proof separately overlaps two slow navigations with
two Locator clicks while PageLoad and the shared network driver remain active.
Its derived bound is 10 because an alternative composition may install four
independent Page-network lanes; the canonical recipe shares one lane and
observed a 6-or-7 cursor peak, then returned to its three steady cursors with
zero queued events and zero lag.
Production tests drive 4,096 completed native lifecycles and 4,096 client wire
deliveries: completed native state returns to zero, the one-slot client ledger
peaks at one lease, and overflow never evicts the causal predecessor.
Optional post-terminal client object leases, response-body commands, routing,
WebSocket, and HAR remain separate follow-up slices. The default immediate
retirement is intentional and this is not full Playwright network parity.

## Extracted lifecycle composition boundary

The ordered delivery queue, lifecycle creation fence, and retained
receive/publish/acknowledge turn now live in
`pwright-composition-chromium-network-events`. The leaf accepts two ports:
`ChromiumNetworkEventBatchDriver` yields one already-normalized batch and
acknowledges its exact Page, while `ChromiumNetworkEventScopeSelector` maps the
batch's Page and event tag to direct Page and optional BrowserContext routes.
This preserves the aggregate registry's committed listener semantics without
giving the leaf ownership of subscription mutation, Page routes, topology, or
a CDP event receiver.

One backend poll retains the driver and future until the entire causal turn
finishes. It applies lifecycle Create records, sends ordered control through the
last Create, admits Page before Context into the shared `RejectNewest` queue,
applies terminal retirement, and only then acknowledges the batch. Any
publication failure leaves acknowledgement closed; terminal connection
notification drops the in-flight future and driver. The complete recipe uses
this backend. The pre-existing aggregate publisher/backend remain available as
compatibility APIs while callers migrate.

## Rejected designs

- **Reuse v1 `network-listen --include-body`:** it has an unbounded pending map
  and no body-size ceiling, so it recreates the memory failure under study.
- **One `network` super-capability:** installing metadata observation would
  transitively install interception, bodies, WebSocket, and recording.
- **Expose CDP request IDs publicly:** ties the API to Chromium session and
  redirect behavior and cannot support Firefox/WebKit adapters.
- **Lossy ring buffer for lifecycle:** a missing response or terminal event
  leaves object state permanently inconsistent.
- **Retain every request until Context close:** simple, but makes long-lived
  contexts accumulate unbounded dispatcher/object history.
- **Fetch bodies automatically on `response`:** response fires before body
  completion and body transfer is the expensive optional authority boundary.
