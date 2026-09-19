<!-- agent-updated: 2026-09-09T08:34:00Z -->

# Pwright protocol IDL decision record

NODE-074 adds optional `navigation-1.2.yml`, checked against frozen 1.1, without
replacing root 1.1. FrameGoto is Frame-only, method 65540/tags 7–8, returning
a portable nonzero object ID and optional response. `rust_minor_module: true`
selects generated Rust import placement, not wire identity; it is rejected on
non-capability schemas. All four selected typed adapters and generated server
bindings are present; production host negotiation/integration is still pending.
[Contract](pwright-frame-navigation.md#node-074-opt-in-typed-navigation-12-contract).

Mouse 1.0 reserves methods `0x000E_0001`–`0x000E_0006` and tags 1–12 for
Page-targeted move/down/up/click/dblclick/wheel. Canonical method ownership,
common metadata, generated models/capability/server and bounded typed adapters
are verified. This is an unpublished initial contract, not a frozen ABI or
completed FFI binding. [Mouse contract](pwright-raw-mouse.md#optional-mouse-10-contracts).

> Status: provisional DT-031 implementation checkpoint. Navigation and locator
> now exercise the generated ownership boundary; this does not freeze the
> public wire ABI or finish cross-language generation.

## Upstream evidence

The initial Keyboard 1.0 schema uses methods `0x000D_0001`–`0x000D_0005` for
Page-targeted down/up/press/insert-text/type, with command/result tags 1–10.
Canonical schema owns identity, field constraints and generated capability,
models, service chain and reference. Typed adapters verify payload/outer-route
identity and preserve delay separately from call metadata. It is not a released
ABI or a completed native/Node binding. [Contract proof](pwright-raw-keyboard.md).

The sequential-typing checkpoint freezes Locator 1.22 and 2.5 independently and
adds `locator.press_sequentially` to 1.23/2.6: method 131095, tags 45/46. Both
minor-compatibility and generated-output checks pass; old minors reject before
encoding. The command preserves timeout in call metadata, with only selector,
text and optional delay in its parameters. See [contract](pwright-sequential-typing.md).

The local Playwright reference was inspected at commit
`de214f440b7e34937fe4886f046b78b757136087`. Its channel protocol has 5,208
lines of source YAML under `packages/protocol/spec/*.yml`.
`utils/protocol_spec.js` loads the files in sorted order and rejects duplicate
top-level definitions. `utils/generate_channels.js` generates shared structs,
client and server channel declarations, runtime validators, and method
metadata. At this commit those generated TypeScript files total roughly 13,510
lines. The generator exits unsuccessfully when committed output differs, so CI
acts as a drift check.

The useful upstream principles are:

- one reviewed, language-neutral schema is the source of truth;
- client types, server types, validators, and metadata come from that source;
- generated output is committed and checked for drift;
- object/channel messages are distinct from the Chromium, Firefox, and WebKit
  backend protocols.

Pwright should not inherit Playwright's Node-only generator, global monolithic
registry, JSON-object wire assumptions, or runtime validator allocation model.

## Options evaluated

| Option | Strengths | Costs for Pwright | Decision |
| --- | --- | --- | --- |
| Playwright-shaped YAML | Human-reviewable, split by concern, maps naturally to commands and events | Needs a strict parser and explicit numeric wire rules | Selected provisionally, with a smaller Pwright grammar |
| Protocol Buffers | Mature generators and language support | Couples the canonical model to protobuf evolution and generated runtime dependencies; capability composition is less direct | Keep as an optional transport/API adapter, not the core source |
| FlatBuffers | Efficient reads and cross-language tooling | Builder APIs and table semantics do not match the existing bounded positional codec or typed Rust command model | Reconsider only for measured large-payload needs |
| Rust macros/traits only | Excellent Rust ergonomics and compile-time linkage | Rust becomes the source language and other clients need a second truth | Rejected as the portable source |

## Selected source and generation boundary

Each independently installable capability owns a versioned `schema/*.yml`.
Small reusable wire libraries may instead declare `library` and omit `root`;
capabilities opt into them through a canonical `imports` list. The format
declares:

- exactly one capability, library, or protocol-family ID and a version;
- a tagged-union root for capabilities and protocol families, but no root for
  libraries;
- direct library imports, with every imported namespace supplied explicitly to
  the generator;
- canonical command IDs and method tags mapped to command/result variants in
  the capability root union;
- canonically sorted record or tagged-union type IDs;
- ordered fields and ordered numeric variant tags;
- scalar wire kinds (including strict booleans and tagged enums),
  required/optional/repeated cardinality, and typed message references.

The development-only `pwright-schema-gen` executable parses with unknown-field
denial, validates identifiers, ordering, counts, references, and finite model
inhabitance, then emits deterministic Rust wire models and a complete static
`BinarySchemaCatalog`. Recursive singular message edges are boxed in owned and
borrowed Rust models; a required cycle with no finite wire value is rejected.
It also generates fallible binary
encoders and decoders over a shared `BinaryValueContext`. Generated files are
committed. There is no `build.rs`; capability consumers do not compile the
generator and do not depend on YAML or Serde.

`pwright-codec-common` is the first source library. It owns call metadata,
duration, and failure-kind wire types, but no command root, transport, executor,
browser backend, parser, or build script. `pwright-codec-navigation` imports
that library and owns only its navigation-specific types. Its composed catalog
still contains the common entries, while generated Rust reexports their exact
types and stable schema constants instead of emitting duplicate structs.

`pwright.selector` 1.0 is the second source library. It owns the minimal
nine-type recursive selector closure but no Locator command, capability root,
transport, or backend. Its generated `pwright-codec-selector` crate can be
imported by Locator-derived capabilities without importing the Locator command
set. Generator tests compare its definitions exactly with both published
Locator major tracks and prove a standalone consumer reuses the generated
types rather than emitting copies.

Generated server bindings retain one typed handler per declared command and
compose them as balanced binary trees in schema order. Capabilities above
sixteen commands split into two typed service/handler groups behind one
associated-handler boundary. The current 22-command locator tables therefore
remain practical to compile at a connection-selected v1/v2 boundary without
replacing typed dispatch with an unbounded dynamic registry.

Generate or check it from the workspace root:

```bash
CARGO_INCREMENTAL=0 cargo run -p pwright-schema-gen --release -- \
  --input crates/pwright-codec-common/schema/common.yml \
  --output crates/pwright-codec-common/src/generated_schema.rs \
  --previous crates/pwright-codec-common/schema/baselines/1.0/common.yml \
  --check

CARGO_INCREMENTAL=0 cargo run -p pwright-schema-gen --release -- \
  --input crates/pwright-codec-selector/schema/selector.yml \
  --output crates/pwright-codec-selector/src/generated_schema.rs \
  --reference-output crates/pwright-codec-selector/schema/protocol-reference.md \
  --previous crates/pwright-codec-selector/schema/baselines/1.0/selector.yml \
  --check

CARGO_INCREMENTAL=0 cargo run -p pwright-schema-gen --release -- \
  --input crates/pwright-codec-control/schema/control.yml \
  --output crates/pwright-codec-control/src/generated_schema.rs \
  --reference-output crates/pwright-codec-control/schema/protocol-reference.md \
  --previous crates/pwright-codec-control/schema/baselines/1.0/control.yml \
  --check

CARGO_INCREMENTAL=0 cargo run -p pwright-schema-gen --release -- \
  --input crates/pwright-codec-handshake/schema/handshake.yml \
  --output crates/pwright-codec-handshake/src/generated_schema.rs \
  --reference-output crates/pwright-codec-handshake/schema/protocol-reference.md \
  --previous crates/pwright-codec-handshake/schema/baselines/1.0/handshake.yml \
  --check

CARGO_INCREMENTAL=0 cargo run -p pwright-schema-gen --release -- \
  --input crates/pwright-cap-navigation/schema/navigation.yml \
  --output crates/pwright-codec-navigation/src/generated_schema.rs \
  --capability-output crates/pwright-cap-navigation/src/generated_capability.rs \
  --server-output crates/pwright-server-navigation/src/generated_server.rs \
  --reference-output crates/pwright-cap-navigation/schema/protocol-reference.md \
  --import crates/pwright-codec-common/schema/common.yml

CARGO_INCREMENTAL=0 cargo run -p pwright-schema-gen --release -- \
  --input crates/pwright-cap-navigation/schema/navigation.yml \
  --output crates/pwright-codec-navigation/src/generated_schema.rs \
  --capability-output crates/pwright-cap-navigation/src/generated_capability.rs \
  --server-output crates/pwright-server-navigation/src/generated_server.rs \
  --reference-output crates/pwright-cap-navigation/schema/protocol-reference.md \
  --import crates/pwright-codec-common/schema/common.yml \
  --previous crates/pwright-cap-navigation/schema/baselines/1.0/navigation.yml \
  --previous-import crates/pwright-codec-common/schema/baselines/1.0/common.yml \
  --check

CARGO_INCREMENTAL=0 cargo run -p pwright-schema-gen --release -- \
  --input crates/pwright-cap-locator/schema/locator.yml \
  --output crates/pwright-codec-locator/src/generated_schema.rs \
  --capability-output crates/pwright-cap-locator/src/generated_capability.rs \
  --server-output crates/pwright-server-locator/src/generated_server.rs \
  --reference-output crates/pwright-cap-locator/schema/protocol-reference.md \
  --import crates/pwright-codec-common/schema/common.yml \
  --previous crates/pwright-cap-locator/schema/baselines/1.21/locator.yml \
  --previous-import crates/pwright-codec-common/schema/baselines/1.0/common.yml \
  --check

CARGO_INCREMENTAL=0 cargo run -p pwright-schema-gen --release -- \
  --input crates/pwright-cap-locator/schema/locator-v2.yml \
  --output crates/pwright-codec-locator/src/generated_schema_v2.rs \
  --capability-output crates/pwright-cap-locator/src/generated_capability_v2.rs \
  --server-output crates/pwright-server-locator/src/generated_server_v2.rs \
  --reference-output crates/pwright-cap-locator/schema/protocol-reference-v2.md \
  --import crates/pwright-codec-common/schema/common.yml \
  --previous crates/pwright-cap-locator/schema/baselines/2.4/locator-v2.yml \
  --previous-import crates/pwright-codec-common/schema/baselines/1.0/common.yml \
  --check

CARGO_INCREMENTAL=0 cargo run -p pwright-schema-gen --release -- \
  --input crates/pwright-cap-context-creation/schema/context-creation.yml \
  --output crates/pwright-codec-context-creation/src/generated_schema.rs \
  --capability-output crates/pwright-cap-context-creation/src/generated_capability.rs \
  --server-output crates/pwright-server-context-creation/src/generated_server.rs \
  --reference-output crates/pwright-cap-context-creation/schema/protocol-reference.md \
  --import crates/pwright-codec-common/schema/common.yml \
  --previous crates/pwright-cap-context-creation/schema/baselines/1.0/context-creation.yml \
  --previous-import crates/pwright-codec-common/schema/baselines/1.0/common.yml \
  --check

CARGO_INCREMENTAL=0 cargo run -p pwright-schema-gen --release -- \
  --input crates/pwright-cap-page-lifecycle/schema/page-lifecycle.yml \
  --output crates/pwright-codec-page-lifecycle/src/generated_schema.rs \
  --capability-output crates/pwright-cap-page-lifecycle/src/generated_capability.rs \
  --server-output crates/pwright-server-page-lifecycle/src/generated_server.rs \
  --reference-output crates/pwright-cap-page-lifecycle/schema/protocol-reference.md \
  --import crates/pwright-codec-common/schema/common.yml \
  --previous crates/pwright-cap-page-lifecycle/schema/baselines/1.0/page-lifecycle.yml \
  --previous-import crates/pwright-codec-common/schema/baselines/1.0/common.yml \
  --check

CARGO_INCREMENTAL=0 cargo run -p pwright-schema-gen --release -- \
  --input crates/pwright-cap-browser-lifecycle/schema/browser-lifecycle.yml \
  --output crates/pwright-codec-browser-lifecycle/src/generated_schema.rs \
  --capability-output crates/pwright-cap-browser-lifecycle/src/generated_capability.rs \
  --server-output crates/pwright-server-browser-lifecycle/src/generated_server.rs \
  --reference-output crates/pwright-cap-browser-lifecycle/schema/protocol-reference.md \
  --import crates/pwright-codec-common/schema/common.yml \
  --previous crates/pwright-cap-browser-lifecycle/schema/baselines/1.0/browser-lifecycle.yml \
  --previous-import crates/pwright-codec-common/schema/baselines/1.0/common.yml \
  --check

CARGO_INCREMENTAL=0 cargo run -p pwright-schema-gen --release -- \
  --input crates/pwright-cap-tracing/schema/tracing.yml \
  --output crates/pwright-codec-tracing/src/generated_schema.rs \
  --capability-output crates/pwright-cap-tracing/src/generated_capability.rs \
  --server-output crates/pwright-server-tracing/src/generated_server.rs \
  --reference-output crates/pwright-cap-tracing/schema/protocol-reference.md \
  --import crates/pwright-codec-common/schema/common.yml \
  --check

CARGO_INCREMENTAL=0 cargo run -p pwright-schema-gen --release -- \
  --input crates/pwright-cap-artifact/schema/artifact.yml \
  --output crates/pwright-codec-artifact/src/generated_schema.rs \
  --capability-output crates/pwright-cap-artifact/src/generated_capability.rs \
  --server-output crates/pwright-server-artifact/src/generated_server.rs \
  --reference-output crates/pwright-cap-artifact/schema/protocol-reference.md \
  --import crates/pwright-codec-common/schema/common.yml \
  --previous crates/pwright-cap-artifact/schema/baselines/1.0/artifact.yml \
  --previous-import crates/pwright-codec-common/schema/baselines/1.0/common.yml \
  --check
```

`--check` never rewrites output and fails if it is missing or stale. With
`--previous`, it first applies the source-level minor-compatibility rules. Each
published source, including a library, keeps its own baseline. Imported types
participate in the resolved catalog and compatibility APIs, so an imported
record mutation cannot be hidden behind an unchanged capability source.
`--previous-import` supplies the old dependency graph separately; when it is
omitted, `--previous` intentionally compares against the current imports.
When optional outputs are present, the core-only capability surface, codec
binding, capability-specific typed server chain, and human-readable protocol
reference are all rendered before any write. `--check` reports every
missing/stale requested artifact without mutating it. The server output shares
one wire protocol across one typed handler per declared command and contains no
backend, executor, runtime, task spawning, or browser acquisition. The
generated reference lists commands, IDs, methods, semantic targets, model
ownership, positional fields, cardinality, variants, defaults, and declared
resource constraints.

Generated value codecs require explicit limits for individual UTF-8 strings,
binary values, repeated-item counts, nesting depth, and total message nodes.
Decode validates declared lengths before copying, uses fallible reservation,
and preflights `count × minimum_element_wire_size` before allocating repeated
storage. The minimum size is derived from the schema. A generated simple goto
has byte-for-byte equality with the published navigation bytes. The production
navigation codec now uses generated models and generated bounded value codecs
for all four commands/results, including response headers, failure kinds, and
typed error details. Its public capability conversions retain navigation-only
checks for aggregate header bytes and header ordering; the former parallel
hand-written payload codec has been removed.

For every type that transitively contains strings, bytes, or repeated values,
the generator also emits a `*Ref<'a>` encoding model. UTF-8 and binary fields
are borrowed directly. Repeated fields use an object-safe borrowed sequence
whose count is checked and written by generated code before item encoding.
Navigation production encoding now uses these views, so URLs, frame/loader IDs,
status text, error messages, and headers are not cloned into an intermediate
wire tree. Response headers retain deterministic canonical order through a
bounded, allocation-free view, including duplicate pairs. Decode intentionally
continues to produce validated owned models.

Integer fields may declare inclusive `minimum` and/or `maximum` constraints.
The source parser accepts the complete `i64 | u64` wire domain without silently
narrowing large unsigned bounds, rejects empty/reversed/out-of-kind ranges, and
emits the constraint into both owned and borrowed encoders, decoders, and the
static catalog. Constraints participate in layout fingerprints and must remain
exact through a compatible minor release. `pwright.common.duration.nanoseconds`
now declares a maximum of `999999999`; generated decode rejects an invalid value
before `Duration` construction, replacing the former manual check.

UTF-8 and byte fields may declare inclusive `minimum_length` and
`maximum_length` byte counts. Repeated fields may independently declare
`minimum_items` and `maximum_items`. These semantic bounds compose with—not
replace—the caller's runtime resource ceilings. Generated encode checks them
before writing, and decode checks declared lengths/counts before copying or
reserving storage. Both ranges enter catalog validation, layout fingerprints,
and exact minor compatibility. Length constraints currently apply to required
or optional UTF-8/byte fields; repeated primitive element constraints wait for
an element-aware borrowed-sequence contract.

A repeated field may instead take its canonical wire length from an earlier
required `u32` field with `counted_by`. The generator rejects a missing or later
field, the wrong scalar/cardinality, and use on a non-repeated field. It emits
the relationship into `BinaryFieldSchema`, layout fingerprints, compatibility
checks, and Markdown. Encode verifies exact agreement between the external
count and actual borrowed or owned sequence length without writing an inline
count. Decode uses that count for the repeated-item ceiling, minimum wire-byte
preflight, fallible reservation, and item loop. Handshake uses two such fields
so its published `required_count, optional_count, required items, optional
items` layout remains exact.

An enum may declare one existing variant as its semantic `default`. Both the
portable public enum and wire enum derive `Default` from that selection.
Unknown selections are rejected, and adding, removing, or changing the default
requires a new major version. Navigation declares `load` as the default for
`wait_until`.

Capability sources may also declare a canonical `commands` list. Each entry
binds one stable command ID and method tag to two distinct variants in the root
union and references its semantic params, result, and typed-error models.
Published semantic models require unique language-neutral `public_name` values;
changing one in a compatible minor release is rejected. The capability emitter
generates public records/enums/unions, marker structs, and
`CommandSpec`/`CapabilityCommandSpec` implementations. The codec emitter
generates the wire models, bidirectional owned conversions, public
`BinaryCommandSchema` constants, and command slice. Navigation tests compare
those mappings directly with the typed contracts. Its command variants embed
the semantic params/results as inline positional messages, retaining the
established payload bytes. Libraries cannot declare commands. Within a
compatible minor release an existing mapping is exact; new commands may be
added only alongside valid root variants.

A schema model may set `public_external: true` when its portable representation
must remain policy-owned rather than mechanically generated. It still requires
a stable `public_name` and remains the command's semantic contract, but the
capability emitter imports it from the hand-written parent module and the codec
emitter does not invent a conversion. Locator uses this boundary for validated
selector/option parameter types, typed errors, and its unit result. Its wire
selector union remains schema-owned and recursively boxed. This keeps command
IDs, targets, root tags, wire layout, catalog, reference, and server chain under
one source without moving construction invariants into generated structs.

The semantic selector and matcher types are now owned by the zero-dependency
`pwright-selector-model` crate and reexported by `pwright-cap-locator`. This
changes no external model name, schema ID, generated wire type, catalog, or
encoded byte. A separate schema-library publication is still required before a
new capability can import the recursive wire representation without copying
the existing Locator schema.

Locator 1.6 also uses this boundary for the semantic `CountParams` and
`CountResult` aliases while the schema remains authoritative for method
`131075`, root tags 5/6, the typed outcome union, and the generated handler
route. Compatibility is checked against the preserved 1.5 source.

Locator 1.7 applies the same rule to `IsVisibleParams` and its boolean result.
The schema owns method `131076`, root tags 7/8, the outcome union, and the
generated fourth handler route; compatibility is checked against the preserved
1.6 source. Client-composed `is_hidden` requires no IDL entry.

Locator 1.8 appends `IsChecked`, `IsEditable`, and `IsEnabled` with methods
`131077` through `131079` and root command/result tags 9 through 14. Each owns
distinct external params/result names, so a backend or client adapter may
implement the routes independently. Compatibility is checked against the
preserved 1.7 source. Client-composed `is_disabled` requires no IDL entry.

Locator 1.9 appends `WaitFor` at method `131080` and root command/result tags
15/16. Its external semantic models are `WaitForParams`, `WaitForOptions`, and
`WaitForState`; the state enum owns attached/detached/visible/hidden and marks
visible as the default. Compatibility is checked against the preserved 1.8
source. The command result is unit, so no wire-owned handle is introduced.

Locator 1.10 appends `TextContent` at method `131081` and root command/result
tags 17/18. Its external `TextContentParams` contains the immutable selector;
its external result is `Option<String>`, represented by an optional UTF-8 field
with a 1 MiB maximum. Compatibility is checked against the preserved 1.9
source, so the nullable read cannot renumber any earlier route.

Locator 1.11 appends `InnerText` at method `131082` and root command/result
tags 19/20. Its external params keep the immutable selector and its external
result is a required UTF-8 string with a 1 MiB maximum. Compatibility is
checked against the preserved 1.10 source, so rendered-text semantics cannot
renumber or weaken the nullable raw-text route.

Locator 1.12 appends `InnerHtml` at method `131083` and root command/result
tags 21/22. Its external params keep the immutable selector and its external
result is a required UTF-8 string with a 1 MiB maximum. Compatibility is
checked against the preserved 1.11 source, so serialized markup cannot
renumber either established text-read route.

Locator 1.13 appends `GetAttribute` at method `131084` and root command/result
tags 23/24. Its external params carry the immutable selector plus a UTF-8 name
bounded to 64 KiB; its external result is optional UTF-8 bounded to 1 MiB.
Compatibility is checked against the preserved 1.12 source, so the new
nullable attribute read cannot renumber or weaken any established locator
route.

Locator 1.14 appends `InputValue` at method `131085` and root command/result
tags 25/26. Its external params retain only the immutable selector and its
external result is required UTF-8 bounded to 1 MiB. Compatibility is checked
against the preserved 1.13 source, so this strict form-value read cannot
renumber or weaken the nullable attribute route or any earlier locator route.

Locator 1.15 appends `AllTextContents` at method `131086` and root
command/result tags 27/28. Its external params retain only the immutable
selector and its external result is a repeated UTF-8 field capped at 4096
items. The handwritten codec further enforces the reviewed 1 MiB per-item and
4 MiB aggregate limits that the current IDL cannot attach directly to each
primitive repeated element. Compatibility is checked against the preserved
1.14 source, so this collection snapshot cannot renumber or weaken any strict
getter or earlier route.

Locator 1.16 appends `AllInnerTexts` at method `131087` and root command/result
tags 29/30. Its external params retain only the immutable selector and its
external result is a repeated UTF-8 field capped at 4096 items. The handwritten
codec applies the same reviewed 1 MiB per-item and 4 MiB aggregate limits as
the raw text-content collection. Compatibility is checked against the
preserved 1.15 source, so the rendered-text snapshot cannot renumber or weaken
AllTextContents or any earlier route.

Playwright-shaped `Locator.clear()` and `Locator.all()` do not add IDL entries.
Clear is a client alias for the existing Fill command with an empty value; All
uses the existing Count command and local `nth(i)` selector construction. The
absence of new command IDs is intentional capability composition, so locator
protocol remains at 1.16 for this checkpoint.

Locator 1.17 appends `Hover` at method `131088` and root command/result tags
31/32. Its external params contain the immutable selector and bounded hover
options: force, auto/no scrolling, an optional
`pwright.locator.keyboard_modifiers` record containing a canonical set of at
most five modifiers, an optional finite `Point`, and trial. The wrapper is
intentional: an absent record preserves the Page's current pressed modifiers,
while a present record with zero values requests an empty set before restoring
the prior state. `Point.x/y` are the first IDL `f64` fields; the binary
representation is big-endian IEEE-754 and generated float-containing models
derive `PartialEq` without incorrectly deriving `Eq`. Compatibility is checked
against the preserved 1.16 source, so Hover cannot renumber or weaken an
earlier route.

Locator 1.18 appends `Focus` at method `131089` with root command/result tags
33/34 and `Blur` at method `131090` with tags 35/36. Both external parameter
records contain only the immutable selector and both reuse the bounded unit
action outcome. Compatibility is checked against the exact preserved 1.17
source. The connection-selected codec also rejects both routes when 1.17 or an
earlier compatible version was negotiated, preventing an append-only schema
addition from becoming an accidental old-peer dispatch path.

Locator 2.1 appends the same Focus and Blur contracts, method IDs, and root
tags to the independent 2.x track. Compatibility is checked against the exact
frozen 2.0 source. A connection that selects 2.0 continues to reject both
routes before encoding or backend dispatch; selecting 2.1 enables them without
changing the 2.0 Click record or any locator 1.x bytes.

Locator 1.19 and 2.2 append the same `Press` command at method `131091` and
root tags 37/38. Its external parameters contain a selector, a bounded
128-byte/16-token key expression, optional delay, and `no_wait_after`.
Compatibility is checked independently against frozen 1.18 and 2.1 sources;
either older selected minor rejects Press before payload encoding or backend
dispatch while every earlier command retains its exact bytes.

Locator 1.20 and 2.3 append the same `SetChecked` command at method `131092`
and root tags 39/40. Its external parameters contain a selector, the desired
boolean state, and a bounded 64-byte extensible record with optional force,
position, scroll, and trial tags 1 through 4. Compatibility is checked
independently against frozen 1.19 and 2.2 sources. Because the route is new,
both majors can use the extensible encoding without changing any existing
positional payload; either older selected minor rejects it before encoding or
dispatch.

Content 1.0 starts a new independently negotiated `pwright.content` capability
with Page-only `PageScreenshot` at method `327681` (`0x0005_0001`) and root
command/result tags 1/2. Its options are an extensible record with optional
format, quality, full-page, clip, and scale fields. The result control payload
contains attachment index, exact byte length, and effective format; screenshot
bytes are never base64 inside this application protocol. The handwritten codec
requires exactly one attachment and validates index, length, format, options,
and the configured decoded-byte ceiling before creating the portable result.
The checked-in 1.0 baseline and generator drift check make this the publication
point for future compatible optional fields.

The public screenshot values named by that schema are external to the
generated capability and now come from the command-free
`pwright-screenshot-model` crate. `pwright-cap-content` reexports the same Rust
types for compatibility. This is a Rust ownership refactor only: content 1.0's
source schema, generated wire types, catalog fingerprints, command IDs, method
tag, and encoded bytes do not change.

Locator screenshot 1.0 is a distinct `pwright.locator_screenshot` capability
with one Page/Frame-scoped `LocatorScreenshot` command at method `589825`
(`0x0009_0001`) and root tags 1/2. Its source imports `pwright.common` and the
command-free `pwright.selector` library. The generated codec therefore reuses
the selector tree without importing or advertising `pwright.locator`. Its
external params pair the portable `Selector` with shared screenshot options;
the 1.0 wire accepts only format, quality, and scale, while the codec rejects
Page-only clip/full-page values before encoding. Successful results declare one
attachment's index, exact length, and format before copying it into a bounded
`ScreenshotBytes` lease. The frozen 1.0 source and generated reference establish
the independent compatibility line before its Chromium backend is installed.
Page and Locator screenshot codecs share the command-free
`pwright-codec-screenshot` attachment boundary for index/count/length/limit
validation and budgeted result copying; this helper owns no schema root or
capability relationship.

DOM content 1.0 is a separate `pwright.dom_content` capability with only
Frame-scoped `FrameContent` at method `393217` (`0x0006_0001`) and root tags
1/2. Page content is a client-side main-Frame facade, not a second command. The
empty request carries no action timeout. A successful control payload declares
attachment index zero and exact UTF-8 byte length; the codec rejects an absent,
extra, length-mismatched, oversized, or invalid-UTF-8 attachment before
publishing a non-cloneable `DomContentData` result. The checked-in 1.0 baseline
keeps this read-only authority independent from screenshot, arbitrary
evaluation, and future DOM mutation.

Capability sources may separately declare a canonically ordered
`dependencies` list. Each item is a capability ID and minimum protocol version
and is emitted into the portable `CapabilityDescriptor`; it does not import a
wire type or select a codec/backend. Libraries, self dependencies, malformed
IDs, and duplicate/out-of-order entries are rejected. The dependency list is
exact minor-version compatibility identity, while a new major may redesign the
installation graph.

## Compatibility rules now enforced

`BinarySchemaCatalog::check_minor_compatibility` validates both catalogs and
then evaluates every previously published type:

- new type IDs are allowed;
- existing type IDs cannot be removed or change among enum, positional record,
  extensible record, and union;
- positional record fields must remain exact within a major version;
- extensible records keep one exact encoded-size range and every existing field
  tag/contract; a minor may add only optional tags;
- existing enum tags must remain exact, while higher numeric tags may be
  appended;
- existing union variants must remain exact, while higher numeric tags may be
  appended;
- referenced nested type IDs participate in layout fingerprints and therefore
  cannot change silently;
- published command IDs cannot be removed or change method/root-variant
  mappings;
- type versions cannot regress or change major inside a minor compatibility
  check.

This rule is intentionally stricter than tagged-field formats. Pwright's
current positional records have no unknown-field envelope, so appending a field
would make old decoders observe trailing bytes rather than safely skip it.

The generator now has an explicit extensible-record alternative for future
option bags:

```yaml
- kind: record
  id: pwright.example.options
  encoding: extensible
  encoded_size: { minimum: 0, maximum: 256 }
  fields:
    - { tag: 1, name: force, type: bool, cardinality: optional }
```

Its wire body is a strictly increasing sequence of `u16 tag`, `u32
value_bytes`, and value bytes inside one `u32 record_bytes` envelope. Tag zero,
duplicates, descending tags, missing tags, unbounded envelopes, cross-field
`counted_by`, and tag properties on positional fields are rejected. The
generated decoder checks the envelope bound before borrowing any body, requires
known fields to consume their complete slice, and skips unknown fields without
allocation. The generated encoder backpatches lengths into its final buffer.
A standalone generated Rust fixture compiled and passed a round trip with an
injected unknown field. No published locator schema uses this encoding yet.

## Still open before DT-031 is complete

- declarative cross-field constraints; navigation aggregate header bytes and
  canonical ordering currently remain explicit codec policy checks performed
  before generated public-model conversion;
- direct backend dispatch adapters beyond generated typed handler constructors;
- transitive library imports and recursive import graphs; the implemented
  import boundary is deliberately direct and acyclic;
- a second language generator and byte-for-byte cross-language fixtures.
