<!-- agent-updated: 2026-09-17T05:42:44Z -->

NODE-164 (2026-09-17): Module ownership now applies to all exported raw native classes, not only JS Handle sidecars: the crate-wide facade produces module-local TypeTag identities consumed by the real N-API checks before typed access. Prototype changes cannot grant foreign Rust-layout authority; session/world/liveness checks remain separate. Portable property results retain independent Frame-owned identity through shared publication. This supersedes the priority raw-class implementation gap; [exact verification and remaining Node batch gates](pwright-node-handles.md) are recorded canonically.

NODE-163: A Handle-targeted producer creates an independently owned result parented to the source Frame, not to its source Handle. Staged publication revalidates both target and argument Handles, even for expressions ignoring them. Native and JS publication failures retain or explicitly release ownership rather than relying on eventual GC. A future property batch must distinguish sibling cleanup tickets; one CallId is not one result identity.

NODE-162: String expressions retain and validate argument owners just like functions; ignored values do not grant permission to skip session/world/liveness checks. A disposed evaluation target is retained as error data, not a descriptor, until the existing converter settles. Live references still remain owned throughout native execution/publication.

NODE-161: The same Live/Disposed state is shared by NativeHandle and its native-branded JS wrapper sidecar. Only that native binding produces owned argument references; visible IDs or JS brand strings cannot create Handle authority. Branding is module-local: another loaded addon cannot authorize a cast into its potentially different Rust sidecar layout. Owned slots hold descriptors through async execution, while explicit disposal still drops the wrapper's own descriptor and preserves completion-only receipts.

NODE-160: Public Handle wrappers preserve the native retained object's identity and expose ElementHandle discrimination. An explicit dispose transitions the wrapper from an owned descriptor to a shared completion-only receipt; concurrent calls await the same release, and session shutdown uses that same ticket. Retaining disposed wrappers does not retain service leases.

NODE-159: Unpublished native envelopes own both admission and recovery; successful publication transfers the same recovery slot into an Owners ticket before replying with PendingHandle. Unread replies and queue close cannot drop an unowned browser result. Descriptor storage remains distinct from the native work fence.

NODE-158: A PendingNativeHandleCreation owns a native result and route admission but is not a portable graph object. It exposes frozen world identity before publication. Only Published returns a HandleInitializer; Deferred retains ownership, and Rejected still owns explicit asynchronous compensation.

NODE-157: A pending Handle cleanup ticket can retain the exact creation-world lifetime independently of its portable descriptor. Frozen source identity, not current Frame routing, selects the witness. A failed release can count as parent cleanup only after both native objects and the local Handle are retired; raw detach is not proof.

NODE-156: Native release receipts retain the exact Arc allocation owning a route's resource lease. Old receipts cannot retire a replacement route even when ObjectId, payload and disposer are reused. Parent retirement can remove route visibility before the receipt releases native storage; ledger ownership remains authoritative.

NODE-155: DisposalPreparation owns either a short prepared transaction or unchanged reserved capacity. Busy deferral retains no graph snapshot or exclusive gate; later preparation observes current descendants. This is storage ownership, not an object-incarnation or native-release receipt, and does not replace the still-pending host coordination.

NODE-154: Selected safe-host PendingHandle results retain portable initializer storage, not native service ownership. Session-owned cleanup tickets and a separate native-work fence are revoked/drained at close even if descriptors remain held. Native state becomes empty while 64 descriptors plus their table still expose 65 storage leases; descriptor drop returns the remaining leases. This is not persistent Node Handle publication.
[Pending-result ownership and verification](pwright-node-handles.md#node-154-session-owned-pending-results).

NODE-139: Frame.goto retains its existing PageLifecycleHandle until publication and result projection end. The borrowed closed() future retains no registry or native route, and dropping a waiter does not keep a Page lease alive. Page retirement and registry termination signal closure; another Page or normal document progress does not.
[Page lifetime publication contract and verification](pwright-frame-navigation.md#node-139-page-lifetime-through-navigation-publication).

NODE-138: An owned Page's pending destruction wait no longer occupies the Node driver's lifecycle request slot. Its owned Context can issue ancestor disposal; sealed proof and original reservations then drive latest-state retirement. Local absence is not native proof; borrowed/default-context Page paths remain on the existing service.
[Node close-driver integration and final verification](pwright-close-cascade.md#node-138-opt-in-node-close-driver-integration).

NODE-137: Native close retirement consumes the latest runtime subtree into topology, Handle, Page-route and exact network-adapter deletion. Native world death comes from the sealed receipt, never local absence. The resulting owner delays publication/readiness/cancellation notification until caller locks are released.
[Native local retirement and post-lock notification contract](pwright-close-cascade.md#node-137-native-proven-local-retirement-and-post-session-lock-notification).

NODE-136: A waiting close intent can retain topology/runtime/publication capacity without acquiring lifecycle mutation gates. Completion consumption moves those owners while borrowing native proof; native evidence remains separately validated against the exact attachment before local retirement.
[Joined close capacity and native completion contract](pwright-close-cascade.md#node-136-joined-close-capacity-and-consuming-native-completions).

NODE-135: Selected local publication and runtime retirement now share one state mutex, so transport close cannot invalidate disposal between them. Callback errors preserve runtime objects but cannot undo caller-owned external mutations; successful cancellation notifications remain deferred until all outer locks are released.
[Runtime-locked publication/disposal contract and evidence](pwright-close-cascade.md#node-135-runtime-locked-local-publication-and-disposal).

NODE-134: Selected control-outbox backing belongs to RuntimeStore, including after close-pool/dispatcher drop when a lifecycle handle survives. Existing messages and EventItem leases retain FIFO identity; storage admission outlives transport close and queue clear because their buffers remain allocated.
[Preallocated runtime outbox contract and evidence](pwright-close-cascade.md#node-134-preallocated-runtime-control-outbox-storage).

NODE-133: Paired and utility-only native-world receipts retain exact owning-Page target identity after route removal. Actual Page destruction can retire those worlds, including OOPIF worlds; generic detach, route absence or destruction of an OOPIF target alone cannot stand in for whole-Page destruction.
[Native Page world-retirement contract and evidence](pwright-close-cascade.md#node-133-retained-worlds-follow-exact-native-page-destruction).

NODE-132: Native close evidence now enters local topology mutation only through an exact attachment check and validation under the same topology lock. Parent-first retirement may leave an absent child route with valid native ancestry, but portable/native rebindings reject. No runtime/public event completion is implied.
[Native close commit-scope contract and evidence](pwright-close-cascade.md#node-132-attachment-bound-native-close-commit-scope).

NODE-131: Deferred runtime disposal installs cancellation before returning its retired graph but invokes no user waker until explicit notification. Child-first records are exposed only by borrow; their owner retains the reservation slot and byte lease even after runtime/pool owners are dropped. Existing caller cancellation and transferred call ownership are preserved.
[Deferred runtime close commit contract and evidence](pwright-close-cascade.md#node-131-deferred-runtime-disposal-notifications).

NODE-130: Local close route commit preserves all surviving provider Arcs, native-world receipt registry and unrelated navigation snapshots. Removed navigation snapshots are revoked only on commit; dropping preparation leaves them live. Route removal is not world destruction or a runtime/object/event receipt.
[Deletion-only close route contract and evidence](pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-129: The scheduler retains each native command through reply/transport termination. Context proof is shared only with exact admitted descendants and keeps its original capacity until all child joins release it. Returned completions retain admission and expose evidence by borrow; consuming the payload drops the proof.
[Bounded native scheduler contract and evidence](pwright-close-cascade.md#node-129-bounded-native-close-scheduler).

NODE-128: Sealed Page bindings capture exact owned Context ancestry, source domain, epoch, target/session IDs and admission generation. Native Context receipts cannot certify another domain/ancestor, rebound IDs/routes or later bindings. Borrowed certification is native evidence, not a mutation guard or public close result.
[Native ancestor authority contract and evidence](pwright-close-cascade.md#node-128-exact-native-context-ancestor-authority).

NODE-127: An acknowledged native Page-close owner is minted only after successful native reply and binds the exact installed source and target. It retains receiver/permit/storage, not an ancestor-success proof. A coordinator must independently validate Context ancestry/epoch before replacing the destruction wait, and must retain any still-written command.
[Native close phase contract and evidence](pwright-close-cascade.md#node-127-native-command-and-destruction-wait-phases).

NODE-126: Deferred publication holds bounded capacity and metadata ownership, not a frozen collection revision or native success proof. Preparation validates its admitted target and uses current visibility; exact native incarnation/ancestor authority remains the coordinator's responsibility.
[Deferred publication contract and evidence](pwright-close-cascade.md#node-126-deferred-close-publication-capacity).

NODE-125: Runtime capacity reservations retain no stale subtree or strong runtime reference while waiting. Preparation validates the exact runtime and uses the latest subtree, so either parent/child commit order avoids resurrection/duplicate disposal. Existing call lifetime ownership and typed cancellation are preserved. Storage admission is not incarnation/native-retirement proof: the coordinator still validates native identity and chooses public close outcomes.
[Runtime reservation contract and evidence](pwright-close-cascade.md#node-125-opt-in-runtime-disposal-and-outbox-reservations).

NODE-124: Topology reservations bind exact owner identity and capture connection epoch without retaining an old object snapshot. Latest-state preparation preserves later siblings and includes later children in parent retirement; foreign owners, epoch changes and competing exclusive transactions are rejected. Native ancestor identity and already-retired child outcome remain composition responsibilities, not inferred from storage reservation.
[Topology reservation contract and evidence](pwright-close-cascade.md#node-124-opt-in-topology-transition-reservations).

NODE-123: In-flight Context-close ownership distinguishes interruption from retired-object idempotency and waits for terminal staging after releasing its operation token. Lost driver producers/queues seal admission immediately, while actual closed-object replies and queue-full backpressure do not stop live connections. The remaining ancestor-close design must separate native wait ownership from exclusive retirement commits; merely spawning another task collides with both transaction gates.
[Close cascade contracts and evidence](pwright-close-cascade.md).

NODE-122: The admitted native Page-close owner retains its exact target, event cursor, cancellation source, transaction/observer reservations and leases through confirmation. Abandoned work transfers to cleanup. Host stop must release the public operation token before waiting for terminal publication; otherwise teardown would wait on that same operation. This terminal fence preserves the original Page emitter through close dispatch.
[Selected confirmation](pwright-page-close.md#node-122-selected-native-destruction-confirmation).

NODE-119: Native Page entries optionally own a pre-admitted creation-acknowledgement token. Ready and later popup groups share it without retaining a Host or transport. Terminal/native retirement drops the index owner, while pending groups retain only charged metadata. Public membership still comes from Rust's exact collection view, not JS caches.
[Native ready-source contract and evidence](pwright-node-lifecycle-events.md#node-119-native-ready-and-popup-source).

NODE-118: A ready conversion owns only temporary Node handles plus charged metadata bindings. Prepared wrappers are absent from browser/context identity caches until the native commit succeeds. Popup emission reuses its announced Page and existing opener; unacquired emitters are not synthesized. Factories capture no Browser beyond the scoped preparation.
[Creation-conversion contract and evidence](pwright-node-lifecycle-events.md#node-118-precommit-creation-wrapper-conversion).

NODE-117: External disposal is derived from native committed effects, not JS collection differences. Compact Context disposal expands its visible Page subtree; existing Page/Context wrappers receive notices after Rust state commits. Source storage is admitted before public object visibility and released on retirement.
[External-close source contract and evidence](pwright-node-lifecycle-events.md#node-117-pre-admitted-external-close-source).

NODE-116: The native close receipt retains metadata-only FrameTrees and immutable planned views until commit or abandonment; it retains no new runtime/transport owner. Page flags move from per-Page raw permits to independent publication groups. Context disposal is a parent-owned subtree cause, not a required list of child Dispose effects.
[Explicit-close source contract and evidence](pwright-node-lifecycle-events.md#node-116-explicit-close-source-integration).

NODE-115: Session, Locator and deferred evaluation explicitly share a non-Send publication port. Its callback captures only a WeakRef to Browser; its Rust host shutdown capability is Weak, not an owning runtime reference. Termination drops callback references before environment-native cleanup joins; metadata storage lasts with the final retained port.
[Terminal dispatcher contract and evidence](pwright-node-lifecycle-events.md#node-115-environment-thread-terminal-dispatch).

# Pwright browser object-model ownership

NODE-114: Terminal capacity is held by the native collection owner before initialization. Teardown transfers used group bodies into the existing metadata-only queue and releases unused reservations. Retained collection/tree descriptors do not keep the host or transport alive.
[Terminal source contract and evidence](pwright-node-lifecycle-events.md#node-114-pre-admitted-native-terminal-source).

NODE-113: Group descriptors retain charged collection/tree metadata only. Page close commits membership and its matching tree's public closed flag before delivery. Old membership views remain immutable; foreign revision domains and mismatched close tokens are rejected.
[Group contract and evidence](pwright-node-lifecycle-events.md#node-113-bounded-dependent-publication-groups).

NODE-112: An admitted close reservation transfers with the same continued native future.
Commit consumes it once after ownership retirement; drop releases abandoned
storage. Production journals may retain admitted metadata, not the runtime
owners used by diagnostic tests.
[NODE-112 close receipt contract and verification](pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: The committed ownership/effects view borrows the driver transaction only during
native projection under the topology mutex. It retains no runtime or foreign
value. Later event ownership must be separately admitted and dispatched outside
native locks; observer failure cannot undo the native commit.
[NODE-111 source contract and verification](pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-110's listener exception port borrows the original Node value only during a
synchronous call, retains no callback/root/host owner afterward, and invokes Node
error handlers without a host mutex. Already published closed/collection state
is not rolled back when a disconnected listener throws.
[NODE-110 contracts, fix and verification](pwright-node-lifecycle-events.md#node-110-listener-failure-boundaries-and-scoped-exception-reporting).

NODE-109's non-cloneable drain retains the metadata publication endpoint and its
existing queue allowance, never a Host, runtime or transport. Complete releases
that endpoint; Drop releases only the claim and leaves pending changes intact.
Discard revokes authority. Retained collection views remain independent.
[NODE-109 contract and verification](pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-107 revisions retain only the publication domain's existing metadata ledger,
not the queue/runtime. Cross-revision rebasing checks that domain and overlapping
metadata identities; old views remain immutable and charged. Tests verify queue
liveness ends before retained revisions and final leases can still be reclaimed.
[NODE-107 contracts and evidence](pwright-node-lifecycle-events.md#node-107-validated-cross-revision-visibility-rebasing).

NODE-106 CollectionView clones an admitted immutable revision Arc plus value masks;
older views keep their membership and descriptors. Hiding does not close a Page or
release its metadata lease. Final-owner release reclaims hidden descriptors without
retaining the Host, runtime or browser. Borrowed iterators add no heap owner.
Cross-revision event payload retention and callback wrapper identity remain open.
[Ownership tests and next boundary](pwright-node-lifecycle-events.md#node-106-immutable-single-revision-visibility-foundation).

NODE-105 requires event arguments to reuse exact Page/Context wrappers through
retirement, even if a getter prunes a facade cache. Proposed Rust delivery steps
must retain charged metadata, not Host/runtime execution ownership, and advance
state immediately before unlocked foreign dispatch. Retained views stay immutable.
This is a design gate, not an implemented event owner; final-state getter parity
does not cover these callbacks.
[Evidence and ownership constraints](pwright-node-lifecycle-events.md).

NODE-103 switches Node collection getters to immutable published Rust membership.
Retained Context settings and Page/main-Frame/tree metadata survive native index
retirement without retaining the host or transport. The producer keeps a separate
last-staged signature and admitted retirement pool, released on clear. Fresh arrays
and cached wrapper identity remain the language facade's responsibility.
Default/combined collection tests pass 7/7; NODE-102 cutover notes below are historical.
[Current ownership and evidence](pwright-frame-publication.md#node-103-rust-collection-producers-and-published-node-reads).

NODE-102 implements immutable Rust collection membership revisions retaining
ContextSnapshot, main FrameNode and FrameTree metadata. A revision has no strong
path to its publication book, HostSession, runtime or transport. Old readers keep
their own count/byte admission through native retirement and later replacement.
The native producer and Node getter cutover is not implemented yet.
[Ownership proof and remaining work](pwright-frame-publication.md#node-102-bounded-rust-collection-revision-storage).

NODE-100 keeps Browser.isConnected and one-shot disconnect suppression in Rust.
Native cancellation does not mutate the published getter. Terminal Page fences
authorize, but do not dispatch, Browser disconnect; the language event port
transitions immediately before emission. Borrowed native Pages survive.
[Ownership and timing](pwright-browser-close.md#node-100-rust-owned-browser-disconnect-publication).

NODE-099 distinguishes the connection-local Page's published closed flag from
native target existence and Frame detach. Borrowed disconnect marks the retained
Page closed while the external native Page survives. Close tokens retain only
budgeted tree metadata until delivery/discard; the active index transfers its
pre-admitted permit rather than allocating a new shutdown owner. Shared queue
storage stays charged through the final publication endpoint, without retaining
the runtime, HostSession, browser or execution worlds.
[Ownership and tests](pwright-page-close.md#node-099-rust-owned-page-close-publication).

NODE-095 lets an exact native detach retire matching pending call ownership.
It does not dispose a borrowed Context/Page, infer parent ownership, or retain a
permanent session tombstone. Future calls still require native validation.
[Contract and evidence](pwright-chromium-topology-events.md#node-095-native-detach-before-an-initialization-response).

NODE-094 does not infer ownership from a native Context-list snapshot. Selected
Playwright mode attributes all unregistered Context Pages to its default view;
registered ownership wins, and borrowed native Contexts survive disconnect.
[Contract](pwright-chromium-topology-events.md#node-094-queued-attachments-from-disposed-contexts).

NODE-092 distinguishes releasing this connection's child inspector session from
terminating the browser Worker or its owning Page. A live-worker echo control and
borrowed-Page checks accompany the native closed-child race. No portable Worker
object API is added. [Ownership](pwright-chromium-topology-events.md#node-092-child-disappears-between-resume-and-detach).

NODE-091 header watches belong to requests, not Node Frame wrappers or an
unbounded session registry. Primary/compensation share one route watch with two
waker slots. Native closure retires pending child work; live errors retain normal
failure/compensation semantics. [Ownership](pwright-extra-http-headers.md#node-091-native-session-closure-during-header-fanout).

NODE-090 distinguishes an unpublished reserved Page ID from a published owner.
A vanished native session can abandon the former without sealing other Pages;
its escaped lifecycle handles retain storage but lose readiness. A replacement
attempt must not retire a live owner sharing its reserved Page identity.
[Ownership](pwright-chromium-topology-events.md#node-090-missing-session-retirement-before-page-publication).

NODE-089 shares one snapshot cell across a Page's native sessions. The first
Page is captured at preparation; later Pages read desired settings at their
sequential dispatch boundary. Failed groups retain started siblings but never
start untouched later Pages. Request ownership still lasts through compensation.
[Native receipt proof and limits](pwright-extra-http-headers.md#node-089-native-receipt-gates-and-context-page-sequencing).

NODE-088 keeps each header request permit and frozen snapshots alive after public
failure until primary siblings and compensation drain. Bounded jobs belong to the
existing driver. Shutdown drops pending work with native uncertainty, not an
invented compensation acknowledgement; exact native/waker retirement remains open.
[Owner lifecycle](pwright-extra-http-headers.md#node-088-caller-polled-native-jobs-and-early-primary-rejection).

NODE-087 prepared setters own a pending update and frozen per-Page native values;
all host metadata reads share a synchronous preparation lock, never an async
native lock. Queue rejection is pre-publication. Abandonment after publication
records native uncertainty because an initializer may already have observed it.
Prepared navigation owns its policy/permit before its future is polled.
[Owner lifecycle](pwright-extra-http-headers.md#node-087-synchronous-rust-preparation-and-frozen-dispatch).

NODE-086 optional overlapping writers each retain their own prior admitted header
snapshot. Failures restore their captured values; acknowledgements never republish
old proposals or repeat restoration. Pending records retain empty target capacity;
retirement prevents late handles from resurrecting objects. The host has not yet
selected this concurrent profile.
[Model and scheduling boundary](pwright-extra-http-headers.md#node-086-pending-setter-race-and-bounded-overlapping-model).

NODE-085 navigation preparation borrows original Page header state, never Context
merged values for conflict checks. An admitted guard owns any implicit referrer
copy through command completion; no JS handles or header snapshots are retained.
It uses separate navigation slots, not setter/creation queue capacity.
[Ownership and bounds](pwright-extra-http-headers.md#node-085-navigation-referrer-reconciliation).

NODE-084 prepared creation headers own only admitted Rust strings and the selected
installation permit. Desired settings become Context-owned before its descriptor
is published; post-native install failure closes that exact Context. Reconciled
retirement reclaims headers; Node never performs a create-then-set transaction.
[Contract and public evidence](pwright-extra-http-headers.md#node-084-context-creation-headers).

NODE-083 retains bounded original header strings even when their HTTP syntax is
invalid; only final effective values are validated for native installation.
A failed explicit Page creation owns a transient Page, closes that exact Page
before rejection, and preserves Context/Browser ownership. Typed failure records
are keyed by portable Page identity and removed on reconciled retirement.
[Lifecycle evidence](pwright-extra-http-headers.md#node-083-deferred-syntax-and-page-local-initialization-failure).

NODE-082 adds public header mutation to the owning Page/Context wrappers.
Scoped Node handles are collected and converted before the Rust future starts;
only admitted owned strings, target identity and the host Arc cross the async
boundary. Public test Contexts are API-owned, never the fallback shared Context;
close checks host and FFI ownership counters return to zero.
[Phase and ownership contract](pwright-extra-http-headers.md#node-082-public-selected-setters-and-language-phase-differential).

NODE-081 validates header setter object kind against current topology and prunes
desired settings through reconciled Context/Page predicates. Header native-side
ownership is distinct: an old CDP session can outlive its Page route, so only
acknowledged header clearing or exact missing-session evidence releases its
applied-state record. No other capability's native session is detached.
[Ownership regression and cleanup](pwright-extra-http-headers.md#node-081-selected-rust-host-and-native-session-reconciliation).

NODE-080's header registry checks a stored Page's owning Context and tracks
pending generations. Native retirement supersedes late completion; metadata
shutdown does not revoke separately retained original Page snapshots. No native
identity or lifetime validation is inferred from the portable key itself.
[Ownership tests](pwright-extra-http-headers.md#node-080-bounded-desired-settings-transactions).

NODE-079 exposes borrowed native routing plus reserved/committed portable
Page/Context ownership to session initializers. OOPIFs inherit their Page;
reserved IDs are not readiness or lifetime capabilities. Retained settings and
native transactions require their own owners, not an initializer's borrowed view.
[Boundary](pwright-extra-http-headers.md#node-079-identity-aware-native-initialization).

NODE-077 verifies actual Node Worker cleanup of a pending Frame operation and
a typed result queued for foreign delivery. Owned native Contexts/targets are
removed; a borrowed Page remains live and finishes its held response after the
Worker exits. This is exact-identity evidence through a diagnostic observer,
not ownership inferred from aggregate Page counts or post-Worker RSS.
[Boundary and limitations](pwright-frame-navigation.md#node-077-causal-close-timeout-continuation-and-worker-teardown).

NODE-076 public Frame.goto retains the caller's exact Frame wrapper in its
Response; Page.goto/reload still default to mainFrame. Foreign delivery copies
metadata while the Rust result and FFI ticket remain admitted, then releases
the Rust result. A JS Response does not retain a native response arena or Rust
result owner. The optional feature does not change Page/Frame cache identity.
[Conversion and selected public evidence](pwright-frame-navigation.md#node-076-public-node-frame-navigation).

NODE-075 returns the portable Frame ID in a charged typed host result. It drops
the native response arena after bounded Rust projection; a separate result
permit outlives host close without retaining HostSession/Page handles. Real
Chrome verifies escaped-result readability and zero leases after the final
result drop. Node Response.frame wrapper selection remains unimplemented.
[Owner boundary](pwright-frame-navigation.md#node-075-typed-rust-host-result-ownership).

NODE-074 distinguishes portable Frame response identity from native CDP IDs:
the selected FrameGoto result carries nonzero `frame_id: u64`, while retained
Page navigation results retain their old string fields. Frame-only command
targets and duplicated envelope IDs are validated by the selected typed path.
This enables, but does not yet implement, returning the correct Node Frame
wrapper from a navigation Response. No new object owner is installed by the
schema/codec feature itself.
[Contract](pwright-frame-navigation.md#node-074-opt-in-typed-navigation-12-contract).

NODE-073 retains an admitted executor slot, selected native route and Page
registration during host navigation. Dropping the invocation releases them;
the moved result retains only its separately charged response/error arena.
Route replacement after dispatch does not itself invalidate the native receipt:
the portable Frame's ordered document/lifetime observation decides completion.
[Ownership contract](pwright-frame-navigation.md#node-073-rust-host-dispatch-scheduling-and-cancellation).

NODE-072 tickets retain one Page/terminal owner and admitted registration,
not host/transport. Their fixed index slot is reclaimed after ticket drop at
a Page checkpoint. Moving out a completed navigation preserves its separate
response/error lease while releasing Page ownership when the ticket drops.
Page-close wakes its own pending consumers; global seal rejects all handles.
[Ownership evidence](pwright-frame-navigation.md#node-072-ordered-operation-registration-and-actual-rust-host-observation).

NODE-071 introduces a non-cloneable per-invocation navigation state owner. It
holds only bounded events, document-request metadata and serialized response
bytes, not the Page/host/transport. Its completion names the original Frame and
borrows response storage; external runtime lifetime and escaped output admission
remain separate. [Contract](pwright-frame-navigation.md#node-071-bounded-navigation-matching-and-response-retention).

NODE-070 adds a bounded registry of independent Page lifecycle owners. Escaped
handles retain their Page and terminal signal, not the host, transport or other
Pages. Registry shutdown seals every handle; normal Page retirement seals only
that Page. Closed slots are reclaimed on a later checkpoint only after escaped
handles drop. [Contract](pwright-frame-navigation.md#node-070-bounded-multi-page-registry-and-opt-in-rust-host-selection).

NODE-069 introduces an explicit Page/epoch lifecycle owner, sharing no ownership
with other Pages by default. The driver borrows it through an Arc; retirement,
loss or driver drop seals reads while escaped owner handles retain their leases.
Borrowed notices do not extend native event payload lifetimes.
[Contract](pwright-frame-navigation.md#node-069-opt-in-production-ordered-lifecycle-observation).

NODE-068 publishes old descendant Frame disposal on current-session document
commit while preserving the selected Frame. A real cross-origin test verifies
that portable Frame identity survives a renderer change while the old retained
navigation route becomes permanently stale. The full navigation waiter must
follow Frame/document identity, not equate route retirement with Frame closure.
[Production transaction and evidence](pwright-frame-navigation.md#node-068-production-document-commit-topology-retirement).

NODE-067 owns bounded native loader/request/session identities alongside the
portable lifecycle reducer. Its borrowed routing port leaves topology ownership
with the embedding Rust driver; worker and migrated-document ownership require
explicit routing proof. Response/error notices borrow the admitted envelope and
cannot substitute for retained navigation-result ownership.
[Contract and verification](pwright-frame-navigation.md#node-067-bounded-native-lifecycle-normalization).

NODE-066 retained navigation snapshots own an exact epoch/Frame/Page/renderer
route, independently of execution worlds. They cannot export an unbudgeted client
Arc. Transaction commit, not speculative preparation, retires replaced routes;
swap-away/back never revives old snapshots. Seal/last-table-drop are terminal,
but observed revocation is not native destruction or an atomic browser lifetime.
[Detailed contract](pwright-frame-navigation.md#node-066-retained-routes-in-the-production-topology-transaction).

NODE-065's optional lifecycle reducer stores only normalized portable identities,
document generations and fixed-size state. It owns no worlds, native clients,
tasks or JS roots. Its store lease is separate from the future retained invocation
and native-normalization owners; copy-out readiness is not a navigation response.
[Ownership and pending integration](pwright-frame-navigation.md#node-065-optional-normalized-lifecycle-reducer).

NODE-064 Frame navigation routes borrow Frame/Page/renderer identity and epoch
from one immutable topology view, independent of main/utility worlds. This is
not an async dispatch lease; exact retention, revalidation and cancellation
ownership remain required before exposing Frame.goto. A remote rebind preserves
the remote Frame while retiring its old child-document routes.
[Ownership and implementation gates](pwright-frame-navigation.md#2-exact-native-route-and-dispatch-lifetime).

NODE-063 page.frame retains one pinned metadata read for selection, with no live
runtime requirement. Scoped Node values do not cross an await or become native
references. The cache pins existing wrappers during callbacks and binds only
missing selected ancestry; retained closed/disconnected Pages remain readable.
Last-wrapper GC returns all host resources to zero and the tree to its fixed index.
[Ownership and evidence](pwright-frame-lookup.md#selected-path-ownership-and-admission).

NODE-062 Context/Page wrappers retain immutable Rust baseURL snapshots without
registry, HostSession or transport ownership. The live race gate now checks every
newly enumerated wrapper against creation identity. Retained settings work after
Context close and browser disconnect; last GC returns the Context ledger to its
560-byte fixed index and all live host resources to zero. Public Frame lookup
is the next integration gate.
[Ownership and evidence](pwright-context-creation.md#verification-status-and-remaining-gates).

NODE-060 lends the existing tree factory to synchronous native glob conversion.
Foreign URL handles remain callback-scoped; no constructor reference, HostSession,
transport or runtime is captured. The primitive works on a retained closed tree;
its last-wrapper test returns native metadata to the fixed host-index baseline.
[Ownership and verification](pwright-frame-lookup.md#node-060-actual-n-api-url-value-conversion-port).

NODE-059 keeps glob preparation, restoration scratch and compiled output under
separate scoped Frame-ledger leases. URL-resolution data is borrowed; the final
plan retains no Context, parser, transport or runtime. Context baseURL ownership
and public lookup wrappers are not yet integrated.
[Ownership and storage contract](pwright-frame-lookup.md#node-059-bounded-glob-preprocessing-and-borrowed-url-resolution).

NODE-058 lookup retains an admitted published Frame collection, not the Page
tree, metadata book, runtime or transport. Matching returns a borrowed node and
does not bind every candidate. Callback errors retain their identity, and
reentrant publication cannot replace the current call's membership.
[Ownership and lifecycle tests](pwright-frame-lookup.md#node-058-rust-traversal-and-bounded-resolved-glob-compiler).

NODE-056 stores navigation names in the same published revision as URLs.
Names survive detach/close with the Frame and retain only metadata leases.
Same-document updates preserve names; direct window/attribute mutation does
not update the public cache. No host/transport/FFI owner is retained by a name.
[Ownership and evidence](pwright-frame-publication.md#node-056-bounded-navigation-names-and-public-framename).

NODE-055 adds an admitted native Page-tree wrapper and one-snapshot cached-ID
enumeration. Existing JS Page objects remain stable, retired cache entries are
pruned, and first-seen popup descriptors acquire their own metadata leases.
All four new-Page states are reserved before creation; native wrapper owners
retain no host/transport/FFI registry. Public enumeration and finalization pass.
[Ownership and evidence](pwright-frame-publication.md#node-055-admitted-page-descriptors-and-cache-aware-enumeration).

NODE-054 makes a newly created Page descriptor move-only so its pre-admitted
main-Frame binding transfers once from Rust to N-API. JS uses that binding
directly; root-wrapper exhaustion can no longer reject after native creation.
Existing-Page enumeration still acquires published-node metadata on demand.
[Lifecycle/resource proof](pwright-frame-publication.md#node-054-creation-time-root-binding-admission).

NODE-053 exposes public Frame collections with stable main/child identity.
JS caches only weak child references; retained children keep their original
parent and Page. Rust owns published membership, URL and detach metadata with
last-native-wrapper leases. A real paired 10-case local/OOPIF gate validates
subtree removal, replacement and close; event timing remains a separate gate.
[Ownership and evidence](pwright-frame-publication.md#node-053-public-node-frame-collections-and-retained-wrappers).

NODE-052 now projects authoritative Page/Frame/parent identities into retained
host metadata, including world gaps and direct Page-service closure. Native
local/OOPIF identity and detach/close observations match the pinned oracle.
N-API enumeration and JS wrapper identity remain a separate pending gate.
[Projection contract](pwright-frame-publication.md#node-052-live-host-projection-and-paused-oopif-attachment).

NODE-051 separates retained Frame metadata from runtime/transport ownership.
Immutable parent IDs and URL views survive detach; Page close alone does not
mark Frames detached. Bounded tree revisions share the URL publication fence,
but live host projection and public Node enumeration are still pending.
[Ownership, oracle observations and next gates](pwright-frame-publication.md).

> Status: the base object vocabulary, schema command targets, client marker
> migration, runtime target admission, parent-kind topology policies, and
> server handshake preflight are implemented. The v2 public/wire ABI is not
> frozen.

## Decision

Remote object identity has two deliberately separate parts:

- `ObjectKindId` is the stable semantic name used by schemas, capability
  contracts, compatibility checks, and diagnostics;
- `ObjectKindTag` is the compact numeric value carried by lifecycle frames and
  stored in the runtime object graph.

`pwright-core` defines only generic descriptors, catalogs, and marker traits.
The tiny `pwright-browser-model` crate is the single owner of Playwright-shaped
browser object IDs and their wire tags. It depends only on `pwright-core` and
contains no browser process, provider, backend, codec, transport, runtime,
executor, Node.js, or download behavior.

This vocabulary is baseline semantic identity, not a negotiated `browser`
capability. Installing it cannot launch, bundle, or download a browser.

## Base vocabulary

| Rust marker | Semantic ID | Wire tag | Complete-graph parent |
| --- | --- | ---: | --- |
| `BrowserKind` | `pwright.browser.browser` | 1 | root |
| `BrowserContextKind` | `pwright.browser.browser_context` | 2 | Browser |
| `PageKind` | `pwright.browser.page` | 3 | BrowserContext |
| `FrameKind` | `pwright.browser.frame` | 4 | Page or Frame |

The client reexports these marker names for compatibility, but no longer owns
or repeats their numbers. Additional identity-bearing Playwright objects such
as Worker, Request, Response, Route, Download, and Artifact must be added to
the same catalog only when their lifecycle is designed.

## Command targets

The production paired-world access port accepts an exact Frame ObjectId, not
a Page ID with an implicit main-Frame substitution. It returns a privately
retained routing snapshot, not a new runtime object kind or portable Handle.
Both execution worlds must belong to that Frame's current renderer session.
World observations surviving a renderer handoff stay in the native inventory
but cannot remain current command routes. The original service and route
leases outlive accessor drop while any budgeted snapshot remains. See
[paired-world ownership](pwright-locator-evaluation.md#production-paired-world-access-2026-09-07).

The new preparation composition retains that paired reference through the
temporary selected native owner and its cleanup continuations. Preparing an
element does not publish a portable Handle or create a Node wrapper identity.
Only pre-user-code selection/adoption can retry on observed replacement, and
the returned owner still refers to the exact selected node if it later detaches.
Route retirement remains distinct from physical native group reclamation.

Every remotely advertised command declares one or more canonical semantic
target IDs in its capability schema. The generator emits a static
`TARGET_OBJECT_KINDS` slice on `CommandSpec`; target changes are part of exact
minor compatibility. Navigation currently targets Page. Locator click/fill
target Page or Frame. Capability surfaces also publish ordered static command
descriptors that combine this target slice with the stable command ID and
method tag.

An in-process-only `CommandSpec` may leave the target slice empty. A
`CapabilityCommandSpec` entering the remote server inventory may not: this
prevents an advertised command from silently accepting every object kind.

## Runtime admission

The direct and controlled dispatchers accept an optional validated static
`ObjectKindCatalog`. With a catalog installed:

1. lifecycle registration rejects unknown numeric tags and invalid declared
   parent kinds;
2. a typed command resolves the live target's numeric tag to its semantic ID;
3. the runtime checks that ID against the command's generated target slice;
4. mismatch fails as `ProtocolViolation` before scheduler admission or backend
   invocation.

If a targeted command reaches a runtime without a descriptor for the live tag,
it fails explicitly as `UnsupportedCapability`; the runtime never guesses from
the numeric value. Generic in-process commands with an empty target declaration
retain the compatibility path and do not require a catalog.

`ObjectTopologyMode` makes graph completeness explicit. `Partial` is the
default for an externally attached subset: a Page may be represented as the
local root, while every parent that is present must still have a permitted
kind. `Complete` permits parentless registration only for kinds declared as
roots, so BrowserContext, Page, and Frame require their canonical chain.
Create and Adopt use the same topology check before mutating the graph.

Before advertising a composed handler chain, a server can build its handshake
configuration with the installed object catalog. Inventory registration then
proves that every generated command target resolves in that catalog; an
unavailable target blocks the manifest before negotiation or command traffic.
The catalog itself stays server-local because clients negotiate capabilities,
not a browser/backend installation plan.

## Remaining object-model work

- generate lifecycle/object reference documentation from the shared schema;
- define identity and retention rules for Request, Response, Route, Worker,
  Download, and Artifact;
- add disposal requests and backend cleanup for every identity-bearing object.

The first explicit request is now the separate
`pwright.context_lifecycle` `browser-context.close` command, not a generic
client-authored Dispose. It targets only `BrowserContext`. Version 1.0 closes
only a context whose backend ownership is `Owned`; default, persistent, and
externally attached contexts reject the request so a context handle cannot
silently terminate a caller-owned browser. The operation has no ordinary
action timeout and is idempotent from the client's perspective. Its backend
close must succeed before the prepared runtime subtree is committed; backend
failure drops the preparation and leaves local objects live. A `reason` field
is intentionally deferred until it can propagate to interrupted-operation
errors end to end.

The experimental Node compatibility host now uses that owned-Context service
for `BrowserContext.close()`. For a borrowed default Context it instead selects
its own attachment-disconnect path, matching Playwright CDP and preserving the
external Chrome process. This does not weaken the reusable capability's
owned-only rule. Context creation/disposal, native inventory cleanup on Worker
exit, and their shared serial topology driver are documented in
[the Node Context checkpoint](../node-ffi-compatibility-design.md#owned-context-lifecycle-and-initialization-checkpoint).

The Chromium path implements this contract with paired prepared topology/runtime
transactions and `Target.disposeBrowserContext`; a real dedicated-Chrome proof
confirms the browser itself survives. In remote mode, the close service and
ordinary control lane share a bounded priority writer. It sends the complete
child-first Dispose sequence before Result, and the unique client reader
synchronously invalidates the context before waking `close().await`. The
attached composition owns that shared topology directly, admits one production
runtime, and constructs the close service from its exact CDP connection. The
same factory requires a caller-owned bounded continuation executor. It reserves
before CDP, preserves laziness when never polled, and transfers the exact
remaining transaction when a started close future is abandoned; no hidden task
or executor is selected by the capability.

Page close is now a separate optional lifecycle capability precisely because
`runBeforeUnload` changes its completion contract. Ordinary close proves native
target absence, then retires Frame/Page topology, Page-session routes, and
runtime identity before Result. Before-unload merely initiates page-session
`Page.close` and preserves those owners. Browser close remains separate because
it depends on provider/process ownership and shutdown escalation. None of these
operations is modeled by making the runtime Dispose control message callable
by users.

## Experimental Node Locator ownership

The Node facade now binds Page/main-Frame Locators to immutable Rust selector
values. Creation, `nth`, `and`, `or`, descendant chaining and filters do not
resolve an element. Each read re-resolves through the existing typed Locator
capability, preserving strict single-element reads and non-strict collections.
An empty selector is a deferred typed issue, rejected on use rather than
replaced by a fake selector. Retained Locators survive DOM replacement and
navigation without retaining a browser DOM handle.

Rust owns a weak host identity plus portable Frame ID and checks both on
composition; identical native targets reached through different connections
are not interchangeable. Node wrappers preserve `locator.page()` identity.
Browser close ends browser work but does not invalidate pure query construction;
later operations reject. The environment separately limits query values and
command-owned copies to 1,024 entries / 8 MiB, exposes their metrics, and
releases them on native finalization. Active-session counters are not those
retained-value counters. See the [Node Locator contract](../node-ffi-compatibility-design.md#vs3-locator-binding-contract-in-progress)
for the supported subset and remaining action/concurrency gates.
