<!-- agent-updated: 2026-09-04T18:17:33Z -->

# Pwright Locator focus and blur design

## Decision

Pwright adds `Locator.focus` and `Locator.blur` together as the additive
`pwright.locator` 1.18 and 2.1 surfaces. They are one coherent focus-state
checkpoint, but remain absent when Locator is not installed, when a 1.x peer
negotiates 1.17 or earlier, or when a 2.x peer negotiates 2.0.

Both commands target Page or Frame through the existing lazy Locator selector.
Their only explicit parameter is that selector; timeout and cancellation remain
generic command metadata owned by the runtime.

## Playwright evidence

At Playwright commit `de214f440b7e34937fe4886f046b78b757136087`,
`packages/playwright-core/src/client/locator.ts` sends both operations with
`strict: true`. `packages/playwright-core/src/server/frames.ts` retries only
when the selected node is missing or disconnected, then delegates to
`ElementHandle._focus` or `_blur`. `packages/injected/src/injectedScript.ts`
proves the browser-side semantics:

- both operations reject a connected non-element node;
- focus calls the element's native `focus()` twice for Firefox compatibility;
- blur calls the element's native `blur()` once;
- neither operation waits for visibility, stability, enabled, editable, or a
  hit target;
- focusing a non-focusable element may succeed without changing
  `document.activeElement`;
- blurring an element that is not focused is still a successful no-op;
- focus and blur events are browser-native consequences, not synthetic events.

Pwright will preserve these semantics. It will not scroll, click, dispatch CDP
keyboard or mouse input, verify the final active element, or retain an
ElementHandle.

## Portable and wire contract

Locator 1.18 adds two append-only commands after 1.17:

| Command | Method | Root tags | Result |
|---|---:|---:|---|
| `locator.focus` | `131089` | `33/34` | unit |
| `locator.blur` | `131090` | `35/36` | unit |

Each command has a positional `selector` parameter and reuses the existing
action result and typed Locator error details. The 1.17 schema is frozen before
the version advances. A 1.17 negotiated codec must reject both routes before
encoding or backend dispatch.

The independent 2.x schema freezes 2.0, then appends the same commands, method
IDs, tags, parameters, and outcomes in 2.1. It does not rewrite the 2.0 Click
option record or any 1.x bytes. A selected 2.0 codec rejects both routes before
payload encoding; a selected 2.1 codec delegates to the already verified
canonical Focus/Blur wire representation.

The typed client extension adds `LocatorFocusExt::focus()` and
`LocatorBlurExt::blur()` only in the optional `pwright-client-locator` crate.
The base client remains unaware of Locator.

## Chromium execution

The existing target-local utility-world route strictly re-resolves the selector
on every attempt. Zero matches or a disconnected result retry under the one
runtime-owned deadline. Multiple matches fail immediately with the typed strict
violation. Invalid selectors, non-elements, script exceptions, and malformed
CDP results are terminal.

Focus invokes native `focus()` twice; blur invokes native `blur()` once. The
script returns only a bounded status record and retains no browser object. A
missing selector runtime after navigation triggers the existing one-at-a-time
reinstallation path.

Both operations share the existing Locator concurrency and `SelectorBytes`
budget. Their generated script is smaller than the already budgeted attribute
and fill branches, so 1.18 must not add a second mutually exclusive peak.
Cancellation owns no browser-side cleanup beyond dropping the pending
evaluation; no input transaction is acquired.

## Acceptance gates

This focus-management checkpoint is complete only when release-profile
evidence proves:

1. schema compatibility against an exact frozen 1.17 baseline and 1.17 codec
   rejection of both new methods;
2. generated capability/server output, binary codec round trips, and typed
   client dispatch for both Page- and Frame-owned Locators;
3. missing/detached retry, strict failure, invalid target, malformed result,
   runtime reinstallation, deadline cancellation, and unchanged resource peak;
4. exact utility-world routing without mouse or keyboard CDP calls;
5. real Chrome focus/blur events and `activeElement` transitions, plus the
   successful non-focusable and already-blurred cases;
6. cleanup of the exact temporary target while the externally owned browser
   remains reachable.
7. the v2.1 typed client/server stream carries both commands, the exact
   negotiated minor reaches the selected v2 codec, and 2.0 rejects them.
