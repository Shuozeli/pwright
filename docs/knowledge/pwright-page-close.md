<!-- agent-updated: 2026-09-03T08:42:14Z -->

# Playwright-aligned Page close design

> Status: implemented experimental v2 checkpoint; it is not yet a stabilized
> compatibility promise.

## Upstream behavior being matched

The source baseline is Playwright commit
`de214f440b7e34937fe4886f046b78b757136087`. Its public
`Page.close({ runBeforeUnload, reason })` deliberately maps to two protocol
operations:

- ordinary close sends protocol `Page.close`, suppresses a racing
  target-closed error, and waits for the server Page's closed promise;
- `runBeforeUnload: true` sends protocol `Page.runBeforeUnload`, does not wait
  for Page closure, and does not suppress a racing target-closed error.

On Chromium, ordinary close uses browser-session `Target.closeTarget`; the
before-unload form uses page-session `Page.close`. The latter may leave the
Page open while a before-unload dialog awaits a user decision. Relevant
anchors are:

- `packages/playwright-core/src/client/page.ts` (`Page.close`);
- `packages/playwright-core/src/server/page.ts` (`close`, `runBeforeUnload`);
- `packages/playwright-core/src/server/chromium/crPage.ts` (`closePage`);
- `packages/playwright-core/src/server/chromium/crBrowser.ts` (`_closePage`);
- `packages/protocol/spec/page.yml` (`close`, `runBeforeUnload`).

## Capability boundary

Pwright adds a separate optional `pwright.page_lifecycle` capability. It
does not become part of `pwright-client`, navigation, locators, browser-context
lifecycle, CDP, or the runtime by default. Version 1.0 owns two typed commands:

1. `page.close`: ordinary close with termination semantics;
2. `page.run_before_unload`: initiation-only before-unload semantics.

Using two commands, as Playwright's own channel does, prevents one boolean from
hiding two incompatible completion contracts. Both target only Page and carry
no action timeout or cancellation metadata. The first slice exposes an
argument-free ordinary close and an explicit before-unload method. Playwright's
optional close `reason`, and a convenience options facade that selects between
the commands, are a later additive contract after close-reason propagation to
interrupted operations is designed end to end.

Ordinary close is idempotent at the client boundary: a locally stale Page or a
racing `TargetClosed` result is success. Before-unload initiation is not
idempotent in this way because reaching the live Page is itself the requested
effect.

## Chromium ownership and completion

Ordinary Chromium Page close is a four-owner transaction:

```text
prepare topology Page removal
  -> prepare runtime Page subtree disposal
  -> Target.closeTarget(exact target)
  -> confirm exact target is absent
  -> commit topology and Page-session route table together
  -> commit runtime child-first disposal
  -> publish every Dispose through Page
  -> publish Result
```

`Target.closeTarget` acknowledgement alone is not termination proof. The CDP
adapter must confirm that the exact target no longer appears before local
commit. Confirmation/retry timing is injected and bounded; the core service
does not read a clock, sleep, spawn, or choose Tokio. Failure leaves both local
owners live so the caller can retry or reconcile.

The ordinary close future uses the same synchronously reserved, caller-owned
continuation rule as BrowserContext close. Once native work starts, dropping
the caller future transfers the exact transaction to that bounded owner.
Remote completion uses the same priority control barrier so Frame and Page
Dispose messages are applied by the client before `close().await` wakes.

Before-unload initiation is intentionally different:

```text
Page.close on the exact page session
  -> return initiation Result
  -> keep Page and descendants live
  -> later dialog/target events decide whether disposal occurs
```

It does not prepare or commit local disposal. A future dialog capability will
own accept/dismiss; dismissing before-unload may preserve the Page. This avoids
falsely making an initiated close look completed.

## Implemented composition

The capability, codec, client extension, and generated server handler are four
independent crates: `pwright-cap-page-lifecycle`,
`pwright-codec-page-lifecycle`, `pwright-client-page-lifecycle`, and
`pwright-server-page-lifecycle`. Selecting the capability installs both command
descriptors and the two native Chromium features; omitting it installs none of
them.

`ChromiumPageCloseCommandService` holds the browser-session client, shared
production topology, exact `CdpPageServices` route table, runtime lifecycle
handle, bounded confirmation count, and an injected polling policy. After
native absence is proven, it prepares the route-table replacement from the
topology candidate and publishes topology plus routes under one synchronous
lock window before committing runtime disposal. `Page.run_before_unload`
routes through the exact Page session and performs no owner mutation.

The attached factory constructs both services from one controlled Page
runtime. BrowserContext close has a target-aware factory as well, so closing a
context retires descendant Page routes in the same topology publication
window. Continuation admission is synchronous and caller-owned for ordinary
Page close, matching the BrowserContext close abandonment contract.

## Acceptance gates

- schema/codegen proves both commands are independently typed and Page-only;
- client tests prove ordinary-close idempotence and before-unload strictness;
- fake-CDP tests prove exact browser-session versus page-session methods;
- failed native close or failed absence confirmation rolls back both owners;
- future Drop before polling is lazy, and Drop after native start transfers to
  an already-reserved bounded continuation;
- remote frames dispose Frame before Page and before ordinary close Result;
- before-unload Result does not invalidate the Page;
- dedicated Chrome proves ordinary close removes only the selected Page and
  preserves its context and browser.

All listed gates have executable release-mode coverage. The live proof ran
against the dedicated Chrome 152 CDP profile and also re-ran the target-aware
BrowserContext close proof.
