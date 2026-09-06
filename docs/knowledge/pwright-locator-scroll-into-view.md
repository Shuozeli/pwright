<!-- agent-updated: 2026-09-05T15:03:54Z -->

# Pwright Locator scroll-into-view design

## Decision

Pwright will add `Locator.scroll_into_view_if_needed()` to both independently
negotiated Locator tracks as locator 1.22 and locator 2.5. Locator 1.21 and 2.4
are frozen first. The command uses method `131094`, root command/result tags
43/44, the existing strict lazy `Selector`, call metadata for timeout and
cancellation, and the existing unit action result.

This is one Locator capability command, not a DOM, input, navigation, Handle,
or screenshot capability dependency. It is a prerequisite for later
element-screenshot and pointer-action work, but installing it does not install
those features.

## Playwright evidence

The reference is Playwright commit
`de214f440b7e34937fe4886f046b78b757136087`:

- client `Locator.scrollIntoViewIfNeeded` resolves a strict Locator through
  `_withElement` and delegates to the ElementHandle operation;
- the server waits for stability only; it does not require visible, enabled,
  editable, or hit-target state;
- Chromium delegates the final operation to `DOM.scrollIntoViewIfNeeded`;
- a node without a layout object and a disconnected node are retryable;
- strict violations, invalid selectors, non-elements, script failures, and
  malformed backend results are terminal;
- the operation returns unit and has no navigation barrier or input event;
- the public option surface contains cancellation and timeout only, which
  Pwright already carries in `CallMetadata`.

Playwright documents “needed” using an `IntersectionObserver` ratio: an element
that is already completely visible must not be repositioned. Its injected
runtime also schedules a frame so engines deliver the observer callback.

## Public and wire contract

```text
ScrollIntoViewIfNeededParams {
  selector: Selector,
}

ScrollIntoViewIfNeededResult = ()
```

There is deliberately no empty options record. Abort/deadline policy remains
generic call metadata, and there is no rectangle argument on Locator's public
API. Adding an element-relative rectangle would be a different operation and
cannot silently mutate this wire shape.

## Chromium state machine

One target-local utility-world precommit evaluation owns each wait attempt:

1. Strictly resolve the current lazy selector. Missing or disconnected targets
   retry; multiple matches and invalid selectors fail terminally.
2. Reject a non-Element target. A node without a layout box retries.
3. Read its bounding rectangle, wait two animation frames, and compare position
   and size with the existing quarter-CSS-pixel stability tolerance. An
   unstable or newly disconnected target retries.
4. Measure the intersection ratio. If it is exactly one, return success without
   mutating scroll position. Otherwise return a fixed `ready-to-scroll` status.
5. Drop the precommit script, then issue one synchronous utility-world commit
   evaluation. It strictly re-resolves the selector, rechecks connection and
   layout, calls `element.scrollIntoView` with instant centered block/inline
   alignment, and returns success. A precommit race retries from step 1.

The JavaScript DOM scroll is an intentional Chromium adapter choice. Both
phases keep the selector in the exact execution world and create no retained
CDP RemoteObject. Separating the async wait from the synchronous commit is
necessary because dropping a Rust CDP future does not cancel JavaScript that
is already awaiting animation frames: a one-evaluation design could scroll
after caller cancellation. Using `DOM.scrollIntoViewIfNeeded` would require
publishing a temporary object ID across two CDP calls plus cancellation-safe
release ownership. That extra Handle-like lifecycle is not justified for an
operation whose portable observable contract is only “scroll if needed”. A
backend conformance test, rather than the CDP method name, is the acceptance
boundary.

Dispatch of the synchronous commit evaluation is the commit point.
Cancellation before it leaves no browser mutation; cancellation after dispatch
cannot reverse scrolling and must not replay the operation. No input-owner
cleanup or navigation wait is registered.

## Resource boundary

The command adds only the selector already bounded by Locator and one
fixed-size generated script at a time, each smaller than the current
selector-action script maximum. It
uses the existing selector-action operation lease, retry delay, caller
deadline, utility-world runtime, and Page/Frame route. It retains no DOM/CDP
handle, result string, queue, collection, pointer state, keyboard state,
navigation cursor, or background task. Production resource requirements remain
the maximum of mutually exclusive Locator branches rather than an additive
peak.

## Compatibility and acceptance gates

The checkpoint is complete only when all of the following hold:

1. frozen 1.21 and 2.4 schemas reject the new command while current 1.22 and
   2.5 schemas generate identical command bytes across majors;
2. capability, codec, client, server, protocol reference, and selected-major
   composition are generated or typed without changing earlier command bytes;
3. tests cover missing, disconnected, unstable, no-layout, already-visible,
   scrolled, strict, invalid-target, malformed-result, runtime-reinstall, and
   cancellation behavior;
4. the old-minor gate rejects before backend I/O;
5. a real remote stream round-trips the typed command through a generated
   server handler;
6. production tests prove no new retained resource class or additive peak and
   v2-off rollback remains unchanged;
7. dedicated Chrome verifies a below-the-fold element moves into the viewport,
   an already-completely-visible element is not repositioned, and no input or
   navigation is dispatched;
8. generation drift, formatting, release tests, strict Clippy, dependency
   inspection, and external-browser preservation all pass.

## Separate Browser.version design boundary

Playwright reads Chromium `Browser.getVersion` once during connection and
publishes the parsed version in the Browser initializer; its client method is a
synchronous cached getter. Pwright control 1.1 currently publishes only object
identity, parent, and kind. Therefore a per-call `browser.version` RPC would be
a behavioral shortcut, while adding optional metadata directly to control
would couple every installation to one capability. Browser metadata stays open
until Pwright defines a bounded, capability-owned initializer-extension and
publication barrier. This ScrollIntoView checkpoint does not disguise or solve
that separate design task.
