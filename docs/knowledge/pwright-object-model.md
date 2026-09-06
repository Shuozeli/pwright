<!-- agent-updated: 2026-09-03T08:42:14Z -->

# Pwright browser object-model ownership

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
