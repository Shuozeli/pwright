<!-- agent-updated: 2026-09-05T11:51:32Z -->

# Playwright-aligned value evaluation design

> Status: implemented and verified for value-only Page, local Frame, and
> out-of-process Frame evaluation.
> Live `JSHandle` and `ElementHandle` objects remain a later, separately
> installable capability.

## Why this is the next slice

Evaluation is the first feature where a convenient API can quietly turn into
an open-ended remote-object and memory-retention system. Playwright separates
`evaluate`, which returns a serialized value, from `evaluateHandle`, which
retains a browser-side object. Pwright must preserve that observable split and
make the cheaper path independently installable.

The first slice therefore implements only by-value evaluation. Installing it
must not add a handle object kind, object-table entry, dispose command, browser
download, Node.js driver, executor, or hidden task.

## Upstream contract

At Playwright commit `de214f440b7e34937fe4886f046b78b757136087`:

- `Page.evaluate` delegates to the Page's main Frame;
- Frame `evaluateExpression` carries source, function-versus-expression mode,
  and one explicitly serialized argument;
- Promise results are awaited;
- the by-value result uses `SerializedValue`, including special numbers,
  BigInt, Date, URL, RegExp, Error, typed arrays, arrays, objects, and reference
  IDs for repeated or cyclic values;
- a thrown JavaScript value is distinct from a transport or target failure; and
- `evaluateExpressionHandle` is a different command returning a live remote
  object that must later be disposed.

Source anchors:

- `packages/playwright-core/src/client/page.ts` (`evaluate` delegates to Frame);
- `packages/playwright-core/src/client/frame.ts` (`evaluate` and argument
  serialization);
- `packages/playwright-core/src/client/jsHandle.ts` (`serializeArgument` and
  `parseResult`);
- `packages/playwright-core/src/server/javascript.ts` (expression
  normalization, Promise waiting, and temporary-handle disposal);
- `packages/protocol/spec/frame.yml` (`evaluateExpression`); and
- `packages/protocol/spec/serialized.yml` (`SerializedValue`).

## Portable capability boundary

Optional capability `pwright.evaluation` 1.0 owns one Frame-targeted command,
`evaluation.evaluate_value`. Its public parameter contains:

```text
source: bounded UTF-8
mode: expression | function
argument: optional EvaluationValue
```

Function mode invokes the supplied function with exactly one argument;
omitting it supplies JavaScript `undefined`. Expression mode does not accept an
argument. Both modes await a Promise result. `FrameEvaluationExt` dispatches
the command directly. `PageEvaluationExt` only obtains the Page's initialized
main Frame and delegates to that same command; there is no second Page command
or backend implementation.

The result is an owned `EvaluationValue`, not generic JSON and not a remote
handle. Finite JavaScript numbers are stored by their IEEE-754 bits so the
portable protocol remains deterministic and Rust values retain `Eq`; `NaN`,
positive/negative infinity, and negative zero remain explicit variants.
Arrays and objects carry optional reference IDs, and `Ref` points only to a
previously declared container. Object properties preserve JavaScript key order.
Typed arrays carry their element kind plus raw bytes. Date, URL, BigInt,
RegExp, and Error stay typed rather than degrading to strings.

The model has a validator shared by the codec and backend. It rejects:

- depth, node, property, string-byte, or binary-byte overflow;
- a duplicate container reference ID;
- a zero, forward, or missing reference;
- noncanonical finite-number variants; and
- typed-array byte lengths that do not match their element width.

## Chromium mapping

The Chromium adapter resolves a portable Frame through the synchronized
topology to its current main execution world and the exact CDP session that
reported that world. A local iframe uses its owning Page session; an OOPIF uses
its attached iframe session, whose target identity is the native Frame ID.
Before a child Frame has a main-world route, evaluation returns
`UnsupportedCapability`; it never silently executes in the main document.

The route uses CDP `uniqueContextId`, not the reusable numeric `contextId`.
Navigation, world destruction, OOPIF swap, or detach removes or replaces the
route in the same prepared topology/runtime/route-table transaction. A command
that races replacement can therefore either use the old globally unique
context and receive an execution-context-destroyed failure, or use the newly
published context; it cannot land in the replacement document by numeric-ID
reuse. The current main Frame keeps its target-session fallback during the
short interval before its first main-world event, preserving the already
implemented attach/page-creation path.

The adapter issues one `Runtime.evaluate` with `awaitPromise: true` and
`returnByValue: true`. A bounded injected serializer/deserializer reconstructs
the optional argument, evaluates the source, and converts the result to the
portable tagged representation. The browser-side serializer checks depth,
nodes, properties, UTF-8-sized strings, and binary bytes while traversing, so a
result is rejected before it can intentionally build an unlimited return
envelope. The raw CDP connection's response-byte ceiling remains the outer
defense if page code or Chromium violates that contract.

The adapter must reject any returned `objectId`: by-value evaluation is not
allowed to leak a live CDP object. CDP `exceptionDetails` maps to typed
JavaScript error details with bounded name/message/stack. Malformed envelopes
map to `ProtocolViolation`; CDP/target failures retain their existing failure
classes.

## Resource ownership

Every production installation declares:

- maximum source bytes;
- maximum argument/result depth, nodes, properties, aggregate string bytes,
  and aggregate binary bytes;
- maximum generated-script and serialized-envelope bytes; and
- maximum simultaneous evaluations.

When Frame execution routing is installed, the profile additionally derives a
maximum route count from `min(topology.frames, topology.worlds)` and a native-ID
byte limit from topology. It reserves persistent backend state for the route
identity, unique context ID, and target-local CDP client copies before the
route table can retain any of them. A composition without evaluation neither
creates this table nor declares that memory.

Call admission reserves one local concurrency slot and the conservative
`BackendStateBytes` peak synchronously before returning a future. Source,
argument, generated script, CDP response, parsed result, and final wire frame
are accounted as overlapping owners where their lifetimes overlap. An unpolled
future, cancellation, JavaScript exception, validation error, completion, and
target close all release their exact reservations. The result transferred to
application code is caller-owned; a remote server must retain its result budget
until the encoded frame owns the bytes.

No collection in this path may grow from a page-supplied length before a local
limit has admitted that length.

## Explicit non-goals of version 1.0

- `evaluateHandle`, `JSHandle`, `ElementHandle`, handle arguments, property
  Handles, and `jsonValue`;
- client callbacks or exposed functions as evaluation arguments;
- utility-world selection;
- CSP or origin policy changes; and
- pretending that a returned value remains live after evaluation.

Those are not silently emulated. A future handle capability must depend on
value evaluation, add explicit object lifecycle and `LiveHandle` budgets, and
prove deterministic async disposal plus navigation/frame-detach invalidation.

## Acceptance gates

- capability, codec, client extension, server handler, backend, and Chromium
  composition remain separately selectable;
- Page and main-Frame entry points produce the same one Frame command;
- argument and result round trips cover special scalars, cycles, objects,
  arrays, errors, and typed arrays;
- malformed references and every configured size/depth/count limit fail before
  unbounded allocation or partial publication;
- JavaScript exceptions, target closure, protocol violations, resource
  exhaustion, and transport failure remain distinguishable;
- fake-CDP tests prove main/local/OOPIF routing uses the exact session and
  unique context, rejects an unknown Frame, and rejects a returned CDP
  `objectId`;
- remote transport proves one bounded command/result round trip with no object
  Create/Dispose controls;
- dedicated Chrome proves expression, function argument, Promise, special
  value, thrown error, and post-navigation main-Frame behavior; and
- normal dependency trees contain no Node.js package, browser binary,
  installer, provider, or live-handle capability.

## Implementation checkpoint

The optional `pwright-cap-evaluation`, `pwright-codec-evaluation`,
`pwright-client-evaluation`, and `pwright-server-evaluation` crates own the
portable contract without importing CDP. The bounded browser serializer and
production service factory now live in the separately installable
`pwright-backend-cdp-evaluation`; the complete `pwright-backend-cdp` reexports
them for compatibility. Both backend descriptors mark value evaluation as
emulated because CDP provides `Runtime.evaluate`, while Pwright supplies the
Playwright-compatible typed value serialization above it.

`pwright-composition-chromium-evaluation` is the corresponding minimal server
leaf. It binds exactly one Frame and injected target-local CDP client. Main
Frames use the default execution context; child Frames require an explicit
`uniqueContextId` and retain their declared route reservation for the complete
installation lifetime. Its generated handler, codec, backend operation peak,
and optional route bytes are claimed from one caller-owned production ledger.
An isolated consumer and exact CI Cargo-tree allowlist prove that navigation,
Handles, the aggregate backend/composition, client, provider, launcher,
installer, runtime choice, Node.js, and browser bytes are absent.

The synchronized service state stores Page routes and Frame execution routes
behind one publication lock. Each observed main world binds its Frame to the
exact local-Page or OOPIF session and `uniqueContextId`; unchanged routes reuse
their service, while replacement or removal retires the stale service during
the same topology synchronization. An unrecognized Frame still returns
`UnsupportedCapability`. Cancellation drops the portable result wait but
deliberately does not call renderer-wide `Runtime.terminateExecution`, which
could kill unrelated JavaScript.

The browser-side walker checks depth, node, entry, aggregate UTF-8 string, and
binary limits while constructing the tagged envelope. The Rust decoder repeats
those admissions before cloning strings, decoding base64, or allocating child
vectors, then runs the shared semantic validator. Production call admission
reserves its fixed conservative peak synchronously; tests prove an unpolled
future blocks a second call and releases the reservation on drop.

Release-profile fake tests cover exact session routing, child-Frame rejection,
cyclic/special values, omitted-as-`undefined` arguments, malformed references,
returned `objectId`, JavaScript errors, and resource admission. A binary remote
round trip emits exactly one Result and no Create/Dispose controls. Chrome 152
then passed expression, cyclic object, negative zero, typed array, BigInt and
URL, async function argument, explicit omitted argument, thrown TypeError,
browser-side size rejection, and post-navigation evaluation against an
externally owned dedicated CDP profile. A second production-composition test
serves a cross-site iframe fixture through `serve-lib`. The optional
caller-driven topology-event component—not test-side event parsing—observes
Chromium create the OOPIF, recursively initializes its session, and publishes
its main world. The portable child-Frame command then executes in that session
via its `uniqueContextId` while the external browser survives cleanup. The
driver boundary is recorded separately in
`pwright-chromium-topology-events.md`.
