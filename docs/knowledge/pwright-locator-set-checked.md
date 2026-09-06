<!-- agent-updated: 2026-09-04T20:35:00Z -->

# Pwright Locator set-checked design

## Decision

Pwright will add one Playwright-shaped `Locator.set_checked` command to both
independently negotiated Locator tracks: locator 1.20 and locator 2.3. Locator
1.19 and 2.2 are frozen before either current schema advances. The command uses
method `131092`, root command/result tags 39/40, and identical public and wire
semantics in both majors.

`check()` and `uncheck()` are client composition over that one command with the
desired state fixed to `true` or `false`. They do not add protocol methods,
backend routes, resource branches, or capability dependencies. SetChecked stays
inside the optional Locator capability because it owns strict lazy selector
resolution and pointer actionability; it does not make the base client,
runtime, transport, or provider depend on Locator, CDP, Node.js, or a browser
distribution.

## Playwright evidence

The reference is Playwright commit
`de214f440b7e34937fe4886f046b78b757136087`:

- client `Locator.check`, `Locator.uncheck`, and `Locator.setChecked` delegate to
  strict Frame check/uncheck calls;
- Frame retries resolution when the target is missing or disconnected;
- ElementHandle reads the current checked state through follow-label
  retargeting and returns immediately when it already matches the request;
- unchecking a radio that is currently checked fails terminally because a radio
  can only be cleared by selecting another radio in its group;
- when a change is required, the operation performs a left pointer click with
  visible, enabled, stable, scroll, position, and hit-target checks unless
  forced;
- that internal click disables navigation waiting; SetChecked does not expose
  `noWaitAfter`;
- trial performs actionability and hit-target work but no pointer click and no
  post-click state assertion;
- after a real click, the checked state is read again and a nonmatching result
  fails terminally instead of reporting a false success.

Pwright will preserve those observable semantics without retaining an
ElementHandle. The Chromium backend may combine strict resolution and state
inspection in bounded utility-world evaluations, but it must not replace the
native pointer click with direct assignment to `checked` or synthetic DOM
events.

## Public and wire contract

```text
SetCheckedParams {
  selector: Selector,
  checked: bool,
  options: SetCheckedOptions,
}

SetCheckedOptions {
  force: Option<bool>,
  position: Option<Position>,
  scroll: Option<ScrollMode>,
  trial: Option<bool>,
}
```

The option record uses the generator's bounded extensible encoding in both new
minor versions. This is deliberate: SetChecked is a new route with no legacy
positional bytes to preserve, and Playwright option presence should remain
distinguishable from explicit `false`. Unknown future fields are skipped within
the record's declared encoded-size bound. Timeout and cancellation stay in
generic call metadata.

`LocatorSetCheckedExt::set_checked` is the primitive extension.
`LocatorCheckExt::check` and `LocatorUncheckExt::uncheck` require only the same
`Service<SetCheckedParams>` implementation and dispatch exactly once. A peer
that selected locator 1.19 or 2.2 rejects all three user-facing operations
before payload encoding or backend work.

## Chromium state machine

The operation reserves the existing Locator selector-action branch and installs
no new long-lived state. Each attempt proceeds as follows:

1. Strictly resolve the current lazy selector and follow an associated label to
   its control for checked-state inspection.
2. Retry only zero matches or a disconnected target. Multiple matches, invalid
   selectors, and non-checkbox/radio targets fail terminally.
3. If current state already equals the requested state, succeed without
   visibility checks, scrolling, pointer input, or navigation observation.
4. If a checked radio is asked to become unchecked, fail terminally before
   actionability or input.
5. Otherwise run the existing Playwright-shaped pointer actionability loop on
   the selected Locator, honoring force, scroll, position, and trial.
6. For trial, return after actionability without sending input or checking a
   changed state.
7. For a real action, acquire the Page-owned input transaction, move to the
   resolved point, send one left-button down/up pair, and always finish cleanup.
8. Re-read checked state through the same strict selector. Disconnection may
   retry the outer operation only when no pointer event was acknowledged. Once
   input is committed, never replay the click; a missing, disconnected,
   malformed, or nonmatching post-state is terminal.

The internal click deliberately has no navigation barrier, matching
Playwright's `waitAfter: disabled`. Cancellation during pointer down uses the
existing Page coordinator cleanup and releases the acknowledged button before
the command terminates. Main Frame, local child Frame, and OOPIF routes share
that same Page owner.

## Resource boundary

SetChecked adds one boolean and a bounded small option record to the existing
selector command. State and actionability evaluations return fixed records.
The operation retains no DOM handle, result string, event queue, map, task, or
navigation cursor. Its input peak is the existing single Page mouse
transaction, and its selector/script peak is no larger than the already
declared Click branch. Production accounting therefore takes the existing
mutually exclusive maxima rather than adding a new concurrent class.

## Acceptance gates

Implementation is complete only when release-profile evidence proves:

1. exact 1.19-to-1.20 and 2.2-to-2.3 schema compatibility, old-minor rejection,
   and byte equality for both current majors;
2. bounded extensible option decoding, including unknown-field skipping,
   nonfinite position rejection, and canonical enum/boolean values;
3. one typed SetChecked backend route with Check and Uncheck composing locally
   into exactly one dispatch each;
4. strictness, follow-label state inspection, already-matching no-op, radio
   uncheck rejection, missing/disconnected retry before commit, and terminal
   post-click mismatch;
5. visible/enabled/stable, scroll, position, hit-target, force, and trial
   behavior without a navigation wait;
6. exact left move/down/up ordering plus failure, dropped-future, and controlled
   cancellation cleanup through the shared Page input owner;
7. typed client/server stream and exact selected-minor routing for both Locator
   majors while v1-only and Locator-absent profiles remain unchanged;
8. dedicated-profile Chrome proves checkbox check/uncheck/idempotence,
   associated-label behavior, radio failure, trial, input events, temporary
   target cleanup, zero pending calls, and preservation of external Chrome;
9. dependency and resource inspection introduces no Node.js runtime, browser
   bundle/downloader, Serde requirement in the portable model, hidden executor,
   new queue, or undeclared production peak.
