<!-- agent-updated: 2026-09-08T01:19:49Z -->

# Pwright Locator press design

NODE-050 passes native local/OOPIF pre-focus detach and Page/Context close during
the post-focus key delay. All 18 Frame cases pass. Selected Node drag/Locator
calls also pass the shared ten-case corpus; public child-Frame error details and
native cancellation interrupted by destruction are not yet proven.
[Selection, evidence and remaining work](pwright-chromium-drag.md#node-050-native-lifetime-gates-and-selected-node-addon).

NODE-049 implements optional post-focus lifetime handoff while retaining the
original Frame metadata/permits/deadline. Navigation preparation accepts a
parented OOPIF session root; the epilogue uses the original Page keyboard client.
Local/OOPIF detach now matches both timeout and finite successful completion.
[Contract, evidence and remaining gates](pwright-chromium-drag.md#node-049-optional-frame-to-page-lifetime-handoff).

NODE-048 verifies eight real local/OOPIF input cases but exposes two detach
failures. Successful Frame focus must hand input lifetime to Page: removing the
iframe during key delay leaves upstream Page press pending, while Rust currently
returns TargetClosed. Do not claim lifetime parity from NODE-047 alone.
[Required handoff contract](pwright-chromium-drag.md#next-implementation-explicit-frame-to-page-lifetime-transition).

NODE-047 selects NODE-046's transaction-local interceptor for Locator press and
sequential typing in the opt-in Rust host. Frame utility focus and Page keyboard
retain distinct original native clients; cancellation retains the exact binding.
Ten native host cases include Locator Escape, chord and delayed-press timeout.
Real child/OOPIF and native interruption gates remain; addon selection is unchanged.
[Integration boundary](pwright-chromium-drag.md#node-047-locator-keyboard-selects-the-owning-page).

## Implementation status

Node now opts into interleaved Page input rather than default Rust whole-action
exclusion. Its new artifact passes 27/27 action tests, including the previously
failing NODE-022 delayed-Press Shift case. Page keys survive timeout; literal
aliases (`a`, `1`) remain literal while physical codes (`KeyA`, `Digit1`) follow
held Shift. Default exclusive cancellation recovery is unchanged. Later failure
paragraphs below describe historical artifacts; broader native cancellation and
modifier changes between down/up remain unverified, not a full parity claim.

The shared input leaf now also implements caller-owned sequential typing and
corrects temporary modifier aliases to emit left-key location on down/up. Its
13-case exact Chrome event/value comparison passes, with 103 Locator regressions.
Public Locator typing is now bound through schema 1.23/2.6 and Node FFI. It reuses Press's focus primitive,
but **not** its navigation barrier or whole-string chord parser; see the
[sequential-typing contract](pwright-sequential-typing.md).

The typed 1.19/2.2 vertical slice is implemented through schema, portable
values, client extensions, bounded codecs, generated server chains, Chromium
target routing, static US-layout resolution, rich CDP dispatch, Page-owned
pressed-key cleanup, navigation ordering, and production composition. Release
tests cover both majors, old-minor rejection, exact cross-major Press bytes,
selection-reset focus, modifier/text fields, repeat, reverse cleanup, unknown
keys before browser I/O, and selected-v2 dispatch through `CdpPageServices`.
Dedicated-profile Chrome 152 conformance additionally proves selection-sensitive
Arrow input, printable and shifted text, `ControlOrMeta` select-all, exact DOM
key events, default Enter/form navigation waiting, controlled cancellation
during a delayed chord, reverse key release, zero pending calls, temporary-target
cleanup, and preservation of the externally owned browser. The Press checkpoint
is therefore release-complete within the currently designed scope.

The later Node-FFI action comparison exposes two broader compatibility gaps
outside that original checkpoint. NODE-021 observes a missing utility-world
read route immediately after Enter/form navigation; commit and world readiness
must not be conflated. NODE-022 observes pinned Playwright 1.58.2 retaining Shift
after a delayed chord times out, while the Rust cleanup contract releases it.
The ordinary paired tests remain required and failing; the original release
claim does not establish full Node or timeout-state parity. See
[the Node discrepancy register](../node-ffi-compatibility-design.md).

## Decision

Pwright will add Playwright-shaped `Locator.press` to both independently
negotiated Locator tracks in one parity checkpoint: locator 1.19 and locator
2.2. The 1.18 and 2.1 sources must be frozen before either current schema is
advanced. Both new commands use method `131091`, root command/result tags
37/38, and the same portable parameters and result.

Press remains part of the optional Locator capability. It does not make the
base client, runtime, transport, or browser provider depend on keyboard input,
Node.js, a browser download, or CDP. A separate LocatorPress capability was
rejected for this checkpoint because its selector wire model would either be
duplicated or would require a cross-major dependency on exactly one Locator
track. A future Page Keyboard capability may reuse the input model and backend
state without changing the Locator command.

## Playwright evidence

The reference is Playwright commit
`de214f440b7e34937fe4886f046b78b757136087`:

- client `Locator.press` sends Frame.press with `strict: true`;
- Frame retries only a missing or disconnected selected element;
- ElementHandle focuses with `resetSelectionIfNotFocused = true`, then invokes
  the Page-owned Keyboard while observing navigation signals unless
  `noWaitAfter` was requested;
- focus calls native `focus()` twice and, only when a previously unfocused
  `input` becomes focused, attempts `setSelectionRange(0, 0)`;
- the key string is split at non-leading `+` characters, chord prefixes go
  down in order, the final key goes down, optional delay elapses, the final key
  goes up, and prefixes go up in reverse order;
- Keyboard owns pressed modifier and physical-code state, so repeated down
  events set `autoRepeat`;
- Chromium dispatch distinguishes `keyDown` from `rawKeyDown` and sends the
  modifier mask, Windows virtual key code, code, key, text, unmodified text,
  repeat, location, keypad flag, and macOS editing commands.

The portable contract follows those semantics. It does not approximate Press
as DOM `KeyboardEvent` dispatch, JavaScript value mutation, Fill, or one
hard-coded Enter key.

## Public and wire contract

The client extension remains `LocatorPressExt::press`. Its values are:

```text
PressParams {
  selector: Selector,
  key: PressKey,
  options: PressOptions,
}

PressOptions {
  delay: Option<Duration>,
  no_wait_after: bool,
}
```

`PressKey` is a validated, owned Playwright key expression with at most 128
UTF-8 bytes and 16 tokens. Validation accepts names, physical codes, printable
US-layout characters, aliases, and the platform-smart `ControlOrMeta` token.
It preserves Playwright's plus parsing, including `+` as a key and `Shift++`
as a chord. Empty tokens, unknown keys, excessive tokens, and excessive bytes
fail as typed invalid values before browser I/O. The 16-token ceiling is an
explicit bounded-work extension; ordinary chords need at most four modifiers
and one final key.

Delay is between the final key down and key up. Zero behaves as no delay.
Timeout and cancellation stay in generic call metadata. `no_wait_after`
controls only the Page navigation-signal barrier; it does not skip focus, key
cleanup, or command completion. The result is the existing unit Locator action
result and failures reuse the bounded Locator error details.

The complete 1.18 and 2.1 schemas are frozen independently. Compatibility
checks must prove that 1.19 and 2.2 append the new route without changing any
existing selector, option, command, result, or error bytes. A connection that
selects 1.18 or 2.1 rejects Press before payload encoding or backend dispatch.

## Shared keyboard model

Key parsing and US-layout resolution are pure, synchronous, bounded code with
no executor or browser dependency. The resolved descriptor contains static
key/code/text data, virtual key codes, location, keypad status, and the shifted
variant. `ControlOrMeta` resolves to Meta on macOS and Control elsewhere before
layout lookup. Text is suppressed whenever a non-Shift modifier is active.

The full Playwright US keyboard table, aliases, shifted variants, and Chromium
macOS editing-command mapping are ported as reviewed static data with fixture
tests against the pinned upstream commit. They must not be loaded from JSON,
generated at runtime, or stored in a hash map. Resolution may use a static
match or sorted static slice and performs no per-process heap initialization.

This model should be placed behind a small reusable crate or module boundary;
it must not expose CDP types. The Chromium adapter converts one resolved key
event into a richer `pwright-cdp` input parameter record. Existing narrow CDP
helpers remain for compatibility, but Press must not grow another long
positional method signature.

## Page-owned input transaction

`CdpInputCoordinator` remains the single Page owner shared by its main Frame,
local child Frames, and OOPIF routes. It gains a fixed-capacity pressed-key
record of 16 physical codes in addition to its existing modifier and mouse
state. There is no map, queue, retained selector, DOM handle, or task.

One Press attempt holds the Page input gate across final focus and all key
transitions. Every acknowledged key down is recorded immediately. Every
acknowledged key up is removed immediately. Cleanup releases remaining keys in
reverse order, then repairs the existing mouse/modifier transaction state. A
dropped delayed Press, runtime cancellation, CDP failure, or failed key release
therefore cannot silently leave a key pressed. If cleanup itself fails, the
typed terminal error remains visible and the next input transaction retries
the bounded repair before doing new work.

Modifier state changes follow acknowledged events, not requested events.
Repeated downs use the fixed pressed-code record to set `autoRepeat`. Modifier
masks describe state after key down and after removal for key up, matching
Playwright. The final key is released before chord prefixes, whose release
order is reversed.

## Locator execution and navigation

Each retry strictly resolves the lazy selector in its current utility world.
Zero matches or a disconnected node may retry under the one runtime deadline;
multiple matches, invalid selectors, non-elements, and malformed script
results fail terminally. The final attempt focuses twice and resets a newly
focused input selection to zero before the first key event.

Once any key event is acknowledged, the Locator attempt is committed and must
never re-resolve or retry the selector, because replay could duplicate input.
The command then either completes, cleans up and fails, or is cancelled and
cleaned up. It retains no ElementHandle.

The default path reuses the existing Page navigation ordering barrier already
used by v2 Click: Page lifecycle observation is enabled before the committed
input sequence and the result waits for a navigation signal created by that
sequence. `no_wait_after = true` omits this wait. Press does not create another
CDP reader or lifecycle queue.

## Resource boundary

The wire adds one selector, one bounded 128-byte key expression, a small option
record, and no attachment. Parsing is linear in at most 128 bytes. At most 16
resolved descriptors live on the command stack, and the Page coordinator adds
one fixed 16-slot pressed-key record. Event dispatch is sequential, so token
count bounds work without multiplying in-flight futures or payload retention.

Delay uses the runtime's existing deadline-aware scheduler and owns no task.
Navigation waiting shares the existing bounded Page event cursor. The static
layout and macOS command tables add code/read-only data only when the Chromium
Locator Press implementation is linked; they add no runtime heap peak.

## Acceptance gates

Implementation is complete only when release-profile evidence proves:

1. exact 1.18-to-1.19 and 2.1-to-2.2 schema compatibility, old-minor rejection,
   and byte equality for the two current majors;
2. bounded parsing of names, physical codes, printable characters, `+`,
   `Shift++`, aliases, smart modifiers, unknown keys, and token/byte overflow;
3. exact key down/up order, shifted text, raw versus text keydown, modifier
   masks, virtual codes, repeat, location/keypad data, and macOS commands;
4. strict Page/Frame Locator routing, missing/disconnected retry only before
   input commit, selection reset, and no actionability or pointer work;
5. cancellation during modifier down, main-key down, delay, key up, and
   navigation waiting releases every acknowledged key in reverse order;
6. Click/Hover and Press serialize through the same Page owner across main,
   local-frame, and OOPIF routes without increasing an undeclared peak;
7. typed client/server stream and exact selected-minor routing work for both
   Locator majors while v1-only and Locator-absent profiles remain unchanged;
8. dedicated-profile Chrome proves text input, Enter/form navigation,
   selection-sensitive Arrow/Home behavior, shortcuts, events, cancellation
   cleanup, temporary-target cleanup, and preservation of external Chrome;
9. release dependency inspection contains no Node.js runtime, browser bundle,
   downloader, Serde requirement in the portable model, or hidden executor.
