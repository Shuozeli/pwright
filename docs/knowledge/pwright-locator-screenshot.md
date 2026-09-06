<!-- agent-updated: 2026-09-05T13:34:54Z -->

# Playwright Locator screenshot composition

## Upstream execution shape

Playwright's public `Locator.screenshot()` does not turn the Locator into a
long-lived remote DOM object. The client resolves the strict Locator in the
`attached` state, receives a temporary `ElementHandle`, runs
`ElementHandle.screenshot()`, and disposes the handle in `finally`.

The server-side element capture is more than `Page.captureScreenshot` with a
selector-shaped clip. It serializes captures per Page, prepares reversible
rendering state, waits for the element to be visible and stable, scrolls it
into view if needed, reads its post-scroll bounding box, converts that box to
document coordinates, captures the enclosing integer rectangle, and restores
the temporary rendering state. A missing layout box, zero width, or zero
height is an error. The filesystem path is still client-local and never
crosses the browser-server protocol.

Relevant upstream sources in the pinned Playwright checkout are:

- `packages/playwright-core/src/client/locator.ts`: `_withElement` and
  `Locator.screenshot`;
- `packages/playwright-core/src/server/dom.ts`: `ElementHandle.screenshot`;
- `packages/playwright-core/src/server/screenshotter.ts`:
  `screenshotElement`, the Page capture queue, and reversible preparation.

## Pwright capability decision

Locator capture must not be appended to either existing capability:

- adding it to `pwright.locator` would make query/action users install pixel
  capture, binary attachment, and screenshot-byte resources;
- adding it to `pwright.content` would make Page-only screenshot users install
  selector resolution and Locator behavior.

The intended public surface is therefore a separate optional
`pwright.locator_screenshot` capability. Its typed client extension is exposed
on `Locator`, but the wire command targets the Locator's Page or Frame scope and
carries the immutable selector value. The implementation injects a
selector-to-geometry resolver and reuses the independent Page screenshot
coordinator without installing either the ordinary Locator command set or the
`page.screenshot` route. The complete aggregate adapts its existing Locator
service into that resolver, while a minimal consumer can install a narrower
selector plugin.

This capability is a composition unit, not a new resource class. Successful
results retain the existing `ScreenshotBytes` class in the same non-cloneable
`ScreenshotData` value, and remote transfer uses one independently admitted
`WireBytes` attachment. The command gets its own feature descriptor, method
tag, codec, generated server binding, backend service, manifest entry, and
profile switch.

## Why not compose through a public temporary handle

The JavaScript client can reliably run `finally` after awaiting the screenshot,
but dropping a Rust future cannot perform async handle disposal. A two-command
client composition (`resolve handle`, then `element screenshot`) could leak the
temporary remote handle when its future is dropped unless the server adds a
call-scoped child-resource protocol.

The first Locator screenshot version therefore uses one server command and no
published handle. Resolution, stability, scrolling, geometry, capture, and
cleanup all stay inside the command's cancellation boundary. A separately
installable public `ElementHandle.screenshot` capability can be added later for
handles the user already owns; it is not the hidden implementation contract of
`Locator.screenshot`.

## Portable model boundary

Screenshot result and option values are not commands and do not belong to the
Page screenshot capability. `pwright-screenshot-model` owns the portable
format, scale, clip, options, and retained-data values. It depends only on
`pwright-core` for `BudgetedBytes` and has no protocol, transport, runtime,
backend, browser, filesystem, or Node.js dependency. `pwright-cap-content`
reexports the exact types so the existing Page screenshot API and downstream
source code remain compatible.

The immutable selector value now lives behind the zero-dependency
`pwright-selector-model` boundary and `pwright-cap-locator` reexports the exact
types for compatibility. The recursive selector wire types are now also
published as the command-free, versioned `pwright.selector` 1.0 schema library
and generated `pwright-codec-selector` crate. Definition-level tests keep it
exactly aligned with both published Locator tracks. The new capability can
import that library instead of copying the tree or importing Locator commands.

## Version-one semantic scope

The first Locator capture surface should include the options already safe in
the Page capture backend: PNG/JPEG/WebP, JPEG/WebP quality, and CSS/device
scale. `full_page` and caller clips are Page-only and invalid for Locator
capture. Path remains a client helper concern.

Omitted background, caret, animation, injected style, and masks require the
same cancellation-safe, all-frames reversible transaction that Page screenshot
will need. They must be added to both APIs from one shared transaction design,
not implemented with Locator-only mutations or silently ignored.

## Backend transaction and cancellation

The backend command has these ordered phases:

1. acquire the Page capture admission slot and maximum screenshot-byte lease;
2. strictly resolve one connected Element in the exact Frame utility world;
3. wait for visible and two-animation-frame stable geometry under the caller
   deadline;
4. scroll only when it is not fully visible;
5. re-resolve and synchronously read the final connected layout rectangle;
6. reject empty or non-finite geometry and compute the enclosing integer clip;
7. issue the browser screenshot capture, validate/decode the bounded bytes,
   shrink the lease to exact size, and return `ScreenshotData`;
8. release all admission and temporary state on success, error, cancellation,
   or future drop.

The scroll dispatch and screenshot dispatch are observable commit points. A
future dropped before scroll must not scroll later. A future dropped after CDP
capture dispatch cannot cancel Chromium's command, but it must release Pwright
admission and must not publish late bytes or retain a DOM handle.

## Delivery tasks

1. Completed: extract and verify the command-free screenshot model without
   changing the existing `pwright.content` 1.0 wire or API.
2. Completed: extract `pwright-selector-model`, preserve exact
   `pwright-cap-locator` reexports, and publish the matching recursive wire
   closure as the command-free `pwright.selector` 1.0 schema library without
   changing existing v1/v2 bytes.
3. Completed: add the `pwright.locator_screenshot` 1.0 schema and frozen
   baseline, core-only capability, bounded attachment codec, typed Locator
   client extension, and generated one-command server binding. The schema
   imports `pwright.selector`; its manifest advertises neither
   `pwright.locator` nor `pwright.content`.
4. Completed: factor command-free `pwright-codec-screenshot`; Page and Locator
   capture now share exact attachment index/count/length/limit validation and
   `ScreenshotBytes` result-copy accounting without sharing commands.
5. Completed: implement the bounded Chromium transaction and independently
   selected profile routing. The Page capture slot is acquired before strict
   resolution and remains owned through stability, optional scroll,
   post-scroll geometry, capture, decode, and exact-byte shrink.
6. Completed: prove direct and negotiated remote routing, profile isolation,
   strict failure, scroll/revalidation geometry, future-drop cleanup, and
   retained-byte lifetime with AAA tests.
7. Completed: run a dedicated Chrome proof for already-visible and below-fold
   elements. It verified exact 120x80 and 90x60 CSS-scale PNG dimensions,
   scrolling, zero retained screenshot bytes after drop, temporary Context and
   target cleanup, and continued `Browser.getVersion` reachability.
8. Completed: add Playwright-aligned child-Frame translation. The backend uses
   `DOM.getFrameOwner` and the owner's native border box for every ancestor,
   while the composition builds a shared linked translation chain from the
   bounded Frame topology. Unit proofs cover a local child and a nested OOPIF:
   selector work stays on the child renderer session, owner lookup runs on each
   parent session, and final capture stays on the Page session. A dedicated
   Chrome same-process iframe proof produced the exact 70x50 CSS-scale PNG,
   released its byte lease, cleaned its temporary Context, and left the
   externally owned browser reachable.
9. Completed: extract `pwright-backend-cdp-locator-screenshot` and
   `pwright-composition-chromium-locator-screenshot`. The capture leaf now
   depends on the command-free `LocatorScreenshotGeometryResolver` plugin
   boundary rather than `pwright-cap-locator` or the aggregate CDP backend.
   The full backend supplies a compatibility adapter. At that checkpoint,
   standalone and umbrella Cargo proofs enforced the exact 21-package
   resolver-injected orchestration closure and rejected public Locator command
   authority.
10. Completed: extract the command-free selector compiler/runtime into
    `pwright-backend-cdp-selector-runtime`, then add the independently
    installable `pwright-backend-cdp-selector-geometry` first-party plugin.
    The complete Locator backend and the small plugin now compile selectors
    against the same runtime identity and limits. The plugin owns only strict
    resolution, actionability retries, an explicit scroll commit, validated
    viewport geometry, runtime recovery, and its declared selector/context
    resources. The runnable standalone profile combines capture and selector
    requirements and has an exact 23-package closure that still excludes
    public Locator commands and the aggregate backend. Its dedicated Chrome
    test resolved exact 120x80 visible geometry, scrolled to and resolved a
    90x60 below-fold element, installed the shared runtime once, disposed the
    temporary Context, and left the externally owned browser reachable.

Cross-site OOPIF live coverage and decoded pixel-fidelity assertions remain
conformance follow-ups; no child-Frame path now substitutes an untranslated
Frame-local rectangle.

## Acceptance gates

- A Page-only screenshot profile contains no Locator command/backend route.
- A Locator-only profile contains no screenshot codec, attachment, or byte
  reservation.
- A Locator-screenshot profile need not advertise `page.screenshot`.
- A Locator-screenshot capture leaf does not depend on `pwright-cap-locator`;
  selector semantics and resources belong to its explicitly injected plugin.
- The capture composition remains resolver-agnostic. The umbrella convenience
  feature selects the first-party Chromium plugin, while direct consumers may
  inject another bounded resolver without linking it.
- No Node.js, Playwright package, browser installer, or bundled browser enters
  the portable or Chromium dependency trees.
- No selector wire model is copied into two independently evolving schemas.
- Cancellation and every error path leave capture slots, byte leases, and DOM
  handles at their pre-call counts.
