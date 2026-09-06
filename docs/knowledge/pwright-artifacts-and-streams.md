<!-- agent-updated: 2026-09-05T10:59:02Z -->

# Pwright Artifact and Stream boundary

## Decision

Remote artifacts use a Playwright-shaped pull model: a producer may create an
optional `Artifact` resource, the client opens a child `Stream`, and bounded
`Stream.read` commands return the next bytes. Pwright will not add an
unsolicited trace-push channel or make tracing depend on Tokio, WebSocket,
gRPC, a filesystem, ZIP, or a browser downloader.

Artifact transfer is an independent capability. Merely knowing the stable
Artifact and Stream object kinds allocates nothing and advertises no command.
Only an installed producer may create an Artifact; only the artifact-transfer
capability may create/read/close its Stream children. Parent teardown and
explicit Artifact deletion dispose all child Streams.

## Playwright source evidence

The local Playwright checkout separates recording from remote transfer:

- `packages/playwright-core/src/client/tracing.ts` requests tracing entries for
  a local connection but requests an archive Artifact for a remote connection.
- `packages/playwright-core/src/client/artifact.ts` implements remote `saveAs`
  by asking the Artifact for a Stream and piping it to a client-local file.
- `packages/playwright-core/src/server/dispatchers/artifactDispatcher.ts`
  creates a file read stream with a 1 MiB high-water mark and keeps the
  Artifact blocked until that stream ends or closes.
- `packages/playwright-core/src/server/dispatchers/streamDispatcher.ts`
  services caller-driven `read(size)` commands and destroys the readable on
  explicit close/disposal.
- `packages/protocol/spec/artifact.yml` defines separate Artifact, Stream, and
  WritableStream protocol objects.

The useful contract is resource identity, explicit close/delete, and
client-driven bounded reads. Pwright need not reproduce Node streams, server
absolute paths, or Playwright's empty-buffer EOF sentinel.

## Pwright differences

1. `ArtifactBytes` is a distinct connection-ledger class. A source artifact
   and one returned read slice may coexist, so production requirements cover
   their attainable overlap rather than counting only the larger buffer.
2. A requested read size is required and nonzero. It is rejected before any
   result allocation when it exceeds the installed per-read limit.
3. Results contain an explicit `eof` flag. Empty artifacts therefore complete
   without relying on an ambiguous zero-length byte sentinel.
4. Server-local absolute paths never appear in the portable initializer. A
   local-only filesystem extension may expose paths under separate authority.
5. There is at most one prepared read per Stream. Preparation reserves and
   copies the result but does not advance the cursor. Commit advances it; drop
   rolls it back. This gives cancellation a precise boundary.
6. Stream close is explicit and idempotent after success. It rejects while a
   prepared read exists, then releases that Stream's shared-source owner. The
   single source lease remains only while its Artifact or another Stream still
   owns it.

## Implemented foundation

`pwright-core` now includes the byte-class vocabulary only.
`pwright-browser-model` assigns stable Artifact and Stream IDs/tags and permits
Artifact parents under Browser, BrowserContext, or Page, with Stream under
Artifact. These descriptors do not install transfer behavior.

`pwright-artifact-model` is command-free and standard-library-only. One bounded
immutable Artifact source is leased exactly once and may be shared by multiple
Streams, each with an independent cursor and prepared-read gate. Deleting the
bare Artifact prevents new Streams; registry-owned deletion additionally
aborts existing children. The model performs no I/O and starts no task. Seven
AAA tests cover chunked reads, exact resource overlap, rollback on prepared-read
drop, concurrent-read rejection, admission failure before cursor mutation,
close cleanup, empty EOF, shared-source accounting, and independent cursors.

`pwright-cap-artifact` now publishes the frozen 1.0 command contract. Artifact
supports `open_stream`, `failure`, and `delete`; Stream supports `read` and
`close`. The five commands target only their exact stable object kinds. Their
schema, command metadata, typed values, typed server chain, and human-readable
protocol reference are generated together and checked against the committed
1.0 baseline.

`pwright-codec-artifact` is an independently installed bounded binary codec.
Commands accept no timeout/cancellation metadata or attachments. A successful
read contains exactly one attachment at index zero, including an empty
attachment at explicit EOF. Declared and actual lengths must match and both
command requests and returned bytes must fit the configured read limit. Decode
copies bytes into a caller-supplied `ArtifactBytes` ledger before returning;
decoded `Artifact.failure` strings are also copied into that ledger, and wire
error strings have independent limits. Six AAA codec tests cover all command
and non-byte result shapes, typed failures, independent attachment ownership,
explicit empty EOF, invalid command admission, and malformed attachment
rejection before result budgeting.

`pwright-server-artifact` supplies only the generated five-service handler
chain. Installing that chain advertises `pwright.artifact` 1.0; a composition
without it does not. It does not choose a producer or implement object storage.

`pwright-client` now has typed Artifact and Stream handles in its stable object
vocabulary. Browser, BrowserContext, or Page may claim an Artifact result;
Artifact may claim a Stream result. `pwright-client-artifact` independently
adds Playwright-shaped `create_read_stream`, `failure`, `delete`, `read`, and
`close` methods. Remote mode requires exact lifecycle Create announcements,
direct mode still obeys the shared client live-object limit, and dropping a
handle performs no hidden I/O.

`pwright-artifact-service` is the first task-free in-memory producer registry.
It fallibly preallocates independent Artifact and Stream tables, accepts only
caller-supplied Artifact parents and bytes/failures, injects the monotonic
Stream ID source, and has explicit source, read, failure, retained-result,
Artifact-count, and Stream-count limits. A production profile calculates the
attainable source/result byte overlap plus worst-case scope/lease count.
Artifact deletion, parent disposal, and connection abort invalidate child
Streams first, including a prepared read, and return the exact child-first
object IDs needed by a lifecycle adapter. Eight AAA tests include a complete
typed client to binary attachment to generated server to registry pull.

`pwright-artifact-runtime` is the opt-in lifecycle adapter. Registry insertions
remain unpublished behind a held preparation until the controlled runtime has
reserved and committed the matching object and `Create`. Artifact deletion and
Stream close lock a stable registry target/child-first plan, compare it with the
runtime disposal plan, commit the runtime, then retire the registry state. The
ordered service bundle holds the shared control barrier and sends through the
exact Stream `Create` or target `Dispose` before allowing a successful command
result. A failed Stream-creation fence triggers local runtime/registry disposal
compensation. Tests cover normal wire order, runtime capacity, transport-close
rollback, failed-fence compensation, and exact TraceBytes-to-ArtifactBytes
ownership transfer without installing a task or executor.

## Tracing producer checkpoint

The raw registry service deliberately remains an in-process building block; a
remote composition must install the runtime adapter and its ordered service
bundle. The shared `pwright-trace-format` layer computes the exact PWRTRACE
size, reserves `ArtifactBytes` before allocating, and returns the same owned
buffer; the Artifact model, registry, and runtime adopt it without a second
copy or lease.

The optional producer is now frozen separately as `pwright.tracing_artifact`
1.0. Its one Context command depends on both tracing and Artifact 1.0, detaches
one complete chunk through the public recorder seam, performs the owned
handoff, and fences the exact Artifact `Create` before its successful result.
Artifact allocation reuses the runtime Artifact service's connection-global ID
source, which is also used by child Streams. Encoding failure releases the
detached chunk; publication failure compensates the unpublished Artifact in
both lifecycle and registry state. Neither outcome leaks `TraceBytes` or
`ArtifactBytes`.

The producer contract, codec, server handler, client facade, and composition
are separate crates, so tracing 1.0 and Artifact 1.0 remain unchanged and
useful alone. Filesystem, ZIP-compatible archives, and other Artifact producers
remain independent compositions of the same transfer capability.
