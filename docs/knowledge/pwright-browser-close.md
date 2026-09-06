<!-- agent-updated: 2026-09-04T12:19:06Z -->

# Playwright-aligned Browser close and disconnect design

> Status: the owned launch/composition/shutdown boundary, portable remote
> `Browser.close`, bounded close-reason propagation, disconnected-event
> contract, ordered composition wiring, and exact-session authorization are
> implemented.

## Upstream behavior

The source baseline is Playwright commit
`de214f440b7e34937fe4886f046b78b757136087`.

Playwright presents one public `browser.close({ reason })` method, but its
effect depends on how the Browser was acquired:

- a normally launched Browser calls the server Browser's process `close`,
  which attempts graceful browser shutdown, waits for disconnection, and may
  force-kill the owned process after a timeout;
- Chromium `connectOverCDP` installs a synthetic browser-process adapter whose
  `close` only closes and waits for that CDP transport, so it does not terminate
  the externally owned Chrome process;
- Playwright's remote `connect` client marks the Browser so local close closes
  the client connection. The server-side pre-launched Browser dispatcher has
  `ignoreStopAndKill`, preventing that client from terminating the shared
  browser;
- browser disconnection closes descendant contexts/pages and emits the public
  disconnected event whether shutdown or transport loss caused it.

The detailed terminal order is also observable. The client stores the supplied
close reason before sending `close`, then awaits its disconnected promise. On
the server, the first non-empty reason becomes the Browser's terminal reason.
`Browser.didClose()` closes Page descendants and Contexts before emitting the
Browser disconnected event. Dispatchers publish each close event before
disposing that channel object, ending with Browser close followed by Browser
dispose. The reason is diagnostic context inherited by descendant operations;
it is not an argument to the native Chromium shutdown provider.

Relevant anchors are:

- `packages/playwright-core/src/client/browser.ts` (`Browser.close`,
  `_shouldCloseConnectionOnClose`, `_didClose`);
- `packages/playwright-core/src/client/connect.ts` (remote connection close);
- `packages/playwright-core/src/server/browser.ts` (`Browser._close`,
  `didClose`);
- `packages/playwright-core/src/server/browserType.ts` (`browserProcess`,
  graceful-close/kill escalation);
- `packages/playwright-core/src/server/chromium/chromium.ts`
  (`_connectOverCDPImpl` synthetic process adapter);
- `packages/playwright-core/src/server/dispatchers/browserDispatcher.ts`
  (`ignoreStopAndKill`);
- `packages/protocol/spec/browser.yml` (`Browser.close`).

## Pwright decision

The public outcome should remain Playwright-shaped—after `close().await`, that
Browser handle and every descendant are stale—but process authority must remain
structural rather than inferred from a flag.

Pwright therefore separates two mechanisms:

1. **Disconnect an external attachment.** This is connection lifecycle, not a
   browser capability and never sends CDP `Browser.close`. It closes the
   Pwright/CDP connection and invalidates its complete portable object tree.
   Existing graceful connection-control machinery owns remote Close/Ack and
   transport shutdown.
2. **Close an owned launched browser.** This is an optional Browser lifecycle
   capability installed only by a launched-browser composition that has moved
   the non-cloneable `BrowserProcessLease` into one shutdown owner. It consumes
   that authority exactly once and uses the provider's bounded graceful
   close/process-exit/force-kill coordinator. An attach-only composition cannot
   construct or advertise this command.

A later ergonomic facade may expose both as `Browser::close`, selecting the
mechanism from the acquisition type, as Playwright does. The lower layers must
not erase the distinction. There is no generic CDP backend feature called
`browser.close`, because process ownership and escalation belong to the
provider/composition boundary, not to Chromium command mapping.

## Local and remote completion

For an owned launched browser whose Pwright protocol transport is independent
of CDP, the required sequence is:

```text
reserve caller-owned continuation
  -> validate the exact Browser target
  -> prepare complete Browser runtime disposal
  -> claim the session-authorized unique process lease
  -> provider shutdown (Browser.close, observe exit, bounded force escalation)
  -> commit complete Browser runtime disposal
  -> publish descendant Dispose controls through Browser Dispose
  -> publish Result
```

Process exit—not CDP acknowledgement—is the native completion proof. Once the
lease has been claimed, cancellation or future Drop cannot put it back; the
exact shutdown future must transfer to a previously reserved owner. A failure
that leaves the provider kill-on-drop guard active is terminal cleanup state,
not a retry using duplicate authority.

For external attachment disconnect, there is no command Result on the closing
connection. Transport closure is authoritative and the client's unique reader
invalidates the whole Browser tree. If a separate remote Pwright transport
controls an external CDP attachment, closing that client session may dispose
its session-owned contexts before disconnecting, but it still cannot send
CDP `Browser.close`.

Remote authorization is mandatory for owned-process close. The launch leaf
adds `pwright.browser_lifecycle` to its own handshake manifest only when the
caller selects it; the attach-only manifest remains unchanged. After the real
server handshake accepts that capability for one session, converting the owner
into a non-cloneable close grant additionally requires the accepting
supervisor and launched composition to share the same production resource
ledger identity. This is a pointer-identity proof already present at the
connection root—not a random token or global registry. A session with the same
capability, limits, plan, or numeric Browser ID but a different ledger is
foreign; a supervisor without production resources is unbound. A rejected
conversion returns the untouched owner, so none of these failures can orphan
the process. Shared/prelaunched and external-attach services cannot construct
the grant, matching Playwright's `ignoreStopAndKill` safety behavior.

## Application cleanup policy boundary

Graceful application-wide cleanup drain, retry, and reporting remain deployment
policy rather than Browser protocol behavior. The optional Tokio adapter now
implements that policy without changing `Browser.close`:

- shutdown rejects new cleanup admission but honors every reservation accepted
  before the transition and waits for reserved plus running work;
- a drain timeout reports remaining work without cancelling it, allowing a
  later completion or a deployment-selected runtime stop;
- ordinary and reserved cleanup futures are never retried automatically;
- retry requires an explicit finite factory for a fresh attempt, so it is not
  available to the consuming owned-Browser close continuation by accident;
- one capacity permit spans every retry attempt and its delay;
- a synchronous reporter observes admission, retry, job, and shutdown events;
  callback panics are counted and cannot orphan a cleanup job.

A composition test polls owned close until the unique process authority has
moved, drops the command future, starts executor shutdown, then proves the
shutdown wait includes that exact continuation and resolves only after the
Browser runtime tree is empty.

## Implemented ownership and remote-close checkpoint

The optional `pwright-composition-chromium-launch` leaf accepts an explicit
installed-browser launch request and returns an `OwnedChromiumComposition` that
retains the only process lease. Its CDP connection carries a production plan
and ledger-identity proof until the prepared composition consumes it; a foreign
plan or ledger is rejected. The ordinary attach composition cannot call owned
shutdown or construct a remote-close grant, both enforced by compile-fail
tests.

Owned shutdown borrows the owner rather than consuming it when the future is
created. Dropping an unpolled future therefore leaves process authority intact.
Once polled, it consumes the lease into the provider's bounded graceful-close,
exit-observation, and force-escalation coordinator, whose cancellation guard
prevents an admitted shutdown from orphaning the process.

Abandoned close execution now has two explicit adapters. The standard-library
`CleanupQueue` remains bounded and caller-driven. The optional
`TokioCleanupExecutor` implements the same reservation contract and can apply a
caller-selected nonzero deadline to every submitted job. Reservation capacity
cannot be stolen between command construction and irreversible shutdown. A
deadline drops a stalled shutdown, which activates the owned process
kill-on-drop guard, releases the executor permit, and increments `timed_out`;
Tokio runtime shutdown instead increments `cancelled`. Because dropping the
pending close rolls back its prepared runtime transaction, the native CDP
disconnect lane remains responsible for final descendant-first disposal.

Shutdown diagnostics are now independently pluggable without widening the
portable API. `BrowserCloseResult` remains `()` and the capability, codec,
client, and server crates retain no provider dependency. The launch composition
defaults to `DiscardOwnedBrowserShutdownReport`, but an assembly root may
select an `OwnedBrowserShutdownReportSink<Report>` through
`OwnedBrowserCloseExecution`. It receives the concrete provider report only
after runtime disposal commits. Encoded testing sends a provider-shaped report
to the sink while decoding unit success on the wire; the live Chrome proof
matches the received report's PID and user-data directory to the exact process
that was reaped.

The portable protocol is independently packaged as
`pwright-cap-browser-lifecycle`, `pwright-codec-browser-lifecycle`,
`pwright-client-browser-lifecycle`, and
`pwright-server-browser-lifecycle`. It contains one `browser.close` command
with an optional UTF-8 `reason` and typed authorization/shutdown/cleanup failures.
Timeout and cancellation metadata and wire attachments are rejected because
shutdown continuation is owned by the server rather than a client deadline.
Client close treats an already-stale Browser as idempotent success.

The reason is bounded by the selected Browser-lifecycle codec's string budget.
An empty reason is treated as absent. After native shutdown, one shared
`Arc<str>` terminal failure is installed into every still-pending operation in
the Browser subtree before its child-first disposal wakes those calls. This
avoids a string clone per descendant. Backend cleanup deliberately continues
to receive the structural `ParentClosed` cancellation reason, while callers
receive `TargetClosed` with the supplied diagnostic. Client-side persistence
uses a separate bounded root-family state: `ClientLimits` caps terminal-reason
bytes, every descendant shares one `Arc<str>`, and first non-empty reason wins.
The Rust close future installs it only when first polled and before dispatch;
oversize fails before the channel is touched. After child-first Dispose, a new
operation reports the same reason without keeping the client connection alive.
Production client recipes must set this bound no larger than the negotiated
Browser-lifecycle codec string bound.

## Disconnected-event foundation

`pwright-cap-browser-events`, `pwright-codec-browser-events`, and
`pwright-client-browser-events` define the independently selectable public
surface. The event carries no payload, matching Playwright's callback; its
route carries Browser identity and the fixed-size codec carries a monotonic
sequence. The reliable client handler uses `RejectNewest`, because replacing a
terminal state notification would make the client view invalid.

`pwright-server-browser-events` supplies a separate queue-free, task-free
publisher for the one possible disconnected event. Its first polled send owns
the attempt; failure or Drop makes publication terminal so a possibly partial
stream frame is never retried. The ordered runtime-control barrier can now send
all child-first lifecycle messages before Browser Dispose while leaving that
final Dispose claimed by nobody. The eventual composition can therefore hold
one writer permit across:

```text
Page/Frame/Context Dispose -> Browser disconnected -> Browser Dispose
```

`BrowserEventsInstallationProfile` now makes that wiring explicit. It cannot
prepare without the bounded remote handshake, ordered control, and a send
frame large enough for the fixed 17-byte payload. When accepted, the attach
composition builds one publisher from the negotiated directional protocol and
the connection's production ledger, without adding a queue. Its returned
bundle shares that publisher with an event-aware CDP liveness backend and a
connection-cleanup decorator. The owned-close handler takes another clone of
the same publisher. Thus native loss and explicit close race through one
at-most-once state instead of emitting independently.

The owned command service validates the Browser ID and prepares complete
runtime disposal before it takes the grant. An unpolled command is lazy. Once
started, dropping it transfers the exact future into capacity reserved from a
caller-owned cleanup queue; no hidden task or executor is introduced. A
terminal provider success or failure commits descendant-first disposal, and
the ordered control barrier sends Page, Context, and Browser Dispose messages
before the typed Result.

Unit and encoded-memory tests prove authority is consumed once, wrong targets
do not consume it, accepted sessions preserve exact supervisor-ledger identity,
shutdown failure still invalidates the tree, a dropped started call continues,
and the client cannot complete before all Dispose messages make its handles
stale. A real Chrome 152 test creates actual accepted unprivileged, unbound,
foreign-ledger, and exact-owner sessions. Each rejected grant returns the same
live process owner; only the exact-owner session proceeds. It then observes
Page Dispose, Context Dispose, disconnected, Browser Dispose, and Result in
order while reaping the exact launched process. Polling the shared native
liveness lane afterward coalesces the same CDP closure without a duplicate
event. The unrelated dedicated Chrome remains reachable.

## Acceptance evidence

- compile-time proof prevents `AttachedChromiumComposition` from terminating a
  process or constructing the owned-close grant;
- one non-cloneable lease and one active shutdown transaction exist per
  launched Browser;
- unpolled close is lazy and admitted started close survives caller Drop via a
  pre-reserved continuation;
- injected provider parts expose graceful/forced completion, lost
  acknowledgement, and shutdown failure;
- report-sink tests prove provider diagnostics remain launch-local while the
  encoded portable success remains unit-valued; the live report identifies the
  exact reaped PID and profile;
- encoded E2E proves Page/Context Dispose, disconnected, Browser Dispose, then
  Result;
- native-disconnect cleanup proves the same lifecycle/event order and still
  delegates ordinary connection cleanup, including on publication failure;
- profile and negotiation tests prove the capability is absent unless its full
  queue-free remote path is selected and uses the shared production ledger;
- accepted-handshake tests prove production connection identity matches only
  the originating supervisor ledger, while unbound and equal-plan foreign
  ledgers cannot authorize the owned close grant;
- optional-Tokio tests prove reservation exclusivity, automatic transferred
  close execution, deadline-driven future Drop, permit release, lifecycle
  rollback, and distinct timeout/runtime-cancellation metrics;
- codec, client, and runtime tests prove the optional close reason is bounded
  on the wire and shared across already-pending descendant failures without
  changing backend cleanup semantics;
- client tests prove unpolled close is reason-lazy, the root family shares one
  bounded allocation, and post-disposal operations report the stored reason;
- live tests prove external Chrome remains reachable, owned remote close emits
  the complete terminal order, and only its dedicated process is reaped;
- dependency checks keep Node.js, bundled browsers, implicit download, hidden
  tasks, and unbounded queues out of this leaf and the minimal attach profile.
