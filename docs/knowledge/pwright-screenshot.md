<!-- agent-updated: 2026-09-07T10:36:07Z -->

# Playwright screenshot semantics and Pwright boundary

## Node compatibility integration

The Node recipe now selects the existing Rust content capability and returns a
Node-owned Buffer with explicit result-copy admission. Its independent bounded
queue serializes each Page, with eight admitted calls and two simultaneous
captures per host; deadlines include queue wait. An opt-in backend profile
intersects non-full-page clips with the viewport; default 1.0 behavior is intact.
Core byte leases are released at the admitted host-copy handoff before the next
queued capture. Ten live tests compare decoded pixels/dimensions and prove Buffer
survival and cleanup on the handoff-stage artifact; final-artifact verification
is 8/10 on attachment failures. The subsequent trace-only 3/12 run also failed
before either new Worker assertion. After the opt-in recursive-attachment join,
the current artifact passes **12/12**, including four queued native settlements,
exact owned Context removal and published Buffer copies surviving Worker exit.
These are lifecycle/copy proofs, not RSS bounds; rendering transactions remain
unimplemented. [Latest evidence](../node-ffi-compatibility-design.md#node-013-recursive-attachment-prerequisite-join-2026-09-07).
This is not full Playwright screenshot compatibility:
combined fullPage/clip, path, rendering-state options and default font/caret
behavior remain work. See [the exact evidence](../node-ffi-compatibility-design.md#node-page-screenshot-binding-and-ownership-checkpoint-2026-09-07).

## Upstream model

Playwright exposes screenshot capture on `Page` and `Locator`. Page capture can
select PNG or JPEG in the pinned Playwright 1.58.2 API, constrain JPEG quality, capture the viewport or
full document, clip a rectangle, omit the default background, and choose CSS or
device-pixel scaling. Its broader option set can also hide the caret, disable
animations, inject temporary style, and mask locators.

The client never sends a filesystem path to the browser server. The server
returns bytes and the language client optionally writes those bytes to a local
path. Server capture is serialized and the style/mask/animation changes are a
reversible page transaction spanning every affected frame. Chromium ultimately
uses `Page.captureScreenshot`; full-page capture depends on layout metrics and
the returned CDP payload is base64 encoded.

## Pwright 1.0 decision

The first portable capability is `pwright.content` 1.0 with one Page command,
`page.screenshot` (`0x0005_0001`). It is independently installable as
`pwright-cap-content`, `pwright-codec-content`, `pwright-client-content`, and
`pwright-server-content`. Installing navigation, locator, evaluation, or page
events does not install screenshot capture.

Version 1.0 includes the options Chromium can implement without temporarily
mutating page content or rendering state: format, quality, viewport/full-page
selection, clip, and CSS/device scale. The options are an extensible record.
Background omission, mask, style, caret, and animation behavior are deliberately
absent from 1.0 rather than accepted and silently ignored. They require a later
minor that defines a reversible transaction and cleanup under cancellation.
Locator screenshot is also a later command; it must compose locator
actionability and clipping rather than smuggle a selector into the Page
command.

The reusable format, scale, clip, options, and retained-data Rust values live
in the command-free `pwright-screenshot-model` crate. The content capability
reexports the exact types to preserve its public API. Locator capture's
separate capability and cancellation boundary are specified in
[`pwright-locator-screenshot.md`](pwright-locator-screenshot.md).

The wire never contains a server-side path. A client helper may write the
returned bytes locally after the command completes.

## Resource and transport contract

Captured bytes are returned as one binary attachment. The generated control
payload carries only the attachment index, exact byte length, and effective
format. Exactly one attachment is accepted and its declared and actual lengths
must agree.

The Chromium adapter rejects an encoded response whose exact padded-base64
decoded length exceeds its configured maximum before allocating the decoded output. It
reserves `ScreenshotBytes` before base64 decode and keeps that non-cloneable
lease inside `ScreenshotData` for the full result lifetime. Remote encoding
separately reserves `WireBytes`, so the source screenshot and its outbound copy
are both included in the production plan. A client codec similarly copies an
admitted wire attachment into a newly reserved `ScreenshotBytes` result before
the frame can be released.

Capture concurrency is explicitly bounded. Version 1.0 permits one active
capture per Page service and rejects excess work instead of accumulating an
unbounded waiter queue. Dropping or cancelling the command drops intermediate
reservations; CDP has no command-specific screenshot cancellation primitive.

The CDP base64 string is transient transport input. Its maximum is still
bounded by the configured CDP response-frame ceiling; `ScreenshotBytes`
accounts the decoded, user-visible result rather than pretending to account
for the browser connection's wire buffer.

## Validation rules

- `quality` is in `0..=100` and is legal only for JPEG or WebP.
- clip values must be finite, `x` and `y` must be non-negative, and width and
  height must be strictly positive.
- clip and full-page capture are mutually exclusive in 1.0.
- the decoded result must be non-empty and no larger than the configured
  screenshot-byte ceiling.
- attachment count, index, and length mismatches are protocol violations.

These restrictions are part of the advertised 1.0 contract. A future backend
must not claim the capability unless it implements them with the same failure
semantics.

The Chromium conformance test uses the dedicated external CDP profile but
creates its own temporary BrowserContext. It captures a real full-page,
CSS-scale PNG, verifies the PNG header and document-height dimensions, observes
the exact retained/released byte lease, disposes the temporary Context, and
then calls `Browser.getVersion` to prove Pwright did not take browser-process
ownership.

## Compile-time installation boundary

Page screenshot is now a complete isolated Chromium server leaf, not merely a
runtime-selectable handler inside the aggregate. The CDP implementation lives
in `pwright-backend-cdp-content`; `pwright-backend-cdp` reexports it for
existing aggregate users, including Locator screenshot's explicit reuse of the
same capture admission primitive. `pwright-composition-chromium-content` binds
one caller-provisioned Page and injected target-local CDP client, rejects a
different Page before native I/O, and materializes only the generated
`Page.screenshot` handler.

Its production plan includes three potentially overlapping owners: the
backend-returned `ScreenshotBytes`, one separately decoded public-protocol
`ScreenshotBytes` result, and the encoded `WireBytes` frame with its binary
attachment. Backend and codec ceilings must agree, and the outbound frame must
admit one attachment at that ceiling. `examples/v2-content-only` plus its CI
allowlist prove an exact 17-package normal Pwright closure with no DOM content,
public evaluation, Locator screenshot, navigation, aggregate
backend/composition, client, provider, installer, runtime, Node.js, or browser
acquisition dependency.
