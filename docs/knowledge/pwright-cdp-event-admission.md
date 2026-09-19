<!-- agent-updated: 2026-09-10T05:45:01Z -->

# CDP event admission for selected Frame worlds

NODE-108 adds bounded TRACE detach/destruction identities after event lease
admission and before fanout. Actual pending-session retirement still occurs
before policy filtering. Thus a custom policy may omit an event before its TRACE
receipt; absence of a receipt is not universal proof of no native retirement.
The selected Node Frame-world policy retains these lifecycle methods. The new
helper copies no payload, changes no admission decision and installs no cursor.
[NODE-108 receipts and verification](pwright-chromium-topology-events.md#node-108-bounded-native-lifecycle-identity-receipts).

NODE-044 adds explicit extension cursor accounting through Chromium's
`with_extension_event_subscribers`. The opt-in host drag runtime declares 32
capture cursors before fanout derivation. Checked accumulation and preparation
reject overflow or stale undersized fanout. This does not change event filtering,
create subscriptions, or prove an RSS bound. Default profiles add zero cursors.

## Why this exists

NODE-025 reproduced a connection close at the 1,025 event-item ceiling during
Runtime.enable. A diagnostic counted 1,012 execution-context announcements.
The topology consumer ultimately ignores foreign isolated worlds, but those
events previously acquired shared payload leases and occupied fanout slots
before that classification. Initialization can hold an event while awaiting
native prerequisites; resource exhaustion occurs before its drain resumes.

This change moves only a conservative, already-ignored subset ahead of retained
event admission. It is not native-world destruction, general burst buffering,
an increase in event capacity, or a renderer-memory/RSS fix.

## Composable boundary

`CdpEventAdmissionPolicy` is an immutable static label and function pointer.
The function borrows the reader's method, JSON parameters and optional session
name. It must not allocate, retain inputs, block, panic or use mutable globals.
It returns whether the event remains in the connection's delivery contract.
There is no mutable registry, extra subscriber, queue, executor or task.

`CdpConnectionProductionProfile::with_event_admission_policy` explicitly
selects one policy and rejects a second installation. Default and legacy
connections remain unfiltered. The policy applies to every subscriber of that
connection, so its composing caller must account for **all** installed consumers.
Consumers needing arbitrary world inventory or future raw CDP subscriptions
must omit this policy or select a compatible union of interests.

The reader still enforces WebSocket/raw-reader and individual event byte limits.
Only afterward does it classify an event. An omitted payload is dropped before
an event-item/byte lease, compact session copy, shared event allocation or fanout
slot. Command responses are processed through the unchanged response branch;
an irrelevant event flood cannot consume their pending-call reservations.
Needed-event resource exhaustion still terminates the reader, rather than
silently discarding needed correctness state.

## Chromium policy selected by the Node profile

`chromium_frame_world_event_admission()` lives in the Chromium composition,
not the raw CDP client. The Node profile explicitly installs it because its
current consumers need main and Pwright utility worlds, not foreign-world
inventory. No aggregate/default profile installs it automatically.

Only `Runtime.executionContextCreated` may be omitted, and only when:

- A nonempty session is present.
- Required context fields have their generated schema types.
- Auxiliary data is an object with a string Frame ID.
- `isDefault` is explicitly false.
- The name does not start with the shared Pwright utility-world prefix.

Main worlds, potentially relevant Pwright worlds, non-Frame/unknown shapes,
missing/default-unknown metadata and malformed inputs are kept. All destruction,
clear, navigation, target, console and other event methods are unchanged.
The exact per-Page name, renderer session, epoch and route validation still
occur in the existing topology consumer. A retained prefix is not authority to
adopt a world, and omission is not evidence of native reclamation.

The Node connection still has 1,024 event slots and a 1,025 event-item ceiling;
the same production resource requirements and admission failures apply.
This adds only inline immutable policy configuration, not a payload cache or
another allowance. Other deployment profiles remain independently selectable.

## Evidence and limits

Six reader/profile tests cover default behavior, unchanged resource declarations,
4,096 omitted events under a completely occupied item budget followed by a real
reader response, preserved selected payload/session/lease, unchanged oversize
failure and needed-event exhaustion. Seven Chromium policy tests compare omitted
worlds with the existing generated decoder/attachment classifier and preserve
main, Pwright, malformed, missing and lifecycle events.

Release evidence is under `.work/node-world-event-admission.SCODws/`: CDP 73
and aggregate composition 254 unit cases pass. Resolution/evaluation leaves
remain 37/41 passing, with live cases explicitly ignored in that offline run.
The bounded attachment probe has six passing offline tests and separately counts
pre-admission omissions without retaining event payloads.

The first live probe on the existing Chrome succeeds in 381 ms: 61 commands,
61 replies, no pending command or diagnostic overflow; 1,440 omitted events and
32 retained execution-context announcements. Explicit close returns every
recorded host metric to default. The original failed probes remain in the
preceding checkpoint's directory; no browser restart or unrelated-page cleanup
was used to manufacture an empty-world starting state.

Full Node regression on rebuilt `ab1a0857...6f7d9a` is not green: Locator 32/32,
evaluation 38/38, lifecycle 39 reported passes before a 180-second watchdog kill,
actions 17/26 and shared 33 matches / 11 unsupported / 2 oracle errors / 11 setup
errors. Both candidate and oracle attachments hit five-second deadlines. This
does not resolve earlier attachment timeouts, browser-wide concurrent activity,
utility-world retention or the complete compatibility goal. See the
[artifact-specific Node record](../node-ffi-compatibility-design.md#optional-world-event-admission-checkpoint-2026-09-07).
