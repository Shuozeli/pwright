<!-- agent-updated: 2026-09-04T18:17:33Z -->

# Pwright generator output ownership

> Status: DT-031 decision with navigation and locator multi-output generation,
> protocol references, and optional server bindings implemented.
> It fixes dependency direction and source ownership; it does not freeze the v2
> public or wire ABI.

## Decision

One capability schema is the reviewed source of truth for its public semantic
operations and protocol mapping. The canonical source belongs to the portable
capability crate, under `pwright-cap-<name>/schema/`, rather than to a codec,
backend, client, server, or runtime crate.

The development-only generator may produce several committed artifacts from
that source, but every artifact is emitted into the crate that owns the
corresponding responsibility:

| Output | Owner | Allowed normal dependencies | Contains |
| --- | --- | --- | --- |
| Portable capability surface | `pwright-cap-<name>` | `pwright-core` | Capability/command IDs, versions, dependencies, typed parameter/result/error models, `CommandSpec` implementations |
| Canonical binary binding | `pwright-codec-<name>` | capability, binary/common codecs, core, transport | Bounded owned/borrowed wire models, schema catalog, encoding/decoding, public-model conversions |
| Optional server binding | `pwright-server-<name>` | capability, codec, generic server boundary, transport contract | Static typed handler constructors; never a global runtime registry |
| Protocol reference | capability schema directory or project docs | none at runtime | Deterministic human-readable commands, fields, constraints, versions, and compatibility notes |

The generic `pwright-core`, `pwright-runtime`, `pwright-client`, and
`pwright-server-wire` crates must not acquire dependencies on individual
capabilities. A generated capability also must not depend on its codec,
transport, executor, browser backend, YAML parser, or generator.

Connection-level message families that are not independently installable
capabilities use a `protocol` schema identity. Their canonical source, generated
codec model/catalog, baseline, and protocol reference stay in the owning codec
crate. They may not declare capability commands, dependency negotiation, or a
server-binding output. Control and handshake use this identity; this prevents
connection and lifecycle traffic from being mislabeled as user-selectable
browser features.

## Install graph

The dependency direction remains:

```text
pwright-core
  └─ pwright-cap-navigation
       ├─ pwright-client-navigation ── pwright-client
       ├─ pwright-backend-cdp ──────── pwright-cdp
       └─ pwright-codec-navigation ─── pwright-codec-binary
            │                         pwright-codec-common
            │                         pwright-transport
            └─ pwright-server-navigation ── pwright-server-wire
```

Installing only the navigation domain contract therefore does not install a
codec, Tokio, CDP, Chrome, Node.js, or a browser downloader. Installing the
in-process backend does not install a network transport. Installing the remote
codec does not install a browser backend. Installing the optional generated
server binding adds only the codec, generic server/wire contracts, and their
portable prerequisites; the application still chooses services, backend,
executor, concrete transport, and supervisor.

The same graph exists independently for locator. Installing
`pwright-cap-locator` pulls only `pwright-core`; the generated
`pwright-server-locator` layer is opt-in and does not select Tokio, CDP, a
browser provider, Node.js, or Chrome.

## Schema shape required for generation

Commands must become first-class semantic declarations rather than merely a
pair of already-expanded root-union variants. Each command needs:

- a language-neutral symbolic name;
- a stable command ID and numeric method tag;
- stable command/result wire tags;
- parameter, success-result, and typed error-detail model references;
- the object kinds on which the command is valid;
- the capability version in which it was introduced.

The generator synthesizes the standard call envelope (`call_id`, `object_id`,
`CallMetadata`) and result envelope (`call_id`, success or `OperationError`).
Those common lifecycle fields are not repeated in every public parameter
model. This follows Playwright's separation between method parameters/results
and channel envelopes while preserving Pwright's bounded binary framing.

The initial migration must preserve navigation's published golden bytes. The
existing root tags and positional field order therefore become explicit
command metadata before the hand-written expanded variants are removed.

Capability dependencies are distinct from wire-library imports:

- `dependencies` controls installation and negotiation between capabilities;
- `imports` resolves reusable wire/value definitions for a selected codec;
- neither implies a backend, transport, executor, or browser provider.

## Multi-output generator contract

Generation remains an explicit development command. There is no `build.rs`.
The CLI will accept explicit output paths for each requested artifact and will:

1. parse and resolve the current source and library imports once;
2. compare previous capability and previous import baselines before emission;
3. build one validated intermediate representation shared by all emitters;
4. render every requested artifact in memory;
5. in write mode, write all outputs only after every render succeeds;
6. in check mode, mutate nothing and report every missing or stale output.

This prevents a failed second emitter from leaving half-updated committed
artifacts. Output paths are CLI inputs, never embedded in the language-neutral
schema.

## Dispatch boundary

The schema may generate command descriptors and typed handler constructors,
but it must not generate a process-global registry. Applications compose only
the handlers for installed capabilities using the existing static
`or_else`/inventory boundaries. The generated inventory is descriptive; it
does not start tasks, open sockets, launch browsers, or choose concurrency.

The implemented descriptor layer is allocation-free: each capability exports
an ordered slice of command ID, method tag, and semantic target kinds.
`ServerCapabilityInventory` validates the installed typed handlers and exposes
the same descriptor by command ID or method tag. This is routing metadata, not
a type-erased service locator; the handler chain remains statically composed.

The implemented server emitter generates one service parameter per declared
command and a complete left-associated `ConcurrentServerHandlerChain` using a
shared `Arc<WireProtocol<_>>`. For navigation this artifact lives in
`pwright-server-navigation`. Its constructor neither imposes a runtime nor
starts work; the existing generic handler traits apply service and transport
bounds only when an application actually runs the chain. Its capability-source
implementation derives the handshake manifest from the installed typed chain.

## Public-model conversion boundary

Mechanical conversions belong in generated codec bindings:

- scalar, enum, option, record, and tagged-union mappings;
- standard call/result envelopes;
- declared numeric, length, count, and future cross-field validation.

Policy-rich portable values may opt out of generated public definitions with
`public_external: true`. The schema still owns their stable public names and
wire mapping, while the capability module owns constructors and invariants and
the codec keeps the necessary reviewed adapters. Recursive singular wire-model
edges are boxed automatically; cycles that cannot encode a finite value are
rejected.

Backend-specific interpretation remains hand-written behind the capability
service. Examples include translating a navigation wait state to CDP lifecycle
events or adapting a semantic locator to a Firefox backend. A generated codec
must never import CDP, BiDi, Firefox remote protocol, or WebKit protocol types.

## Compatibility rules

For a compatible minor release:

- existing command IDs, method tags, wire tags, target object kinds, and model
  references remain exact;
- existing positional models and declared constraints remain exact;
- append-only enum/union variants and new commands are allowed when their
  capability version permits them;
- capability dependency minimums and imported-library versions are checked
  against their own published baselines;
- generated API, codec, dispatch metadata, and documentation must all pass
  drift checks from the same source revision.

## Rejected ownership models

- **Schema owned by the codec:** makes the portable API downstream of one wire
  format and encourages codec-to-capability dependency inversion.
- **Generated API inside the codec:** forces local-only users to install a
  serialization choice and makes optional JSON/binary codecs compete to own
  the public types.
- **Generated capability code inside core:** turns optional browser features
  into a monolithic base crate.
- **Runtime discovery registry:** hides installed behavior, weakens static
  composition, and requires global coordination.
- **`build.rs` generation:** makes normal users compile the parser/generator
  and can silently produce different sources across machines.

## Implementation sequence

1. **Implemented:** symbolic command names, semantic params/result/typed-error
   model references, declarative capability dependencies, and canonical
   command object targets are generated from capability source.
2. **Implemented:** the explicit core-only capability-surface emitter and
   multi-output/check CLI render all requested outputs before writing and
   report every stale requested artifact.
3. **Implemented:** navigation's canonical schema and baseline now
   live in `pwright-cap-navigation/schema/`; its IDs, methods, command list, and
   `CapabilityDescriptor`, public semantic models, marker structs, and
   `CommandSpec` implementations are generated without adding non-core runtime
   dependencies.
4. **Implemented:** the navigation codec binding is generated from the same
   source, including borrowed wire models and bidirectional owned public-model
   conversions. Inline semantic params/results preserve existing golden bytes;
   only policy-bearing borrowed adapters and cross-field validation remain
   hand-written.
5. **Implemented:** deterministic Markdown protocol references,
   allocation-free static command routing descriptors, and the optional
   navigation typed server constructor are generated and checked alongside the
   codec and portable API. Direct backend-dispatch adapters and migrations of
   the remaining hand-maintained capabilities remain.
   Every generated server constructor now receives a named, strongly typed
   `ServerServices` bundle. Each field is still one independent `Arc<Service>`;
   the bundle removes argument-count growth without introducing a registry,
   allocation, backend dependency, or erased dispatch.
   When a capability grows beyond sixteen commands, generation splits the
   concrete services and balanced handlers into two typed groups behind one
   small associated-handler trait. This keeps public constructor signatures
   below Rust/Clippy type-complexity limits without erasing service types or
   adding runtime allocation.
6. **Implemented:** locator is the second capability migration. Its 1.22 source
   is checked against the preserved 1.21 baseline and generates AllInnerTexts, AllTextContents, Blur, Click, Count,
   Fill, Focus, GetAttribute, Hover, InnerHtml, InnerText, InputValue, IsChecked, IsEditable,
   IsEnabled, IsVisible, Press, ScrollIntoViewIfNeeded, SelectOption, SetChecked,
   TextContent, and WaitFor command metadata plus the recursive boxed selector wire
   model,
   including appended placeholder/alt/title and test-ID string/regex variants
   after 1.4's regex text/label/filter/role branches and 1.3's negative-text,
   relative `has`/`hasNot`, and visibility filters. It also generates the
   complete catalog, protocol reference, and opt-in
   complete twenty-two-command server chain. Existing semantic constructors and
   validation remain explicitly
   policy-owned, and the established fill payload bytes remain exact.
   Capability output imports only external models referenced directly by a
   command contract; nested policy-owned models such as wait options and state
   remain the parent module's responsibility and do not leak unused imports
   into generated code.
   The generator also owns big-endian `f64` fields and removes `Eq` only from
   public and wire models that transitively contain floats, allowing Hover's
   fractional position without weakening unrelated generated types.
   Hover's modifier set is wrapped in an optional generated record so the wire
   model preserves absent versus present-empty; the hand-written policy type
   maps those cases to preserve-current versus ensure-empty Page input state.
   The independent locator 2.5 source is checked against its frozen 2.4
   baseline. It preserves the extensible Click, Focus/Blur, Press, SetChecked,
   and SelectOption records and appends the same ScrollIntoViewIfNeeded method
   ID and root tags as 1.22. Its twenty-two commands use
   the generator's split left/right service groups and one associated handler
   boundary; no erased registry or runtime allocation is introduced.
7. **Implemented:** control and handshake are protocol-family migrations owned
   by their codec crates. Handshake's `counted_by` fields preserve its split
   count header, while borrowed sequence adapters avoid an intermediate owned
   encode tree. Both families keep checked baselines, catalogs, references, and
   golden bytes without becoming installable capabilities.

DT-031 is not complete until at least this representative capability is wholly
generated across those boundaries and a second language reproduces its
fixtures.
