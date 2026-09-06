<!-- agent-updated: 2026-09-05T11:08:16Z -->

# Pwright tracing boundary

## Decision

Tracing is an optional BrowserContext-scoped diagnostic capability. Its base
installation records bounded API-call metadata, explicit groups, and chunk
lifecycle. It does not implicitly install DOM or ARIA snapshots, screenshots,
screencast, HTTP bodies, HAR, source capture, a filesystem, ZIP generation, an
artifact service, an executor, or a browser downloader.

Those richer streams are separate contributor plugins. A composition may join
them to the same trace chunk only after each contributor declares its own
authority, resource requirements, redaction policy, and shutdown behavior.
HAR remains a separate capability even though Playwright's server tracing
implementation reuses `HarTracer` internally.

## Playwright source evidence

The local Playwright protocol exposes `tracingStart`, `tracingStartChunk`,
groups, `tracingStopChunk`, and `tracingStop`. A stopped chunk may be discarded,
returned as server-side entries, or returned as an Artifact archive.

The client additionally collects call stacks and source files, then either
zips local entries or downloads a remote Artifact. The server recorder owns
sets for cross-chunk and per-chunk resources, in-progress calls and groups, a
serialized filesystem writer, snapshotter, HAR tracer, screencast recorders,
and event listeners. BrowserContext network resources intentionally survive
across chunks. This is useful Playwright behavior but also demonstrates why a
single tracing switch can retain several unrelated kinds of state.

Source anchors:

- `packages/protocol/spec/tracing.yml`
- `packages/playwright-core/src/client/tracing.ts`
- `packages/playwright-core/src/server/trace/recorder/tracing.ts`

## Base state machine

The portable base has these explicit states:

1. `Idle` accepts `start` and rejects chunk/event operations.
2. `Started` retains bounded session metadata and accepts `startChunk` or
   `stop`.
3. `Recording` accepts ordered metadata entries and balanced groups. A second
   `startChunk` must explicitly discard or stop the current chunk; it cannot
   orphan it.
4. `stopChunk` atomically detaches one complete chunk for an injected sink or
   discards it. Sink failure never leaves the recorder ambiguously recording.
5. `stop` requires no active chunk, releases the session, and returns to
   `Idle`.

The implemented 1.0 protocol keeps those operations on BrowserContext rather
than adding an always-present Tracing remote object. It exposes six commands:
`start`, `start_chunk`, `group`, `group_end`, `stop_chunk`, and `stop`, at
methods `0x000A_0001` through `0x000A_0006`. `stop_chunk` selects `discard` or
an injected `sink` and returns only a bounded numeric summary. Artifact/ZIP and
bulk-entry transfer therefore cannot appear merely because base tracing was
installed.

Connection or Context teardown aborts the active chunk, releases all in-memory
leases, and asks the selected sink to clean its own bounded staging. Rust
`Drop` releases memory but is not the public stop contract.

## Resource contract

The base profile must declare:

- maximum session-name, chunk-name, and chunk-title bytes;
- maximum active group depth; group name/location encoding is bounded as an
  ordinary entry payload;
- maximum entries per chunk and bytes per entry;
- maximum total in-memory chunk bytes;
- exactly one concurrently detached chunk awaiting a sink;
- one `TraceBytes` lease for every retained variable payload;
- a count bound independent of byte length, including zero-byte entries.

Admission reserves before copying. Overflow stops or rejects tracing with a
typed diagnostic; it never silently grows a vector. A sink is caller-provided
and must expose bounded admission. The core model performs no I/O and creates
no task. A filesystem/ZIP Artifact sink and a remote streaming sink are
independent optional crates.

## Atomic file sink contract

`pwright-trace-format` owns the single PWRTRACE v1 size calculation and writer.
It can stream to a sink, or reserve an exact caller-supplied `ArtifactBytes`
lease before allocating and return that same owned buffer without a staging
copy. Its encoded-byte ceiling rejects before lease or allocation. The format
crate selects no sink, filesystem, runtime, transport, or executor.

`pwright-trace-sink-file` is the first concrete sink and remains an optional
leaf above that shared encoder. The caller grants it one new directory that
does not already exist. The sink never scans, adopts, rotates, or deletes an existing directory, so stale
artifact recovery and restart policy stay with the application. It starts no
task and adds no runtime, browser, downloader, archive, or network dependency.

Every chunk is encoded directly into a create-new temporary file, the file is
synced, and the temporary path is atomically renamed to its final `.pwtrace`
name. An error removes the temporary file on a best-effort basis and never
advances committed metrics. Writes are synchronous under the sink state lock,
so `write` cannot return while a staging operation is still owned by the sink.
Consequently `abort_context` and `abort_all` have no staging to cancel; they
intentionally retain already committed files.

Admission happens before any per-chunk path is created. Independent hard
limits cover encoded directory-path bytes, distinct Contexts, committed chunks
per Context, bytes in one file, and total committed bytes per Context. The
Context table is fallibly preallocated to its declared maximum. These disk
limits do not borrow from `TraceBytes`: the recorder's lease is released when
the synchronous handoff returns, while the sink accounts persistent files in
its own metrics.

The version-1 file is a big-endian, length-delimited stream:

```text
"PWRTRACE" | u16 version | u64 context | u64 chunk | u8 label flags
[u32 name bytes | name] [u32 title bytes | title]
u32 entry count | u64 payload-byte summary
repeated { u64 sequence | u64 monotonic micros | u8 kind | u32 bytes | payload }
```

Entry kind tags 1 through 6 are `ApiCallBefore`, `ApiCallAfter`,
`ApiCallError`, `Log`, `GroupStart`, and `GroupEnd`. This is a sink format, not
the tracing-control wire protocol and not a Playwright ZIP Artifact format.

## Pull and remote-streaming boundary

`pwright-trace-sink-pull` is the task-free handoff point for applications that
want to stream or otherwise consume complete chunks. It owns a fallibly
preallocated FIFO and selects no executor, wakeup mechanism, wire codec, or
network transport. Independent limits cover global queued chunks, queued
chunks per Context, bytes in one chunk, and aggregate queued bytes. Retained
bytes include chunk name, title, and entry payloads; queue peaks, acceptance,
and rejection are observable.

While queued, each non-cloneable chunk continues to hold its original
`TraceBytes` leases and recorder detachment gate. `pop` transfers that complete
ownership to the caller and releases only pull-queue admission; the leases and
gate remain until the caller drops the returned chunk. Context or connection
abort discards matching chunks still owned by the queue. It cannot reclaim a
chunk already transferred to a consumer, so any async remote sender must own
and bound that post-pop stage explicitly. This keeps cancellation, retry,
framing, and delivery acknowledgment in a separately installable transport
adapter instead of silently choosing them in base tracing.

## Opt-in remote Artifact producer

Remote trace retrieval is now a third independently installable output policy.
`pwright.tracing_artifact` 1.0 contributes exactly one BrowserContext command,
`tracing.stop_chunk_artifact` (`0x000C_0001`), and declares explicit minimum
dependencies on `pwright.tracing` 1.0 and `pwright.artifact` 1.0. Neither base
capability gains this command when installed alone.

The producer detaches the one complete chunk, records its bounded summary,
reserves the exact final `ArtifactBytes` size, writes PWRTRACE directly into
that owned allocation, and transfers the allocation into the Artifact registry
without a staging copy. Dropping the detached chunk then releases every
`TraceBytes` lease and reopens the recorder gate. Artifact and Stream IDs use
the same connection-global source, so independently composed producers cannot
collide with Stream identities.

For a remote composition, successful completion is fenced behind the exact
Artifact `Create` control message. A missing or failed fence changes the result
to `Cleanup`, retires the unpublished Artifact from both runtime and registry,
and releases its bytes. Encoding rejection similarly drops the detached chunk
and leaves no Artifact. The opt-in client claims the announced initializer and
returns a typed Artifact plus chunk/entry/payload/encoded-size summaries.

This producer still adds no filesystem, ZIP archive, Node.js driver, browser
download, executor, or background task. Its four producer tests cover exact
PWRTRACE readback and resource handoff, pre-publication encoding rejection,
failed-Create compensation, and a typed-client-to-generated-server binary
round trip. A separate server test proves the capability is absent without its
handler, rejects incomplete dependency composition, and advertises only with
complete tracing plus Artifact dependencies.

## Contributor contract

Every contributor receives the same chunk identity and monotonic sequence
boundary, but owns its own data acquisition and budget. Base tracing can be
installed alone. DOM snapshots, ARIA snapshots, screenshot/screencast frames,
network metadata/bodies, console/errors, and source files must also be useful
or rejectable independently; selecting one does not authorize another.

The first implementation slice is complete. `pwright-trace-model` implements
the core-only recorder with explicit Idle/Started/Recording transitions,
balanced groups, monotonic sequence and chunk IDs, independent count/byte
limits, `TraceBytes` leases, abort cleanup, and a production profile that
derives its complete retained peak. A detached `TraceChunk` gates the next
chunk or session until drop, so two retained chunks cannot exceed the declared
one-chunk profile.
Chunk drop releases its entry vector and all payload/name leases before it
opens that gate, so a racing next chunk cannot overlap the old retained peak.

`pwright-cap-tracing` freezes the BrowserContext-only 1.0 command schema and
generated capability metadata. `pwright-codec-tracing` now carries all six
commands and typed failures over the bounded binary wire contract, rejects
attachments and control metadata, and returns no trace body. The opt-in
`pwright-client-tracing` crate adds the Playwright-shaped `context.tracing()`
facade, while `pwright-server-tracing` supplies the generated six-service
handler chain.

None of those crates owns I/O, an executor, a task, a browser backend, a sink,
or any rich contributor. `pwright-tracing-service` is the explicit assembly
boundary: the caller pre-registers a bounded recorder for each Context and
injects a monotonic clock plus a `TraceSink`. One preallocated bounded Context
vector backs all six generated services. Context removal aborts the recorder,
releases its base leases, and then invokes mandatory sink cleanup; even a sink
cleanup failure cannot retain the recorder. The sink may stage the one detached
chunk, but its retained chunk keeps the recorder gate closed until release.

Group locations and correlated API-call boundaries use versioned binary
entries. The recorder calculates each exact size, reserves `TraceBytes`, and
only then allocates and copies once. API entries carry a fixed-width call ID
and bounded method name, so concurrent before/terminal pairs remain
unambiguous without formatted-string staging.

`pwright-plugin-tracing-navigation` is the first independently installable
contributor. Its `Layer` wraps only the Playwright-shaped `navigation.goto`
command, resolves Page to BrowserContext through an injected route, remains
inert until a chunk is explicitly active, and records exactly one
`ApiCallBefore` plus `ApiCallAfter` or `ApiCallError`. A trace admission or
route failure becomes a typed navigation failure instead of silently dropping
diagnostics. Installing base tracing does not install this plugin, and
installing Chromium navigation does not install tracing.

The generated six-command client/server path has a full bounded in-memory
duplex test, while a dedicated-Chrome release test attaches over CDP and proves
one real `goto` produces a correlated pair, releases `TraceBytes`, removes the
temporary target, and leaves the externally owned Chrome alive. The first
DT-003 tracing workload repeats one warmup plus three measured chunks; every
sink handoff returns `TraceBytes.current` to zero. Its 2026-09-05 Chrome 152 run
observed runtime RSS `+851,968` bytes and browser RSS-sum `-257,785,856` bytes
while the browser process tree settled from 11 to 8 processes. These values are
diagnostic evidence, not budgets. An immediate final-source rerun again passed
at 11 to 8 processes and observed `+327,680` runtime bytes and `-232,587,264`
browser RSS-sum bytes, reinforcing that these are ranges to measure rather than
constants. The independent bounded atomic file sink now has exact-format,
pre-admission, count, byte, directory-ownership, and tracing-service vertical
tests. The bounded pull sink separately proves task-free FIFO backpressure,
Context cleanup, and transfer-without-release semantics. Rich contributors, a
concrete remote transport adapter, repeated series, and reviewed budgets remain
future slices.
