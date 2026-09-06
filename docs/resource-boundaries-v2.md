<!-- agent-updated: 2026-09-05T13:26:15Z -->

# Pwright v2 resource ownership and limits

This document is the DT-060 allocation inventory, limit contract, eviction
policy, and internal memory-metrics schema. It covers memory retained by the
Pwright client/runtime process. Browser parent, renderer, GPU, and utility
process memory is measured separately; a Pwright allocation ledger cannot and
must not pretend to cap Chromium's own RSS.

## Rules

1. Every retained collection has a count limit. Every retained variable-size
   payload has both a local structural limit and a connection-wide byte budget.
2. A connection owns one explicitly constructed `ResourceLedger`. There is no
   process-global ledger and no hidden default budget.
3. `ResourceLimits::deny_all` begins with zero allowance for every class. The
   composition root enables only the classes installed capabilities require.
4. A reservation checks its global and per-scope limits atomically before the
   producer retains data. Multi-class reservations either all succeed or make
   no counter change.
5. A non-cloneable `ResourceLease` stays beside the retained state. Explicit
   release, normal return, cancellation, panic unwinding, and future drop all
   return its units through RAII.
6. Correctness state is never silently evicted. A component must apply the
   class policy below: backpressure, a declared lossy queue policy, spill, or a
   typed `ResourceExhausted` failure.
7. Browser acquisition and process ownership are unrelated to memory
   accounting. Creating a ledger never launches, downloads, bundles, closes,
   or kills a browser.

## Units and ownership scopes

Count classes use one unit per item. Byte classes use one unit per byte of
retained application data. Rust allocator and map-node overhead is not guessed;
the separate hard limits on objects, scopes, leases, queues, calls, and topology
records bound that metadata.

One ledger is connection-local and aggregates all participating components.
Each reservation is additionally attributed to one scope:

| Scope | Use |
| --- | --- |
| `Connection` | Handshake/control state and data with no narrower owner |
| `Object(ObjectId)` | Page, Frame, BrowserContext, request, response, or handle state |
| `Call(CallId)` | Command parameters/results and transient operation state |
| `Extension { namespace, id }` | Adapter/plugin state whose identity is intentionally not portable |

An extension namespace is assigned by the composition root. It is not a backend
protocol ID and never appears in the public Playwright-shaped API.

## Resource classes

The core ledger defines these stable accounting classes:

| Class | Unit | Intended owner | Release boundary |
| --- | ---: | --- | --- |
| `LiveObject` | item | runtime connection/object graph | child-first Dispose or transport close |
| `PendingCall` | item | call correlator/runtime | result, cancellation, deadline, send failure, or close |
| `ActiveCommand` | item | connection scheduler active permit | operation completion, cancelled admission, or future drop |
| `EventItem` | item | subscribed object/event route | dequeue, declared eviction, unsubscribe, or close |
| `EventBytes` | byte | queued typed event | with its event item |
| `WireBytes` | byte | encoded/incoming frame | final frame clone/decoder/transport release |
| `ResponseBodyBytes` | byte | response/call retaining a body | body consumer/dispose/stream cancellation |
| `LogBytes` | byte | opt-in log capability | dequeue, ring eviction, or capability stop |
| `TraceBytes` | byte | opt-in trace recorder | bounded chunk flush/spill or trace stop |
| `ArtifactBytes` | byte | optional Artifact source and Stream read result | explicit Stream/Artifact disposal or returned read-result drop |
| `ScreenshotBytes` | byte | screenshot call/artifact; retained value type is owned by command-free `pwright-screenshot-model` | result consumption or artifact dispose |
| `DomContentBytes` | byte | Frame DOM serialization result | returned budgeted UTF-8 value drop |
| `SelectorBytes` | byte | operation carrying a `pwright-selector-model` recipe or backend cache | operation end or bounded cache eviction |
| `RemoteHandle` | item | Page/Frame execution world | explicit handle dispose, world destruction, or page close |
| `BackendStateBytes` | byte | adapter extension scope | target/session detach or backend close |

`ResourceLimits` supplies a global and per-scope hard limit for every enabled
class, plus independent maximum counts for live scope records and active
leases. Zero is a valid limit and rejects the first reservation.

## Allocation inventory

| Retained state | Owner | Existing local bound | Shared-ledger class | Overflow/eviction policy | Implementation status |
| --- | --- | --- | --- | --- | --- |
| Runtime object graph | runtime connection | `RuntimeLimits.live_objects` | `LiveObject` in object scope | reject Create before graph/outbox mutation; parent Dispose is child-first | shared count implemented when injected |
| Client live-handle tokens and unclaimed announcements | client connection | `ClientLimits.live_objects` shared by announced, claimed, and disposed-before-claim entries | `LiveObject` | reject initializer; remote Result must claim exact prior Create; Dispose invalidates clones; late Result consumes its tombstone and fails | local bound implemented |
| Pending runtime calls | runtime dispatcher | `RuntimeLimits.pending_calls` | `PendingCall` in call scope | reject before scheduler/backend; completion/future drop releases; cancellation alone retains | shared count implemented when injected |
| Pending encoded client calls | client-wire channel | pending-call/tombstone limits | `PendingCall` in call scope | reject before state/send mutation; result, cancellation terminal result, send failure, or close releases | shared count implemented when injected |
| Target lanes and queued commands | target and connection schedulers | lane/waiter limits | already retained by call-scoped `PendingCall`; no `ActiveCommand` while waiting | bounded wait, cancellation, or typed exhaustion | local bounds plus shared pending-call count implemented |
| Active backend commands | connection scheduler | `RuntimeLimits.active_commands` | `ActiveCommand` in call scope | reserve before grant; atomically transfer the lease on FIFO promotion; operation/cancellation/future drop releases | shared count implemented when injected |
| Typed event queues | client event extension | capacity plus declared maximum decoded bytes per event | `EventItem` + `EventBytes` in source-object scope | RejectNewest does not reserve; DropOldest atomically replaces the old lease; local oversize rejects before ledger mutation; dequeue/clear/drop releases | shared budget and production profile implemented |
| Lifecycle/control outbox | runtime control | explicit item capacity | `EventItem` in object/call scope | reserve the whole lifecycle batch before graph mutation; claim rollback retains, while pop/commit/close releases | shared count implemented when injected |
| Abandoned-operation continuation | injected cleanup executor | fixed queue or semaphore capacity; optional caller-selected nonzero Tokio job and drain deadlines; finite explicit retry attempt count | one non-cloneable reservation or active permit per admitted logical job; retry keeps the same permit | reserve before irreversible backend work; shutdown closes admission then drains accepted reservations/jobs; job deadline drops the opaque job so its own cancellation-safe guard escalates; drain timeout observes but does not cancel; only a caller factory can create retry attempts | standard-library caller-driven queue plus opt-in Tokio reserved execution, application drain, explicit retry factories, reporting events, and detailed metrics implemented |
| Owned-browser shutdown diagnostics | optional launch-composition report sink | one report per successfully terminated owned Browser; no portable result payload | provider-owned report moves synchronously after runtime disposal; default sink is zero-state discard | no queue, task, clone, or wire encoding in the composition; a sink needing I/O must perform its own bounded nonblocking admission | generic sink and discard default implemented; encoded unit-result and exact-PID/profile live proof pass |
| Prepared object disposal | controlled runtime | at most one active transaction; exact existing subtree and lifecycle-outbox capacity | reserved `EventItem` leases in object scope; no `WireBytes` | reject before backend work; Drop releases every reservation; commit transfers leases child-first; transport close invalidates the token | implemented and coordinated with Chromium topology close |
| Prepared object creation | controlled runtime | at most one active transaction; one object or a finite caller-supplied related-object batch with validated graph insertions and exact lifecycle-outbox slots | reserved `LiveObject` + optional `EventItem` in each new object scope | validate the full candidate before reserve; keep every object absent before commit; Drop releases; commit publishes the graph and ordered Creates together; transport close invalidates the token | implemented for single Context and Page+main-Frame publication |
| Graceful-close machines | unique client driver / server control decorator | one fixed-size phase value and counters per endpoint | `Connection`; no dynamic units | no queue or allocation; cancelled Ack wait retains only phase; preceding frames are validated and dropped one at a time | implemented |
| Tailnet endpoint authentication | caller-owned endpoint plus one caller-polled accept/connect future | one zeroized 32-byte PSK; 32-byte nonce; two 32-byte proof buffers; exactly 105 transcript bytes | fixed stack/endpoint state, no variable shared-ledger units | reject non-Tailscale addresses before I/O; reject invalid proof before frame transport; caller bounds concurrent futures and injects deadline | implemented |
| Mutual-TLS endpoint authentication | caller-owned endpoint; semaphore-bounded accepted handshakes; one caller-polled client connect | rustls cap: 65,535-byte handshake message and 65,536-byte certificate; explicit outbound TLS buffer limit `max(65,536, stream envelope + 8)`; compile-time pin count | third-party TLS state is documented separately; authenticated Pwright frames enter the existing `WireBytes` ledger | reject wildcard/config/ALPN/certificate/pin failures before frame transport; handshake permit releases on success/error/cancellation; caller injects deadline and PKI policy | implemented |
| Caller-driven Chromium endpoint server | deployment composition | one caller-selected pre-bound listener, stateless attach factory, and external CDP endpoint; exactly one accept future per call | endpoint-specific pre-authentication resources; each successful peer gets a fresh connection ledger | endpoint rejection precedes CDP; no loop, task, retry, admission policy, or deadline; runtime/application installers remain explicit per connection | Unix UID plus dedicated-Chrome live proof; Tailnet and mTLS static adapters implemented; operational admission/deadline policy remains caller-owned |
| Complete current-capability Chromium recipe | one accepted Chromium connection | one controlled Page runtime; one static command chain; one activated control receiver; lifecycle plus three typed-event outbound lanes; topology and PageLoad backend cursors; caller-sized cleanup queue; four supervisor/runtime command slots in the full profile | existing per-capability ledger claims; topology driver owns one fixed backend-state lease; PageLoad adds one selected event cursor and one retained future; each typed event owns its declared queue leases | reject any unselected or unnegotiated capability/control/event component; derive reclaiming subscriber capacity from selected plugins; keep supervisor ingress and runtime active-command limits equal; terminate on cursor lag, mapping failure, or event overflow; Page-close poll and cleanup draining remain injected | all eight current browser capabilities plus control and three Page event lanes compile as one concrete application; unit cursor proof, dedicated-Chrome assembly/reattach proof, and four-command navigation/Locator overlap proof pass |
| Typed result/command to encoded wire frame | protocol connection | per-payload, attachment, attachment-count, aggregate frame, and source representability limits | `WireBytes` | reserve source + maximum output before codec allocation; atomically shrink/move to exact route-owned frame; failure releases | shared budget and production protocol claim implemented |
| Outgoing stream envelope and pending write | Tokio stream transport | exact encoded and length-prefixed sizes | `WireBytes` in connection scope | reserve before each allocation; pending buffer owns lease across cancellation; production rejects a frame from another/no ledger | shared budget and production transport claim implemented |
| Incoming stream envelope | Tokio stream transport | outer declared length checked before allocation | `WireBytes` in connection scope | reject before body read; cancellation/parse failure releases through RAII | shared budget and production transport claim implemented |
| Decoded incoming frame | stream codec/transport | payload, attachment-count, attachment, and aggregate limits | `WireBytes` in route-derived scope | reserve before copying; no truncation; final frame clone owns lease | shared budget implemented when injected |
| Generic fragment assembly | transport utility | declared length checked before exact reserve | `WireBytes` in caller-selected scope | reserve full declaration before buffer allocation; reject declaration/overrun/incomplete stream; finished bytes retain lease | shared budget implemented when injected |
| Raw Chromium CDP connection | CDP WebSocket connection | pending/write/event counts plus command/response/event/error/WebSocket byte limits; event capacity plus one incoming allocation-before-admission item; optional fixed subscriber-slot bookkeeping | `WireBytes` + `EventItem` + `EventBytes` + optional `BackendStateBytes` in connection-selected scope | synchronous command admission; bounded serialization; pending/queue clones share leases; legacy v1 uses Tokio broadcast; opt-in v2 fanout reclaims after every current cursor advances, drops events with no cursor, and reports bounded overwrite as lag | shared budget, legacy compatibility path, task-free reclaiming path, derived composition preflight, and production connector implemented |
| Native CDP liveness | one attached Chromium composition | one cancellation-token clone and one boxed caller-polled future; no task, queue, Browser owner, or replacement connection | fixed backend state only; the concrete connection retains its existing resource leases | reader/writer termination makes the signal sticky; liveness backend returns terminal failure; supervisor cleanup drops sibling backend turns and shuts down the remote transport; old native sessions are never rebound | implemented independently of topology-events; fresh remote-client snapshot/event-view bootstrap pending |
| Reusable Chromium attach recipe | process/application configuration | immutable capability profile, `ResourceLimits`, and Browser root ID only; no retained ledger, connection, topology, ObjectId source, event receiver, runtime, or browser lease | none while stored; every `prepare` allocates a new connection-local `ProductionResourceLedger` | validate profile/limits/root namespace before I/O; preparation or attachment failure drops only fresh state; repeated calls never share ledgers or rebind Browser graphs | `ChromiumAttachFactory` and ledger-identity tests implemented; complete remote session unit pending |
| Fresh Chromium remote connection unit | one accepted remote connection | exactly one fresh CDP attachment, production remote transport/supervisor, runtime graph, and optional topology-event driver; no hidden task/listener/executor | every component must prove the fresh connection ledger; transport requirements must exactly equal the handshake profile | reject missing handshake before CDP; reject builder error, foreign ledger, plan drift, duplicate topology driver, installer that returns without installing runtime, unnegotiated recipe capability, missing discovery/control component, or an unconsumed topology driver; topology updates fence through their final lifecycle `Create` even without portable event queues; failed staging drops the new unit rather than exposing a partial session | consuming runtime/handshake/application stages, mandatory liveness wrapping, exact shutdown, pre-provisioned navigation execution, client-observed automatic discovery, packaged Unix-listener admission, and complete current-capability assembly have dedicated-Chrome proof |
| CDP context/target/session/frame/world maps | Chromium backend | explicit object/context/page/frame/world/native-ID limits; at most one owned async-spanning transaction | `BackendStateBytes` in one adapter extension scope | production mutation pre-reserves a maximum shadow; ordinary candidates commit exact bytes; BrowserContext close keeps the conservative shadow lease until the next mutation so post-CDP commit cannot fail; drop rolls back | shared budget, production profile, and owned close transaction implemented |
| Explicit Chromium BrowserContext close | composed lifecycle service + ordered runtime control | one topology transaction, one runtime transaction, and at most supervisor in-flight commands plus one outbound writer slot | topology `BackendStateBytes` + runtime child-first `EventItem`; command futures are already bounded by supervisor `PendingCall`; raw transports retain existing `WireBytes` | reject non-Owned before CDP; backend failure drops both preparations; success commits topology/runtime, sends through target Dispose, then sends Result; barriers precede ordinary queued writers | local, remote-stream, client-application, and abandonment ordering implemented |
| Explicit Chromium BrowserContext creation | composed creation service + ordered runtime control | one monotonic ID, one topology candidate, one runtime creation transaction, and one synchronously reserved continuation | topology `BackendStateBytes` + runtime `LiveObject`/`EventItem`; existing CDP/wire command budgets cover native and remote traffic | reject invalid target/metadata/capacity before CDP; burn failed ID; compensate post-CDP publication failure by disposing the exact native context; remote sends exact Create before Result | local, remote-stream, client-application, abandonment, compensation, and dedicated-Chrome proofs implemented |
| Explicit Chromium Page creation | composed creation service + ordered runtime control, optionally joined to the single-cursor network driver | two monotonic IDs, one topology candidate, one Page-route candidate, one two-object runtime creation batch, at most one provisional Page/session registration, bounded exact-target cleanup checks, and one synchronously reserved continuation | topology/routes/network-registry `BackendStateBytes` + two runtime `LiveObject`/`EventItem` leases; existing CDP/wire command budgets cover native and remote traffic | reject invalid Context/metadata/capacity before CDP; create `about:blank`, flatten-attach, require a bounded root frame; under inherited Context interest register and enable the exact session before publication, hold a racing provisional event, then commit the lane with runtime/topology/routes; on failure disable/retire before closing and confirming target absence; remote sends Page Create, Frame Create, then Result | local, remote-stream, client atomic-claim, abandonment, ordinary compensation, provisional-network success/publication-failure/cancellation recovery, and dedicated-Chrome proofs implemented |
| Explicit Chromium Page close | composed lifecycle service + ordered runtime control | one topology transaction, one runtime transaction, one exact Page-route replacement, bounded target-absence checks, and the existing ordered-writer limit | topology/routes `BackendStateBytes` + runtime child-first `EventItem`; each `Target.getTargets` response is bounded by the CDP response-byte limit and is not retained after a check | reject before CDP on invalid/lifecycle capacity; close exact target; commit only after exact absence; publish topology/routes together, then runtime Frame→Page Dispose, then Result | local, remote-stream, client-application, abandonment, and dedicated-Chrome proofs implemented |
| Chromium Page before-unload initiation | exact Page-session route | one command and existing CDP pending/write response bounds | existing route `BackendStateBytes`, CDP `WireBytes`, and supervisor/runtime `PendingCall`/`ActiveCommand` | send page-session `Page.close`; return without topology, route, or runtime disposal; later dialog/target lifecycle owns resolution | implemented with fake-CDP and owner-liveness proof |
| Abandoned lifecycle continuation | caller-owned reserved cleanup executor | one synchronously reserved slot per admitted create/close; queued jobs plus reservations never exceed executor capacity | existing future retains its already-accounted topology/runtime/wire owners; queue adds fixed bookkeeping only | reject before CDP if reservation fails; unpolled Drop releases; started pending Drop transfers the exact future without a second capacity check; job completion/failure is metered | implemented for BrowserContext creation/close, Page creation, and ordinary Page close |
| Attached Chromium topology/runtime owner | attached composition | one shared topology and at most one installed production runtime | topology/runtime use their already-declared exact ledger claims; the `Arc`/mutex are fixed attachment state | topology mutation requires a synchronous guard; runtime factory rejects a second owner; Page-runtime close factories reuse exact CDP/topology/route identities and require the continuation owner | implemented and covered by dedicated-Chrome Context/Page proofs |
| Portable Page to CDP session routes | Chromium composition | topology page/native-ID limits; four target/session slots plus two overlapping utility-context slots per Page | `BackendStateBytes` in a distinct composition extension scope | fixed six-slot maximum lease before retention; unchanged routes reuse services; utility-world replacement rebuilds only the affected locator route; detach/reconnect drops retired entries | shared budget and production profile implemented |
| Chromium utility-world bootstrap | optional topology-event infrastructure profile | one fixed 34-byte per-Page name, one browser-retained empty new-document registration per live target session, and sequential commands over at most the bounded frame tree | no separate Rust retention class; command/response bytes use the existing CDP `WireBytes` plan, world unique IDs use topology and Page-route `BackendStateBytes` | no task or queue; only a creation failure for a frame absent from a fresh authoritative tree is tolerated; target/session teardown owns Chromium-side registration disposal | implemented for Page and OOPIF sessions with fake-race and dedicated-Chrome isolation proof |
| Locator query/runtime source | locator capability/Chromium backend | depth, node, aggregate string, value-capacity, generated-script, and concurrent-action limits | `SelectorBytes` in composition-selected scope | production call reserves its complete transient peak synchronously before returning a future; completion/cancellation/unpolled future drop releases | shared budget and production profile implemented |
| Active Chromium `Page.goto` tracking | navigation backend action | URL/native-ID/inflight-request/header/status limits plus concurrent-operation bound | `BackendStateBytes` in composition-selected scope | production call reserves complete peak synchronously; reject before request insertion/header copy; completion/cancellation/drop releases | shared budget and production profile implemented |
| HTTP Request/Response lifecycle metadata | optional `pwright.network-events` model/contract, Chromium normalizer, single-cursor attachment driver, subscription registry, and one cross-event remote stream | active request count; Page-lane and BrowserContext counts; one fixed parent Context ID per Page driver lane; separate four-bit Page/Context masks; at most one in-flight Context plan containing bounded Page IDs, compact session IDs, and actions; at most one provisional Page transaction containing bounded session/target identity and fixed client/bookkeeping ownership; one unpublished-batch Page ID; two temporary runtime children per active request; compact in-flight Request/optional-Response metadata; bounded terminal snapshots and ordered queue items/bytes | pre-reserved `BackendStateBytes` for native correlation plus the maximum compact Request/Response snapshot of every active request, exact-session Page state and its parent Context ID, one replacement-only shadow adapter beyond the public Page-lane count, empty-Context masks, the maximum Context transition plan, and one maximum provisional Page transaction; `EventItem`/`EventBytes` count complete terminal snapshot capacity for selected queues; `LiveObject` and lifecycle `EventItem` cover only in-flight Request/Response children | register one connection cursor and exact Page adapters before enable; use direct/inherited union for native ownership; Page edges commit after exact-session CDP success; Context edges snapshot before I/O and commit only after the whole batch, reverse-compensate definite failure, and preserve cancelled enable as idempotent disable cleanup; explicit `newPage` and automatic new-Page discovery hold provisional-session events until atomic publication and reverse enable before target/session cleanup on failure; replace a discovered session through the pre-enabled shadow while preserving masks and disposing old active Request roots in the runtime rebind; Page/session detach and explicit Page/Context close hold network state across topology/routes/runtime publication, preserving adapters on failure and retiring them only after parent-subtree commit; Context close filters adapters by their retained parent identity without another Page-ID allocation or a subscription lock across publication; require exact Page acknowledgment after every normalized batch before another receive, replacement, or retirement; the owned driver turn combines receive, lifecycle/control/event publication, and acknowledgment; fan out Page then Context with shared identities; lifecycle Create is fenced before first exposure; terminal Request/optional-Response values are self-contained and one Request-root Dispose commits only after every selected queue accepts the batch; queue loss or retirement failure is connection-terminal; application install requires unique-driver consumption and the retained backend drops it on terminal connection notification; no completed history or body read | portable model, object kinds, complete terminal snapshot codec/client contract, production normalizer, bounded single-cursor driver/registry, transactional Page activation, resumable multi-session Context activation, explicit-newPage and automatic-discovery inheritance/rollback, atomic session replacement/rollback/recovery, atomic parent retirement, unpublished-batch barrier, owned publish turn/backend, explicit Page/Context close adapter retirement, Page/BrowserContext client helpers, empty Context retention and dual-scope publication, ordered codec/client path, all-capabilities application wiring, 4,096-iteration bounds, overflow proof, unit terminal cleanup, dedicated-Chrome install/drop/external-browser-survival proofs, a persistent-Page Chrome workload with stable `[4, 4, 4]` live objects, and two consecutive three-run RSS series implemented; cross-host series and reviewed budgets pending |
| Response bodies | future independent body capability | per-call result or chunk bytes and concurrent consumers | `ResponseBodyBytes` | reserve before allocation; bounded result or stream/backpressure; never read or retain all traffic implicitly | capability not yet implemented |
| Console and browser logs | future opt-in log capability | must declare item and byte limits | `LogBytes` | event-specific ring/drop policy with loss metrics | capability not yet implemented |
| Trace metadata chunks | optional BrowserContext tracing capability plus injected sink | session/chunk name/title bytes; group depth; entries per chunk; bytes per entry; total in-memory chunk bytes; exactly one detached chunk awaiting sink; preallocated registered-Context count | `TraceBytes` plus independent entry and Context count bounds | reserve before copy; group location and correlated call-ID entries reserve exact encoded size before one allocation; `stopChunk` atomically transfers or discards; a detached chunk gates the next chunk/session and drops payload leases/vector storage before reopening the gate; Context teardown releases recorder first and then requires sink cleanup; sink-cleanup failure cannot retain base state; wire control accepts no attachments; no core I/O/task | core-only bounded recorder, exact production profile, six-command 1.0 schema/capability, bounded binary codec, opt-in client facade, generated server chain, task-free shared recorder service, injected clock/sink, full typed duplex proof, optional navigation contributor, dedicated-Chrome `goto` proof, first DT-003 tracing workload, and bounded atomic file sink implemented; rich contributors, remote streaming, repeated series, and reviewed budgets pending |
| PWRTRACE encoding and atomic trace files | shared `pwright-trace-format` plus optional `pwright-trace-sink-file` leaf | encoded Artifact byte ceiling; encoded directory-path bytes; distinct Contexts; committed chunks per Context; one encoded file; total committed bytes per Context; exactly one synchronous staging file under the state lock | Artifact path reserves exact `ArtifactBytes` before allocation and returns that owned buffer; file path retains recorder `TraceBytes` only until synchronous handoff and accounts disk separately | one shared exact size/writer contract; Artifact oversize rejects before lease/allocation and makes no staging copy; file sink requires a fresh directory, admits before path creation, then create-new temporary file, stream, sync, and atomic rename; failed publication cleans staging best-effort | two format AAA tests prove exact budget/bytes and oversize pre-admission; six file-sink tests preserve exact tracing-service bytes and Context/chunk/aggregate/directory gates; producer command, directory recovery, archive/ZIP, serving, and remote delivery remain separate |
| Pullable trace chunks | optional `pwright-trace-sink-pull` leaf | fallibly preallocated global FIFO slots; queued chunks per Context; retained name/title/payload bytes per chunk and globally | original chunk-owned `TraceBytes` leases plus independent queue item/byte admission; peaks and accept/reject totals are sink-local | reject before queue mutation; queued chunk keeps recorder gate closed; `pop` releases queue admission but transfers the non-cloneable chunk and all leases to caller; Context/all abort drops only still-queued chunks; caller must bound and clean any post-pop async send | six AAA tests cover tracing-service handoff, ownership after pop, item/byte/per-Context rejection, label accounting, selective abort, and connection-wide abort; concrete remote transport, cancellation/retry, and delivery acknowledgment remain separate |
| Artifact byte stream | optional command-free model, `pwright.artifact` 1.0 contract/codec/server, opt-in client facade, task-free registry/service, and separate runtime lifecycle adapter | fixed preallocated Artifact and Stream tables; one immutable source shared by bounded Streams; one cursor/prepared read per Stream; source, read, failure, retained-result, Artifact-count, and Stream-count limits; one successful-result attachment; controlled-runtime live-object/control capacity and ordered-writer waiters | `ArtifactBytes` in source-object and result-call scopes for sources, failure results, and read results; production profile covers maximum sources plus retained-result pool and worst-case one-byte result scopes; a prebudgeted source may transfer the same owned lease/buffer into the Artifact; wire attachment remains separately owned until exact decode copy succeeds; runtime owns its existing `LiveObject` and control-message leases | reserve/copy result before cursor mutation; commit advances, prepared-read drop rolls back; explicit close rejects during preparation; Artifact/parent/connection teardown aborts child Streams first; registry Create stays unpublished until runtime commit; disposal holds and compares stable child-first plans; exact Create/Dispose is fenced before command success; failed Stream Create publication is compensated locally; codec requires index zero, one attachment, exact declared length, and explicit EOF including empty artifacts | stable handles/object vocabulary, eight model tests, frozen generated five-command schema, six codec tests, three client tests, ten registry/service tests including owned-source scope/no-copy and complete typed duplex pull, opt-in/absent manifest proofs, and five runtime transaction/order/failure/trace-ownership tests implemented; tracing producer command pending |
| DOM/ARIA snapshots, screencast, network bodies, HAR, sources, video | independent future trace contributors/artifact capabilities | each plugin declares its own acquisition, retained bytes, staging count, redaction, and sink limits | contributor-specific bytes; never borrowed implicitly from base trace budget | independently selectable; shared chunk sequence only after explicit composition; failure attribution remains per contributor | deliberately not bundled with base tracing |
| OS process-memory attribution | optional `pwright-process-metrics` diagnostic; optional `ledger` feature depends only on `pwright-core` | process-table snapshot bytes, process count, inspected command bytes, explicit stable runtime/browser root PIDs, total/phase observation counts, exact bounded series run count, and one typed owned-process exit proof | sampler-owned transient bytes plus one pre-reserved bounded observation vector per run and one pre-reserved bounded report vector per series; results correlate fixed-size OS counters, `ResourceMetrics`, and browser object counts and are not a `ResourceLedger` limit | read at most limit plus one sentinel byte; kill/reap oversized `ps`; discard command text after role classification; reject malformed/duplicate/cyclic PID graphs, overflow, phase/time/root changes, incomplete runs, teardown deadline violations, series plan mismatch, unclean logical state, or measured teardown above the stable warm process count; owned teardown requires the same snapshot to contain the runtime and omit the exact Browser PID and never fabricates zero browser RSS; reusable variable byte payload requires an explicit per-class plan while owner/lease/scope baselines remain strict | default standard-library-only crate and CLI, opt-in caller-driven external/owned recorders and bounded series, AAA tests, dedicated-Chrome idle/detach, Page/Context, retained-Handle, first tracing run, one three-run Playwright-shaped `goto` series, two consecutive three-run network-lifecycle series, and one three-run real owned-browser teardown series implemented; other repeated/cross-host series, Linux PSS, and budget thresholds remain DT-003 work |
| Page screenshots | optional `pwright.content` 1.0 | decoded-byte ceiling, one active capture per Page, configured connection concurrency, one attachment | `ScreenshotBytes` for backend/client result; `WireBytes` for frame copy | reserve maximum before decode, reject encoded oversize, shrink to exact retained bytes; drop releases admission | production Chromium adapter and remote/client transfer implemented |
| Locator screenshots | optional `pwright.locator_screenshot` 1.0 | selector depth/node/string ceilings, bounded Frame topology and native-ID length, one active capture per Page, configured selector/capture concurrency, decoded-byte ceiling, one attachment | `SelectorBytes` during resolution; `ScreenshotBytes` for backend/client result; `BackendStateBytes` for independent Frame routes and shared owner-translation chain; `WireBytes` for frame copy | acquire capture before resolution; drop releases selector/capture admission; scroll then re-resolve geometry; translate each ancestor via `DOM.getFrameOwner` plus native border box; shared attachment helper validates metadata before exact retained copy | independent schema/capability/codec/client/server, Page/main/local-child/nested-OOPIF routing, negotiated remote transfer, dedicated-Chrome main and same-process child proofs implemented; cross-site OOPIF live and decoded pixel fidelity remain conformance work |
| Frame DOM content | optional `pwright.dom_content` 1.0 | UTF-8 byte ceiling, one active read per Frame, configured connection concurrency, one attachment, shared utility-route count/native-ID ceiling | `DomContentBytes` for backend/client result; `WireBytes` for frame copy; one `BackendStateBytes` route reservation shared with locator | reserve maximum before native read/copy, shrink to exact retained text; no navigation retry; drop releases admission; exact Frame route and utility world only | capability, codec, client/server, Chromium adapter, direct/remote composition, shared route, and dedicated-Chrome proof implemented |
| PDF and other artifacts | future content/artifact capabilities | command and artifact byte limits | `ScreenshotBytes` or a reviewed plugin class | fail before retaining oversized result; no truncation | capability not yet implemented |
| JS/element handles | future DOM capability | per-world/page and connection counts | `RemoteHandle` | explicit release; world/page teardown releases descendants | capability not yet implemented |

The independent Locator-screenshot capture composition owns only the
capture/result/wire portion of the table entry. Its injected
selector-to-geometry plugin owns and claims `SelectorBytes` plus any persistent
route state; the application must combine both profiles concurrently before
constructing the shared production ledger. The complete Chromium aggregate
continues to supply and budget the full-Locator resolver for compatibility.

“Not yet implemented” means the feature is absent, not that it may retain data
without a limit. Its future capability must reserve the listed class before it
can be advertised by a production composition.

## Implemented shared ledger

`pwright-core::ResourceLedger` contains only standard-library synchronization
and collections. Cloning the ledger shares one connection budget; cloning does
not create capacity. It exposes:

- atomic `reserve` and `reserve_many` operations;
- global and per-scope checks with checked arithmetic;
- independent scope-record and lease-count caps;
- `ResourceLease::try_resize` for bounded stream growth without release/re-add
  races;
- `ResourceLease::try_replace` for atomic lossy-queue replacement without
  temporarily oversubscribing or discarding the old admitted value on failure;
- current, peak, and rejected counters per class, plus live/peak scopes and
  active/peak leases;
- non-cloneable leases and non-cloneable `BudgetedBytes` for immutable retained
  byte payloads.

Malformed requests such as zero units or duplicate classes are programming
errors and do not consume scope metadata. Admission failures increment the
aggregate rejection counter; class-specific capacity failures also increment
that class's rejection counter.

Production composition has a separate, zero-dependency validation layer. A
`ResourceRequirements` value declares the minimum global and per-scope
capacity for every class a component may reserve, plus the minimum scope and
lease metadata capacity. `merge` takes the maximum for mutually exclusive
requirements or two views of the same peak. `combine_concurrent` saturating-adds
every dimension for independently live components. A composition root must use
the operation matching actual lifetime overlap and must never silently lower a
component declaration.

`ProductionResourceLedger::new` checks that the merged declared plan fits the
configured `ResourceLimits`; `from_pristine_ledger` additionally rejects a
ledger with current or historical use. Components then call `claim` with their
own requirements. A claim returns another handle to that same ledger only when
the component was declared. Failures distinguish an insufficient configured
limit from an omitted component in the declared production plan, so a remote
composition cannot accidentally turn an unbudgeted constructor into its
production path. Constructing or claiming this token has no browser, provider,
transport, executor, or process side effect.

`pwright-composition-chromium::PreparedChromiumAttach` now uses additive
concurrent composition for the raw CDP connection, production topology,
page-session routes, runtime, each selected backend service profile, and each
selected wire protocol. Navigation-only composition does not advertise locator
or include `SelectorBytes`; selecting locator against a plan that omitted its
profile fails before any endpoint is available. The root also rejects
non-extension service scopes, duplicate component scopes, an empty capability
set, and a runtime object bound smaller than the topology it must represent.
The page-route profile reserves six bounded native-ID slots per Page, covering
its stable route key pair, the pair owned by `CdpSession`, and old/new
utility-context values during locator-route replacement; unchanged routes
share their service instances instead of duplicating them. Navigation
and locator factories claim their profiles once and share profile-local
concurrency counters across every Page, so an overprovisioned common ledger
cannot bypass a selected capability's local limit. The server supervisor now
declares one global and one call-scoped `PendingCall` unit per retained command
future. This is intentionally additive with the runtime's own pending-call
lease: the supervisor owns the outer handler/send future while the runtime owns
correlation, cancellation, and cleanup state. Duplicate active call IDs fail
the per-call bound before another future is retained, and clearing active work
releases every supervisor lease.

The optional Tokio/Chromium adapter computes stream transport retention from
the codec before any socket exists. An authenticated accepted connection can
enter the Chromium root only after its production transport claims that exact
declaration from the attachment's fresh ledger; the existing root check then
rejects plan drift or foreign-ledger identity. Pairing the transport with its
endpoint-derived authorization gate adds no queue, lease, task, or second
ledger. Runtime and handshake stages move that same ownership forward.

`ChromiumAttachFactory` does not clone this ledger-bearing preparation. It
stores only immutable recipe inputs, validates them at construction, and calls
`PreparedChromiumAttach::new` for every preparation. Two calls therefore have
different `ResourceLedger` identities even with identical limits and plans.
The Browser root ID can repeat because every resulting remote connection owns
an independent wire namespace; its first child source and topology epoch are
also created anew only after native attachment.

Remote server resources remain optional. Selecting
`ChromiumRemoteHandshakeProfile` adds the caller-supplied production transport
requirements and twice the maximum admitted peer receive-frame size for
server-hello source/output overlap. A larger untrusted ClientHello offer is
rejected before authorization or response encoding; the emitted ServerHello
is tagged with the same ledger. Selecting `ChromiumRemoteControlProfile` adds
one control encode plus one object-scoped `EventItem` lease per subscription.
The two profiles must share the same peer frame ceiling. Without these options,
an in-process Chromium attach does not pay their byte or subscription costs.
Remote navigation and locator handler protocols use the accepted directional
limits, while preparation proves those limits cannot exceed or contradict each
selected capability's declared wire profile.

Subscription activation is a two-phase resource transition. Enable reserves
its `EventItem` lease before awaiting the backend but does not publish the route
until success; failure drops the reservation. Disable keeps both route and
lease while awaiting deactivation and releases them only after success. The
receiver stores that one in-flight edge, so cancellation cannot erase an
uncertain backend side effect or its resource owner. Another control is rejected
until cleanup resolves it.

The supervisor first polls the ordered control. Only handlers that certify the
input frame was released before returning Pending allow a subsequent receive;
the supervisor may then retain exactly one budgeted next frame, never dispatch
it early, and stops receiving again. This preserves the transport's one decoded-
frame bound instead of assuming arbitrary handlers dropped their input. EOF
cancels the control and transfers its retained in-flight lease to close
compensation. No unbounded control or prefetch queue exists.

Object disposal walks matching routes directly in the bounded map, awaiting
and releasing one successful edge at a time; failure retains the current and
remaining leases for retry. Every explicit terminal supervisor path signals
runtime closure and then removes one route before each best-effort backend
disable. It attempts all routes, reports cleanup failures, and leaves zero
subscription leases because retry is impossible after peer ownership ends.
Neither path copies the subscription set or allocates a second cleanup queue.
The async close GAT reaches one completed attempt before the supervisor returns
or drops command futures; receive, protocol, command, control, observer, outbound,
capacity, and resource failures therefore have the same release boundary as
clean EOF. A dropped run future releases Rust-owned leases synchronously but
requires a later owner-supplied async close call for backend deactivation. If
that future interrupts an in-progress disable, the current subscription route
and lease remain retained, and the supervisor leaves the barrier incomplete so
the next owner call can retry it.

Cross-component topology mutation uses a prepare/commit protocol rather than
three independent fallible updates. The production topology candidate owns its
shadow-state lease and rolls back on drop. Page routes build a bounded candidate
under their route lock. The runtime lifecycle batch simulates the complete
graph, reserves the full outbox and all new `LiveObject` leases, and validates
pending-call cancellation before replacing state. The composition publishes
route and topology candidates only after runtime admission succeeds, leaving no
fallible step after the first live mutation. Failed admission therefore retains
the previous topology bytes, object leases, lifecycle queue, and route table.

`pwright-transport::WireFrame::new_budgeted` reserves its aggregate payload and
attachment length as `WireBytes`. A budgeted `WireProtocol` first reserves
twice the maximum structurally attainable frame size in connection scope before
calling its codec. Production codecs must bound variable-size typed source data
by that same frame ceiling, so this covers source and output buffers
simultaneously. After encoding, `WireFrame::new_reserved_up_to` validates the
parts, atomically shrinks the reservation to exact retained bytes, and moves it
to `FrameRoute::resource_scope`: Command/Result use the call scope, Event uses
the source-object scope, and Handshake/Control use the connection scope.
`WireFrame::clone` shares one immutable storage allocation and the same lease,
so cloning a queued frame does not duplicate its byte buffer or accounting. The
final clone returns the budget. Codec and frame failures release the staging
reservation without retaining output.

`BoundedBytesAssembler::new_budgeted` gives non-Tokio and noncanonical-stream
adapters the same rule. It validates the local declaration, reserves the full
declared `WireBytes`, and only then allocates the fragment buffer. Successful
`finish` transfers both the exact bytes and lease into non-cloneable
`AssembledBytes`; incomplete input, construction failure, or assembler/output
drop releases it. The unbudgeted constructor remains for isolated adapters.

For inbound byte streams, `TokioStreamTransport::with_resource_ledger` reserves
the declared envelope in connection scope before allocating or reading its
body. `StreamFrameCodec::decode_budgeted` validates the complete header and all
declared lengths, then obtains an opaque `WireFrameReservation` before copying
payload or attachments. The envelope and retained frame reservations overlap
during the copy because both allocations really exist then; this makes the
reported peak honest instead of disguising the copy as a zero-cost transfer.
After decode, the envelope lease is released and only the route-attributed
frame lease remains. Cancellation while reading, allocation failure, malformed
input, decode failure, and reservation-size mismatch all drop their exclusive
leases without a cleanup gap.

The same Tokio injection covers outbound staging. `encoded_len` validates and
computes the exact envelope size without allocating; the transport reserves it
before encoding. It then reserves the length-prefixed pending-write buffer
before copying. Both leases overlap only while both buffers exist, after which
the encoded envelope and its lease are explicitly dropped. The pending-write
lease stays inside `WriterState` across cancellation, so a later send can finish
the partial frame without creating unaccounted retained memory. Successful
flush or terminal write failure drops that buffer and returns its budget.

`ShutdownFrameTransport` allocates no second frame or shutdown queue. Tokio
shutdown takes the existing writer lock, finishes the one already-accounted
pending buffer, flushes it, and closes the write half. The unique client driver
uses one already-bounded priority send-gate waiter, so only the active writer
can precede shutdown and queued command writers cannot follow it. Cancelling
shutdown retains only the same pending-write state and lease needed by the next
retry. Server shutdown likewise runs after its bounded supervisor work and
control cleanup; preserving two close errors adds fixed result state, not a
retained history.

The optional in-memory transport has a constructor-fixed frame count per
direction. It retains no encoded byte copy: queued `WireFrame` values keep their
existing immutable storage and resource leases. Each direction adds one
`VecDeque` bounded by that count, two closure flags, and at most one replaceable
receive waker. An unpolled send retains its caller-owned frame only inside the
future and never enters the queue. Full queues reject the new frame; receiver
drop clears all queued frames and releases their leases; sender shutdown lets
the peer drain existing frames and then returns EOF. This adapter is not marked
`ProductionFrameTransport`: a future production installation must separately
declare its queue-item metadata rather than treating already-accounted frame
bytes as complete queue accounting.

`TypedEventHandler::new_with_resource_ledger` makes decoded event retention a
separate opt-in composition. A capability supplies a type-specific size
function for variable decoded payload data; every admitted entry reserves
`EventItem` and, for nonempty payloads, `EventBytes` as one transaction in its
source-object scope. Reliable `RejectNewest` overflow returns the unqueued event
without consuming ledger capacity. Lossy `DropOldest` atomically replaces the
old entry's lease, including when the new event belongs to another object, so a
failed replacement preserves the old queue entry. Dequeue, explicit `clear`,
handler destruction, and conversion back into the protocol all release queued
leases. The codec remains responsible for bounding transient decode work; the
event lease measures data retained by the queue after decoding.

`TypedEventProductionProfile` multiplies the queue capacity by its declared
maximum decoded bytes per event, while reserving one multi-class lease and up
to one source scope per slot. `new_with_production_resources` must claim that
complete `EventItem`/`EventBytes`/metadata profile before construction. The
runtime sizing callback is checked against the declared per-event maximum
before touching the ledger, so spare capacity contributed by another event
plugin cannot let this handler exceed its own contract. A zero-byte event still
owns its `EventItem`; zero queue capacity claims and retains nothing.

Event sequencing adds no history buffer or source-indexed map. Each installed
typed server channel retains one `Option<EventSequence>`, and each client
handler retains one last-received scalar plus fixed counters. The eight-byte
sequence travels inside the already bounded wire payload and is covered by
`WireBytes`; the queued wrapper is covered by the existing fixed `EventItem`
slot. Server assignment happens before logical or resource admission, so a
failed admission consumes a sequence but no extra retained allocation. Client
observation happens before local queue admission; a locally rejected event is
therefore counted as received and its returned value remains the explicit
evidence of local loss. Connection-epoch reset clears queued items and their
leases before accepting a new baseline.

`CdpTopologyProductionProfile` derives a native-string storage ceiling from
the topology's context/page/frame/world counts and per-string byte limit. The
formula counts every retained duplicate across forward/reverse indexes,
pending or materialized records, session keys and owner values, crash status,
and execution-world keys. Production topology inputs are compacted before
retention so a short caller string with large spare capacity cannot bypass the
bound; the live lease measures actual retained `String::capacity()`, not an
estimate. Fixed records and map nodes remain bounded by the independent count
limits described above. The composition root must also assign an explicit
`ResourceScope::Extension`; generic connection/object/call scopes are rejected,
so separately installed adapters cannot silently collide in one scope.

Each production mutation first reserves one maximum-size `BackendStateBytes`
shadow lease, then clones and evaluates the pure state transition. A topology
or resource failure drops the shadow and leaves both live topology and its
exact lease unchanged. Success shrinks the shadow lease to the new exact byte
count, swaps it with the live state, and releases the old state and lease. The
production requirement consequently covers at most two topology copies, one
extension scope, and two leases. This deliberate bounded overlap makes atomic
failure semantics and the transient clone peak visible instead of hiding it.

`CdpLocatorService` itself has no variable-size retained service state: its
client handle, retry duration, one `OnceCell<()>`, and atomic counters are fixed
size. Locator memory instead belongs to each active action. A
`CdpLocatorProductionProfile` multiplies an explicit concurrent-action bound by
a conservative per-action `SelectorBytes` peak covering selector/value input
capacity, formatted selector labels, generated click/fill/query JavaScript,
selector runtime installation, a bounded text-content result, and intermediate
string-allocation overlap. Depth, node, aggregate string, fill-value, and
text-content limits constrain the derivation.

Production `call` validates logical size and spare string capacity, obtains a
clone-shared local action slot, and reserves the full byte peak synchronously
before returning the future. This means an unpolled future is already charged;
rejection captures neither command data nor browser work. Completion,
cancellation, or future drop releases bytes before returning the local slot.
The local counter prevents this locator plugin from borrowing another plugin's
unused global ledger allowance. Raw constructors remain explicit unbudgeted
test/compatibility paths.

The command-free selector compiler/runtime is now a separate leaf with the
same depth, node, aggregate string, retained-capacity, generated-script, and
runtime-install bounds used by the complete Locator backend. The first-party
selector-geometry plugin adds its own `CdpSelectorGeometryProductionProfile`.
It reserves one conservative `SelectorBytes` peak before an async resolution
can retain selector labels, geometry and scroll scripts, or installation
source. Its clone-shared local gate prevents borrowing unrelated ledger
headroom. Optional Frame utility-world routes are separate persistent
`BackendStateBytes` leases, bounded by route count and native-ID bytes and
released only after the last service clone drops. No resolved DOM or protocol
handle survives an evaluation attempt.

Frame locator routing has a separate persistent `BackendStateBytes` boundary.
`with_context_routes` derives its route count and native-ID ceiling from the
selected topology profile. Each route may retain its target ID, session ID,
selected-world `uniqueContextId`, and the target-local CDP client's copies; the
reservation doubles that five-string bound so current and replacement tables
can overlap during atomic synchronization. The route lease is acquired before
any Frame strings can be retained. Route count, every ID, and actual compacted
string capacities are checked before commit. Page routes therefore use only
their four-string target/session bound, and Page locator calls reuse the main
Frame route instead of retaining another utility-context copy.

Locator 1.3's negative-text, relative `has`/`hasNot`, and visibility filters
remain inside the same selector depth, node-count, aggregate string, generated
JavaScript, and transient-overlap bounds. They add no retained DOM handle,
server object, cache entry, route, or per-filter resource class. The semantic
runtime is versioned and installed once per execution context; extending it
cannot silently reuse an older runtime lacking the new visibility primitive.

Locator 1.4's regex source and canonical flags are charged to the same
aggregate selector-string and retained-capacity limits on construction,
encoding, decoding, and backend admission. Regex objects exist only during one
browser-side selector evaluation; no compiled expression, match result, or DOM
identity is retained across retries. Runtime v3 prevents a context containing
the older string-only runtime from being reused. Invalid ECMAScript source
terminates as typed `InvalidSelector` rather than entering an unbounded retry.

Locator 1.5's placeholder, alt, title, and test-ID values remain inside the
same per-selector node, aggregate string-byte, retained-capacity, escaped
JavaScript, and transient-overlap bounds. `TestIdAttributes` additionally caps
one immutable comma-separated name list at 16 entries and rejects empty or
unsafe names before encoding or backend admission. Expanding those names into
a temporary JSON array is covered by the existing per-node generated-script
allowance. Runtime v5 retains neither configured names, compiled regexes, nor
DOM matches after one evaluation.

Locator 1.6 `count` uses the same selector depth, node, string-capacity,
generated-script, concurrent-operation, and `SelectorBytes` admission bounds.
It evaluates one non-strict snapshot and copies only a checked `u32`; the
temporary browser-side match array and DOM nodes are not retained. A replaced
runtime permits exactly one reinstall plus one further evaluation, preventing
navigation recovery from becoming an unbounded loop. Snapshot evaluations are
reported separately from actionability evaluations.

Locator 1.7 visibility shares those same bounded snapshot resources and copies
only one boolean. It retains no node, geometry, style, or hidden-state cache.
The derived `is_hidden` future owns only the in-flight `is_visible` future and
negates its result; it creates no additional wire frame or browser operation.
Runtime replacement recovery remains bounded to one reinstall and one second
evaluation.

Locator 1.8 checked/editable/enabled queries reuse the same selector-storage,
generated-script, concurrency, and `SelectorBytes` admission peak. Each
evaluation copies one boolean or one bounded typed error and retains no DOM
handle, state history, or result cache. Missing and detached targets may retry,
but only inside the runtime-owned call deadline; every attempt increments the
separate `state_evaluations` metric and holds the same single operation lease.
The derived `is_disabled` future retains only one in-flight `is_enabled`
future and creates no second wire frame or browser evaluation.

Locator 1.9 `wait_for` reuses that same selector storage, generated-script,
concurrency, `SelectorBytes`, and one-operation-lease boundary. Each attempt
copies only the state outcome or a bounded typed error; it retains no DOM node,
ElementHandle, geometry, style, or match history. Unmet attached/visible and
detached/hidden conditions retry only within the runtime-owned caller
deadline, and every attempt increments the separate `wait_evaluations` metric.
The default visible state is represented by a small enum value rather than a
second command or backend allocation.

Locator 1.10 `text_content` uses the same single operation lease and retry
deadline while adding an explicit 1 MiB UTF-8 output ceiling to the per-action
`SelectorBytes` peak. The backend moves the validated JSON string out of the
CDP response, compacts spare capacity, and returns `Option<String>`; null stays
null and exact whitespace is preserved. Strictness, invalid selectors,
malformed non-string values, and oversized results are terminal. Each attempt
increments `content_evaluations`, and no DOM node, ElementHandle, result
history, or cache survives the call.

Locator 1.12 `inner_html` is a third mutually exclusive content-result branch.
It moves and compacts at most 1 MiB of browser-serialized UTF-8, reusing the
same operation lease, retry deadline, selector storage, and
`content_evaluations` counter. The production peak remains the maximum of raw
text, rendered text, and serialized markup—not their sum—and the call retains
no node, markup history, or cache after completion.

Locator 1.13 `get_attribute` adds a mutually exclusive 64 KiB attribute-name
input branch and a nullable 1 MiB attribute-result branch. The input peak is
`max(fill value, attribute name)`, the generated-script peak is
`max(fill script, attribute script)`, and the copied-result peak is the maximum
of text content, inner text, inner HTML, and attribute value—not their sum.
The backend rejects an oversized name before CDP, compacts a present result,
and records retries in `attribute_evaluations`; it retains no element identity,
attribute history, or cache after completion.

Locator 1.14 `input_value` adds no variable-size command input. Its non-null
UTF-8 result is capped at 1 MiB, moved out of the CDP response, compacted, and
accounted as another mutually exclusive locator-result branch rather than
being summed with text, markup, or attribute output. Retries increment the
independent `input_value_evaluations` counter. The shared runtime retargets an
associated label at evaluation time and no DOM identity, value history, or
cache survives the call.

Locator 1.15 `all_text_contents` adds the first retained collection result.
The wire and backend reject more than 4096 strings, any string above 1 MiB, or
more than 4 MiB of aggregate UTF-8. The production peak adds that aggregate to
the simultaneous `Vec<Value>` and `Vec<String>` container slots used while
validated JSON strings are moved and compacted. This collection branch is the
maximum against scalar locator results, not their sum. It is an immediate
non-strict snapshot, increments `all_text_contents_evaluations`, permits only
one runtime-reinstall retry, and retains neither the match array nor DOM nodes
after completion.

Locator 1.16 `all_inner_texts` uses the same collection ceilings and transient
`Vec<Value>` plus compacted `Vec<String>` shape, so it does not increase the
per-call maximum. A call can return either raw text contents or rendered inner
texts, never both; production therefore takes the maximum of the two equal
collection peaks rather than their sum. Its independent
`all_inner_texts_evaluations` counter distinguishes layout-sensitive reads,
while the immediate snapshot, one runtime-reinstall retry, and handle-free
completion keep the lifetime boundary identical.

Locator 1.17 `hover` adds no variable-size result and no DOM-handle lifetime.
Its command owns one already-bounded selector, fixed scalar flags, an optional
fixed-size modifier bitset with at most five members, and an optional fixed-size
pair of finite `f64` coordinates. Modifier presence itself is retained because
omission and explicit empty have different Page-key semantics.

Locator 1.18 `focus` and `blur` add no variable-size result, input transaction,
or DOM-handle lifetime. Each owns one already-bounded selector and one compact
evaluation script, shares the existing Locator action admission and selector
byte lease, and returns only a fixed status record. The two operations are
mutually exclusive call branches, so production resource requirements retain
the existing maximum instead of summing focus and blur peaks.

Locator 1.19/2.2 Press adds one
128-byte key expression, at most 16 stack-resolved key descriptors, and one
fixed 16-slot pressed-key record per selected Page. Events remain sequential;
delay shares the deadline scheduler and navigation waiting shares the existing
Page event cursor. The existing mutually exclusive Fill input peak is larger
than the Press expression, so the selector-action reservation does not grow.

Each selected Page owns exactly one fixed-size input coordinator, bounded by
the existing `max_pages` topology limit and shared by all of that Page's main,
local-frame, and OOPIF Locator routes. It contains one mutex, two finite pointer
coordinates, at most one pressed mouse button, a pressed-modifier bitset, one
fixed 16-slot physical-key record, and
at most one pending-restore bitset; it owns no queue, task, map, DOM handle, or
selector. One transaction holds the gate across key transitions and the pointer
action. Completion restores immediately. Controlled runtime cancellation
releases a pressed button and restores a transaction whose action future was
dropped; if a backend future is dropped outside that control path, the next
transaction performs the same bounded repair before accepting new input. Trial
may dispatch temporary modifier key events but terminates before mouse dispatch.

The existing Click command adds no wire or allocation peak at this checkpoint.
It now uses the same fixed Page state to emit mouse move, press, and release.
The pressed-button record is updated only after successful CDP acknowledgement;
a failed release stays pending for the cleanup retry. Cleanup failure remains a
typed terminal error instead of pretending that browser input state is known.

The extensible-record prototype bounds forward-compatible option bags before
reading any unknown data. Every schema must publish an unchanged maximum for
the complete record body; the decoder checks that `u32` length before taking
the slice. Field iteration is zero-copy and retains only a borrowed reader
cursor plus the previous `u16` tag. Skipping an unknown field allocates
nothing. Encoding
uses one output `Vec` and backpatches record and field lengths in place, so it
does not temporarily duplicate the option bag. Per-field string, byte,
repeated-item, nesting, and message-node limits still apply inside known field
bodies. Locator 2.0 now assigns Click options a 128-byte maximum record body;
its empty default occupies only the four-byte record-length prefix and the
complete current field set stays under the declared bound. The generated wire
model changes no locator 1.x production budget. Locator 2.x maps the public
option bag without an extra retained copy and caps `steps` at 10,000. The CDP
adapter emits those mouse moves sequentially from fixed Page input state, so
the count bounds work but does not multiply retained memory. Its navigation
barrier adds one fixed selected event cursor per active waiting Click and shares the
connection's already-budgeted immutable event payloads; it creates no second
reader or event queue. Locator 2.x can now enter a production profile only
through the explicit `with_v2_track` opt-in. The advertised support range is
built from a separate v2 inventory, and the accepted handshake installs only
the selected major's exact codec and handler tree. Enabling the track adds no
backend-state, queue, wire-byte, or retained-allocation requirement; disabling
it restores the v1-only manifest with identical resource requirements.

Locator 2.1 Focus and Blur reuse the same selector, call, retry, and utility-
world evaluation budgets already declared for locator 1.18. They add no input
transaction, retained DOM handle, queue, or second mutually exclusive peak.
The exact selected 2.x minor is a fixed codec value, not a per-call allocation.

Locator 2.2 Press reuses the same Page input owner and selector-action lease as
1.19. Its key record is fixed-size, every acknowledged down/up mutates only
that record, and failure or controlled cancellation releases remaining keys in
reverse order before later input can acquire the Page gate.

Locator 1.20/2.3 SetChecked adds one boolean and a bounded 64-byte extensible
option record to the already-bounded selector command. Pre-state,
actionability, and post-state results are fixed records; Check and Uncheck are
client composition and add no routes. The real action uses the existing Page
mouse transaction and selector-action lease. It retains no DOM handle, result
string, queue, map, task, or navigation cursor, so production takes the
existing Click maximum rather than summing another concurrent branch.

Locator 1.21/2.4 SelectOption bounds both request and result to 1,024 items,
64 KiB per string, and 1 MiB aggregate string bytes. The request retains only
descriptors and the generated script for one mutually exclusive locator
attempt; the production profile reserves their larger maximum rather than
summing them with Click, Press, or SetChecked. Each retry returns a fixed
status until every descriptor has an enabled match. Successful mutation
returns bounded selected values in DOM order and retains no element handle,
queue, map, pointer state, or navigation cursor. ElementHandle selection is
not hidden in this budget: it remains outside this slice pending a separately
negotiated cross-world Handle adoption contract.

Locator 1.22/2.5 ScrollIntoViewIfNeeded reuses the existing locator operation,
selector, utility-world installation, and action-script branches. Its request
adds only one already-bounded selector and its result is unit. The asynchronous
precommit script is dropped before the synchronous commit script is built, so
only one script branch is retained at a time. Every retry returns a fixed
status after releasing renderer-local references; no DOM
handle, point, observer, input transaction, navigation cursor, queue, map, or
task survives the evaluation. The generated script is below the existing
focus-script bound, which is already below the SelectOption-dominated action
maximum, so this mutually exclusive command adds no resource class and does
not increase the declared production peak. Cancellation before the scroll
commit is stateless; cancellation after commit cannot undo the scroll and must
never replay it.

The generated 22-command locator handler tables are split into two balanced
typed groups.
This is a build-resource boundary rather than a runtime heap allowance: it
prevents dual-major compilation from expanding a pathological left-nested
error/event type. It does not add a runtime collection or retain the unselected
major's handler tree.

The generated action script remains below the existing maximum action-script
branch, and each attempt uses the same locator operation lease and caller
deadline. Retries increment `hover_evaluations`; completion retains neither the
element, point, nor platform marker.

Client `Locator.all()` does not allocate a vector proportional to the match
count. `LocatorList` retains one moved base Selector plus a `u32` snapshot
length; each `get` or iterator step creates only the requested `nth` Locator.
This keeps hidden retention constant in N while making any caller-chosen
collection of yielded Locators explicit. The Count command still owns its
normal bounded call/selector state. A count above `i32::MAX`, which the current
portable nth AST cannot encode, fails before list construction. Client
`clear()` adds no state at all beyond the existing empty-value Fill call.

`CdpScreenshotProductionProfile` declares the decoded-byte maximum and maximum
simultaneous captures. Its shared factory allows one active capture per Page,
enforces the connection-wide count with atomics, and accumulates no waiting
requests. Admission reserves a full per-call `ScreenshotBytes` lease before CDP
response decoding. The exact padded-base64 decoded bound is checked first; an
oversized response fails without allocating the decoded vector. Successful
decode moves the vector into `BudgetedBytes::from_reserved`, which shrinks the
existing lease without a release/reacquire gap. Completion, decoding failure,
and dropping a pending capture release both Page/global admission and unused
bytes.

Remote encoding retains the backend `ScreenshotData` while separately
accounting the attachment copy as `WireBytes`. The client codec validates one
and only one attachment, its declared index and byte length, and its own
configured ceiling before copying it into a call-scoped `ScreenshotBytes`
lease. Dropping the client result releases that lease. The CDP base64 string is
bounded by the connection response limit and is not misreported as decoded
artifact retention.

Locator 1.11 `inner_text` shares that operation lease, retry deadline, and
`content_evaluations` counter, but its result is a non-null rendered string.
The backend moves and compacts at most 1 MiB of UTF-8 after proving the target
is an HTML element. Because a call can return either text-content or inner-text
but never both, the production content-result peak is the maximum of the two
1 MiB branches rather than their sum; no DOM handle, layout object, result
history, or cache survives the call.

`CdpNavigationLimits` bounds every variable-size value retained by an active
Chromium goto action: input and committed URLs, frame/loader/request identifiers,
simultaneous request IDs, response-header count and aggregate name/value bytes,
and status/error text. `CdpNavigationProductionProfile` multiplies a
conservative per-operation `BackendStateBytes` peak by an explicit concurrent
operation bound and requires a composition-selected resource scope.

Production `call` acquires a clone-shared local slot and reserves the full peak
synchronously before returning its future. Thus an unpolled future is charged,
and unused capacity in the shared ledger cannot let this plugin exceed its own
concurrency contract. Event processing checks request identifiers and the
inflight count before mutation. Response parsing checks header count, JSON value
type, and aggregate bytes before creating the owned vector. Retained strings
are copied into compact allocations. Completion, cancellation, and both
unpolled and polled future drop release the resource lease before returning the
local slot.

This profile ends when the backend future yields its portable result. Retention
by application code after an in-process return is caller-owned. Remote response
encoding and pending protocol transport writes have separate owners and carry
their own leases. The selected CDP event delivery path is separately bounded as
described below.

Legacy/v1 callers retain Tokio's broadcast receiver API and its bounded replay
ring. A v2 production profile may instead select the task-free
`ReclaimingEventFanout`. It preallocates bounded event and subscriber slots,
retains no event published without a subscriber, and removes a slot as soon as
every subscriber that existed at publication advances beyond it. A slow cursor
that crosses the event bound receives an explicit lag count; connection close
allows queued items to drain before returning `Closed`. Dropping the final
cursor reclaims every unread event.

The composition profile derives the required subscriber capacity from selected
plugins and their declared concurrency: topology and PageLoad each add one,
network adds its maximum Page lanes, and navigation plus Locator add their
maximum active operation counts. An explicitly smaller fanout fails production
validation before CDP I/O. Legacy delivery remains valid for preserved v1 and
non-opt-in profiles.

The raw CDP layer has independent hard limits for pending commands, write and
event channel items, serialized command/response/event/error bytes, and the
WebSocket frame/message ceiling. Pending admission is synchronous and
clone-shared. Its RAII registration removes an unpolled or cancelled command;
the shutdown failsafe drains the table even when the reader future is dropped.
Serialization writes into a bounded buffer rather than allocating an
arbitrarily large command and checking afterward.

`CdpConnectionProductionProfile` accounts the reader's maximum raw-message and
parsed-JSON overlap as a fixed `WireBytes` lease. Every admitted command owns a
full command/result peak shared by its pending entry and outgoing queue item;
the lease therefore survives caller cancellation while the writer still owns
the bytes. Each unique incoming event owns exact `EventItem` and `EventBytes`
units. `CdpEvent` clones share its immutable payload and lease, eliminating
subscriber-count deep copies. The profile reserves event capacity plus one
maximum event because the incoming value is allocated before either delivery
strategy performs bounded admission. Reclaiming mode additionally reserves one
fixed `BackendStateBytes` lease for event/subscriber bookkeeping. Release tests
prove that delivered values keep the shared payload lease alive and that it
returns to zero after all cursors and values drop. Event oversize or budget
exhaustion terminates the connection instead of silently omitting protocol
evidence. The production attach connector rejects an omitted declaration,
config/profile mismatch, or undersized subscriber plan before network access.
Fanout metrics expose both current and peak subscriber counts. The complete
profile derives a safe bound of 10, while the canonical shared-network recipe's
real Chrome overlap proof observed 6 or 7 cursors at peak and returned to three
steady cursors, zero queued events, and zero lag.

`DirectDispatcher::with_resource_ledger` and
`ControlledDispatcher::with_resource_ledger` install the ledger only while the
runtime is pristine. Each successful Create reserves one `LiveObject` in the
new object's scope after graph/outbox validation and before graph mutation.
Adopt changes only parent ownership and keeps the same object-scoped lease.
Child-first Dispose and transport-close drain remove the exact corresponding
leases; a rejected Create leaves both the graph and lifecycle outbox unchanged.

When the lifecycle outbox is enabled, Create, Adopt, and every child-first
Dispose message also own an `EventItem`. A multi-message Dispose reserves the
entire ordered batch before changing the graph, so exhaustion cannot leave a
half-disposed tree. Merely claiming the next message retains its lease; rolling
the claim back therefore keeps both message and capacity, while pop, claim
commit, queue clear, and transport close release the exact front lease.

`PreparedObjectDisposal` reserves that same complete child-first batch without
yet changing the object graph. Its single active transaction token excludes
competing lifecycle mutations while an owner awaits backend cleanup. Drop
returns all reserved leases; commit transfers them to the outbox and removes
the objects; transport close drains the runtime and invalidates the token. No
wire payload is allocated by preparation. This bounds the otherwise dangerous
gap between successful native cleanup and local lifecycle publication, but the
composition layer also serializes its native-ID topology before invoking the
backend.

For remote close, `OrderedRuntimeControl` bounds its active and waiting writers
to the supervisor's declared in-flight commands plus the singleton outbound
lane. These are not a second command queue: each waiting barrier remains inside
an already-accounted supervisor command future, and the one ordinary sender is
the fixed outbound future. The priority gate is allocation-bounded at
construction. It is acquired before CDP work, excludes ordinary claims, and
drains already-budgeted outbox entries through the exact context Dispose.
Dropping a waiting/acquired barrier removes or releases its slot and wakes the
next barrier before ordinary output. Result encoding and its existing
`WireBytes` reservation begin only after that control sequence finishes.

An explicit close also reserves one slot from an injected
`ReservedCleanupExecutor` before constructing browser work. The reservation is
counted together with queued cleanup jobs, so direct submission cannot steal
capacity from an admitted close. Normal completion or unpolled Drop releases
the slot. If a polled future is abandoned while pending, consuming the
reservation moves that exact boxed future into the queue without rechecking
capacity; the existing topology, runtime, CDP, and wire leases move with it.
The queue counts accepted, completed, and failed continuations. It does not
spawn, poll, retry, or choose shutdown policy for the owner.

The same runtime injection reserves `PendingCall` in call scope before inserting
the call table, cancellation signal, or scheduler admission. The lease covers
target/connection queueing, backend execution, cancellation wait, and an
abandoned operation. Completion and future drop release it. Parent or transport
close deliberately retains the call lease after releasing affected object
leases, because the cancelled call remains owned until its cleanup path reaches
a terminal state.

Only a connection-wide execution permit reserves `ActiveCommand`; a target head
waiting for that permit remains solely a `PendingCall`. Immediate admission
reserves before scheduler state changes. Releasing an active permit atomically
moves its lease to the oldest connection waiter and changes the call scope
without a release/re-reserve window, so current usage and peak usage remain
truthful at capacity. Cancellation, failed admission, completion, and dropping
either an unpolled granted acquisition or its permit release the exact lease.

`FrameLimits::max_frame_bytes` derives the largest structurally attainable
frame body rather than assuming every independent limit can be reached at
once. `WireProtocol::production_resource_requirements` declares twice that
capacity for the typed-source/encoded-output overlap, and
`with_production_resources` refuses to encode unless that class was present in
the plan. Every budgeted
`WireFrame` records the exact ledger identity, including a zero-byte frame;
equal numeric limits from another ledger do not satisfy that proof.

`StreamFrameLimits::retention_bounds` solves the attainable envelope/body
maximum while attachment headers grow and remaining body room shrinks. Its
outbound peak includes the input frame, encoded envelope, and length-prefixed
pending write. Its inbound peak includes the envelope and decoded frame; the
full-duplex requirement is the sum because one reader and one writer may run
concurrently. `TokioStreamTransport::with_production_resources` claims that
complete byte/scope/lease profile. Production send then rejects any outbound
frame not budgeted by the very same ledger before encoding or allocating
staging memory. Extra concurrent send attempts still contend against the hard
ledger rather than gaining hidden capacity.

`EncodedClientChannel::new_with_resource_ledger` and its control-enabled and
post-handshake variants apply the same `PendingCall` class on the client side.
Registration reserves before correlation, pending-map, send-gate, or transport
mutation. A normally delivered result releases the count when correlation ends,
even if its separately wire-budgeted frame awaits application consumption. If a
possibly sent future is dropped, the lease moves with the bounded tombstone;
sending a cancellation is not terminal and does not release it early. The late
result, send/receive failure, EOF, explicit close, or receive-driver drop clears
the exact remaining leases. Unbudgeted constructors remain available for
isolated and compatibility compositions.

The production client path additionally requires a `ProductionFrameTransport`.
That small codec-neutral trait exposes the transport's retained-resource
profile and exact connection ledger; Tokio is only one possible implementation.
`ClientWireProductionProfile` combines it with the bounded pending-call
backlog. Its wire-byte maximum covers both attainable peaks: full-duplex I/O
plus the other pending slots' frames, and a full pending table plus the next
frame encoded before its registration is rejected. Its metadata bound includes
pending leases, frame leases, and transport staging leases. Direct,
control-enabled, and post-handshake production constructors claim this complete
profile and reject a production transport from another ledger.

The server's outbound lifecycle adapter uses the same budgeted control protocol
and transport handle as its inbound receiver. The supervisor retains at most
one outbound attempt and rotates it with receive and active command lanes. An
idle result is parked on the plugin's readiness future, whose default never
wakes; receive or command progress also retries it. An empty runtime outbox
therefore cannot consume CPU in a ready-loop. A successful send
commits the runtime's existing bounded outbox claim; encoding or transport
failure leaves that claim in place and terminates the supervisor distinctly.

When negotiated graceful close is installed, the Chromium control profile adds
one further protocol-encode peak. The ordinary lifecycle outbound future may
already retain its maximum frame while the ordered inbound Close barrier must
encode CloseAck, so counting only one peak would make a valid shutdown depend
on scheduler timing. Both protocols claim the same connection ledger; actual
frames reserve and release exact bytes. The reserved `pwright.control`
handshake entry also consumes one explicitly bounded capability slot and fails
composition before I/O when that slot or its ID-byte bound is unavailable.

`pwright-server-events` supplies the matching typed event-output boundary. One
production profile adds a bounded `EventItem`/`EventBytes` queue to one maximum
protocol encode, while the separately declared transport covers its own
retained I/O state. A claimed front event remains logically inside capacity
during an outbound attempt. New producers therefore cannot evict it under
`DropOldest`; a failed or cancelled attempt restores it without a second
enqueue count for ownership accounting. Such an attempt is still terminal for
that connection because the transport may own a partial write. A successful
send commits the claim and releases the decoded payload lease while the
budgeted `WireFrame` continues to own its exact `WireBytes` until the transport
releases its final clone. Chromium remote event
profiles are optional and additive, and their accepted-handshake factory
rejects a production transport that does not share the composition ledger.
Each typed queue stores at most one outbound waker; accepted enqueue takes and
wakes it, while static chains race only readiness futures and keep actual sends
serialized.

The concrete `PageLoad` event adds no variable decoded payload bytes: its
portable payload is zero-sized and its canonical encoded payload is exactly 17
bytes (kind, sequence, and duplicated source Page identity). The
`BrowserContextPage` and `PagePopup` payloads each retain one `ObjectId` and
encode to 25 bytes by appending the related Page identity. Their client queues
use `RejectNewest`; capacity, `EventItem`, overlapping encode `WireBytes`, and
transport staging remain declared by the existing per-event production
profiles. CDP lifecycle activation owns no retained queue or background task
and reuses the exact synchronized Page route and CDP client retained by
`CdpPageServices`. Topology-derived subscriptions activate no CDP stream, and
their explicit publisher owns no queue beyond the independently installed
typed senders.

The topology connection pump retains only its driver, two optional typed
sender handles, a nonzero reconstruction limit, counters, and terminal state;
it adds no collection, queue, task, timer, or transport reader. Each successful
same-connection recovery consumes one bounded reconstruction attempt and one
strictly increasing `ConnectionEpoch`; epoch exhaustion is terminal rather
than wrapping. Portable discovery lanes prohibit replay and convert CDP loss,
publication overflow, driver failure, or a dropped polled turn into a terminal
connection state. Supervisor termination records `BackendEventFailed`, runs
the reasoned control cleanup once, and attempts transport shutdown even if
cleanup fails.

The server backend lane adds no event queue and no concurrency multiplier. Its
Chromium adapter retains at most one boxed pump future, moves the single pump
into that future, and restores it before reporting the outcome. Receive,
commands, outbound delivery, and backend work are polled with rotating
priority, but only the existing bounded command vector can contain more than
one operation. Dropping another lane's ready future cannot drop the backend
turn. Dropping the entire caller-owned connection run remains terminal; its
backend termination hook drops that one retained turn synchronously and marks
the adapter closed. No browser work is orphaned in a task.
Leaving the Page domain enabled after the final listener is intentional shared-
domain safety; the lifecycle-event toggle itself is disabled.

Fresh remote assembly does not introduce another budget or another owner. The
transport builder must prove both the declared transport plan and the identity
of the attachment's fresh ledger before a supervisor exists. Runtime install,
handshake acceptance, and application install are consuming stages; an error
drops the attachment, supervisor, runtime, authorization, topology driver, and
all their leases together. The application context cannot be independently
constructed and exposes the supervisor's exact shared transport. A selected
topology driver must leave that context exactly once. The root adds the
task-free CDP liveness lane around every caller backend without adding a queue
or capacity multiplier, and the final complete run invokes shutdown on that
same transport after terminal cleanup.

The original unbudgeted constructors remain for isolated codecs, tests, and v1
compatibility. `RuntimeLimits::resource_requirements` now declares its exact
`LiveObject`, `PendingCall`, realizable `ActiveCommand`, lifecycle `EventItem`,
scope, and lease minima. `DirectDispatcher::with_production_resources` and
`ControlledDispatcher::with_production_resources` claim that declaration and
reject an omitted or undersized runtime plan before state exists. The common
wire protocol and Tokio stream transport now enforce the same production claim
and exact-ledger identity. The encoded client now composes through the same
proof. Typed event queues, Chromium topology, selected navigation/locator
services, the server supervisor, handshake, and control subscriptions enforce
their own production profiles as well. A complete deployed remote server must
still add its concrete transport requirements to the optional handshake
component; each installed typed event output now contributes its own queue and
encode profile. Preserving v1 does not authorize an unbounded v2 server profile.

## Eviction matrix

| Data category | May drop? | Required behavior at limit |
| --- | --- | --- |
| Object lifecycle, command result, cancellation | No | backpressure/reject before mutation or terminate an invalid connection |
| Pending calls and active commands | No | reject admission with `ResourceExhausted` |
| Page close/crash and protocol diagnostics | No | reserve protected capacity or close the affected connection |
| High-rate telemetry/event samples | Yes, by declared event class | RejectNewest or DropOldest and increment loss metrics |
| Response body requested by user | No silent truncation | bounded streaming or `ResourceExhausted` |
| Logs | Only when configured lossy | bounded ring with dropped item/byte metrics |
| Trace/HAR/video | Not silently | bounded asynchronous spill; stop recorder on sustained pressure |
| Screenshots/PDF | No truncation | fail the command before retaining the result |
| Selector/query/cache | Cache entries only | reject oversized query; LRU/epoch-evict bounded cache entries |
| Remote JS/element handles | No implicit live-handle eviction | explicit release or parent/world disposal |

## Metrics schema

Each connection exposes a `ResourceMetrics` snapshot. For every class it
contains:

```text
global_limit
per_scope_limit
current
peak
rejected
```

The snapshot also contains `live_scopes`, `peak_scopes`, `active_leases`,
`peak_leases`, and `rejected_reservations`. Component-local metrics remain
necessary: queue drop counts, object counts, pending calls, scheduler waiters,
topology records, selector-runtime installations, and cleanup failures explain
which subsystem owns the shared units.

`scope_metrics(scope)` returns current units and active leases for one known
connection, object, call, or extension scope without cloning or exporting the
whole bounded scope table. Diagnostics can therefore attribute pressure to a
specific Page/call/plugin while global metrics remain allocation-free.

OS measurements must report the Pwright client/runtime process separately from
the browser parent, renderer, GPU, and utility processes. Ledger values explain
retained logical data; RSS/allocator measurements capture allocator overhead,
code, stacks, and fragmentation. Neither number substitutes for the other.

## Remaining DT-060 verification

The common accounting contract and bidirectional retained-wire integration are
complete, fresh staged remote assembly preserves one ledger identity through
transport shutdown, typed client and server event queues can share item/byte
budgets, and Page screenshot bytes now retain exact backend, wire, and client
leases across their distinct lifetimes. DT-060
also covers runtime live objects, client/runtime pending calls, and active
commands. It remains open until all advertised v2 producers use the appropriate
lease for retained payloads; backend and capability production factories
reject missing declarations as runtime, wire, encoded client, and typed events
now do; and the DT-003 memory workloads verify teardown and steady-state RSS
separately for runtime and browser processes. The optional process-metrics
probe now makes that OS split executable with bounded snapshot, record, and
command inspection. Its opt-in recorder correlates those samples with internal
ledger and object snapshots, enforces a cleaned warm comparison baseline, and
has real-Chrome idle external-attachment, Page/Context, Playwright-shaped
navigation, and retained-Handle teardown proofs. Handle observations correlate
native routes, runtime objects, and `RemoteHandle` units, including explicit and
parent retirement. Idle teardown separately proves runtime invalidation, CDP
connection-owner release, and external-browser survival through reattachment.
The drivers also distinguish immediate logical cleanup from bounded Chrome
process-tree settlement before post-teardown RSS sampling. A bounded exact-run
series now rejects mismatched or unclean reports and records diagnostic RSS
ranges; the navigation driver has one and the network driver has two consecutive
three-run Chrome series. Other
workload series and reviewed thresholds remain open.
