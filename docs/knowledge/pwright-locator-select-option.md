<!-- agent-updated: 2026-10-20T23:11:41Z -->

# Pwright Locator select-option design

Implementation checkpoint (2026-10-20): the pinned official Core Chromium
`Page.selectOption` full file passes 32/32 in the frozen candidate and Oracle
lanes with unchanged assertions and an adapted `connectOverCDP` fixture.
The [v12 paired receipt](../../.work/official-conformance/outcomes/core-page-select-option32-strict-paired-v12/promotion-receipt.json)
preserves the v11 navigation-context RED as history and advances only its
one candidate row. The opt-in `select-option-navigation-commit` profile and
Page-binding publication-drain guard address the no-binding path after
selection-triggered navigation. The guard remains globally scoped, so a
binding on a different Page is an open case. This browser-backed result does
not resolve the cross-world `ElementHandle` design decision below or prove
unadapted/full public-API parity.

## Decision

Pwright will add a Playwright-shaped descriptor/string `Locator.select_option`
command to both independently negotiated Locator tracks: locator 1.21 and
locator 2.4. Locator 1.20 and 2.3 are frozen before either current schema
advances. The command uses method `131093`, root command/result tags 41/42,
and identical public and wire semantics in both majors.

The command deliberately does not accept `ElementHandle` values in this
checkpoint. Playwright adopts those handles into the target element's utility
world before comparing DOM nodes. Pwright's current Handle contract binds each
native object to one exact Frame, Page, CDP client, and execution world, while
cross-world `ElementHandle` adoption is still an explicit open compatibility
decision. Hiding adoption inside Locator would couple an otherwise independent
capability to Handle lifecycle/evaluation and would make resource ownership
untruthful. A later optional, separately negotiated Handle-selection command
may compose behind the same Rust-facing API after adoption has its own design,
limits, and disposal proof. String and descriptor selection requires no Handle
plugin.

This extends only the new composable architecture. The existing legacy bridge's value-only
`select_option(&str)` remains preserved during migration, but it is not the
semantic reference: it focuses a resolved node, sets one value, ignores the
returned selection, does not wait for options, and does not enforce
Playwright's actionability and disabled-option behavior.

## Playwright evidence

The reference is Playwright commit
`de214f440b7e34937fe4886f046b78b757136087`:

- client `Locator.selectOption` delegates to strict Frame selection;
- `null` or an empty array means no requested options and deselects everything;
- a string becomes `{ valueOrLabel }`, matching an exact option value or an
  exact/whitespace-normalized label;
- descriptor fields `value`, `label`, and `index` are conjunctive when more
  than one is present; an empty descriptor is valid and matches the first
  option;
- option matching walks DOM order. A non-multiple select chooses the first
  match and returns one value; a multiple select must satisfy every requested
  descriptor and returns selected values in DOM order;
- a missing option, a disabled option, a disabled containing `optgroup`, a
  missing target, or a disconnected target retries until the action deadline;
- strict violations, invalid selectors, and a target other than `<select>` are
  terminal;
- unless forced, the select itself must be visible and enabled. Force skips
  those target checks but does not make a disabled option selectable;
- successful selection first clears the select, sets the matched options, then
  emits bubbling `input` with `composed: true` followed by bubbling `change`
  with the default `composed: false`, even when the requested list is empty;
- the operation returns the selected option values and does not wait for
  navigation caused by those events.

## Public and wire contract

```text
SelectOptionParams {
  selector: Selector,
  values: Vec<SelectOptionValue>,
  options: SelectOptionOptions,
}

SelectOptionValue {
  value_or_label: Option<String>,
  value: Option<String>,
  label: Option<String>,
  index: Option<i32>,
}

SelectOptionOptions {
  force: Option<bool>,
}

SelectOptionResult {
  values: Vec<String>,
}
```

`SelectOptionValue::value_or_label` is the Rust string convenience representation;
the builder also preserves Playwright's multi-field descriptor semantics.
`i32` is intentional: Playwright's protocol accepts any integer, including a
negative one that simply remains unmatched and waits. An empty values vector
preserves Playwright's null/empty deselection behavior without introducing a
Rust null spelling. The option record is a bounded extensible record, so field
absence remains different from explicit `false` and future fields can be
skipped safely. Timeout and cancellation stay in generic call metadata.

The wire permits at most 1,024 descriptors and 1,024 result values. Every
string is at most 64 KiB and each list has a 1 MiB aggregate UTF-8 budget.
Those limits bound decode allocation, generated JavaScript, renderer return
parsing, and retained Rust results independently. The descriptor list is a
message sequence rather than four parallel arrays, preserving conjunctive
matching and allowing later additive descriptor fields without changing list
semantics.

## Chromium state machine

The operation reserves the Locator selector-action branch and installs no new
long-lived state. Each attempt proceeds as follows:

1. Strictly resolve the current lazy selector in the target Frame's routed
   execution world, then follow an associated label to its control.
2. Retry zero matches, disconnection, invisible/disabled target state when not
   forced, missing requested options, and disabled selected options or
   containing optgroups. Multiple matches, invalid selectors, non-elements,
   non-select targets, script exceptions, and malformed results are terminal.
3. Build matches in DOM option order. All fields in one descriptor are
   conjunctive. Remove satisfied descriptors only for a multiple select; for a
   single select stop after the first match.
4. Do not mutate until all required descriptors are present and enabled.
5. Clear selection, mark matched options selected, dispatch `input` then
   `change` with Playwright's exact bubbling/composed flags, and return the
   selected values from the same renderer evaluation.

One renderer evaluation owns match, validation, mutation, events, and result,
so a successful attempt is the commit point. The backend must never retry
after receiving a successful result: event handlers may already have navigated
or replaced the document. Retry statuses are returned only before mutation.
There is no CDP mouse/keyboard transaction and no navigation barrier.

## Capability and resource boundary

SelectOption stays inside optional `pwright.locator`; it adds no dependency on
Handle lifecycle/evaluation, value evaluation, CDP, Node.js, a browser
distribution, or an executor. The portable model remains core-only and the
client extension remains client-plus-Locator only.

The production Locator profile must account for the mutually exclusive
maximum of existing action scripts and one select script containing a bounded
serialized descriptor list. It must also account for the 1 MiB returned-value
aggregate and bounded collection overhead. No DOM node, native object ID,
route, event queue, map, task, or cleanup continuation survives an attempt.
Cancellation before commit merely stops retrying; abandonment during the one
CDP evaluation has no separately recoverable native resource. A late success
must not be replayed.

## Acceptance gates

Implementation is complete only when release-profile evidence proves:

1. exact 1.20-to-1.21 and 2.3-to-2.4 schema compatibility, old-minor rejection,
   and deterministic generated output for both current majors;
2. bounded option/result collections, per-string and aggregate limits,
   extensible option decoding, and signed index preservation;
3. typed client/server routing with string convenience mapping to
   `value_or_label` and no mandatory Handle dependency;
4. strictness, label retargeting, force, target visibility/enabled checks,
   missing/disconnected/option-not-found/option-disabled retries, and terminal
   invalid-target behavior;
5. exact value/label/normalized-label/index/conjunctive matching, first-match
   single-select behavior, all-match multiple-select behavior, and empty-list
   deselection;
6. exact returned values plus `input`/`change` ordering and composed flags,
   with no post-commit replay or navigation wait;
7. selected-minor routing for both Locator majors while older and
   Locator-absent profiles remain unchanged;
8. dedicated Chrome proves descriptor forms, delayed option availability,
   disabled option recovery, multiple selection, deselection, events,
   temporary target cleanup, zero pending calls, and preservation of external
   Chrome;
9. dependency and resource inspection introduces no Node.js runtime, browser
   bundle/downloader, Serde requirement in the portable model, hidden executor,
   persistent handle, undeclared queue, or additive production peak.

The `ElementHandle` overload is not counted as complete by these gates. It
remains explicitly tracked behind the separate cross-world adoption decision;
the public documentation must not claim full Playwright `selectOption` parity
until that optional extension is implemented and negotiated.
