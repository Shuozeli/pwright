<!-- agent-updated: 2026-09-05T19:52:03Z -->

# ElementHandle adoption

## Upstream contract

This decision is based on the local Playwright `1.63.0-next` checkout at
`de214f440b7e34937fe4886f046b78b757136087`.

Playwright distinguishes two argument rules:

- a JSHandle must already belong to the exact target execution context;
- an ElementHandle from another execution context is temporarily adopted into
  the target context before evaluation.

Chromium adoption is a two-request backend operation. `DOM.describeNode` maps
the source RemoteObject `objectId` to a stable `backendNodeId`, then
`DOM.resolveNode` materializes a new RemoteObject in the target numeric
`executionContextId`. A null or unavailable result reports that the node cannot
be adopted from the other document. The temporary adopted RemoteObject is
released after the user operation whether that operation succeeds or fails.

This can make a same-process child-frame ElementHandle usable in a parent
Frame. It does not make ordinary JSHandles portable and does not bridge an
out-of-process document whose CDP session cannot resolve the backend node.

Source anchors:

- `packages/playwright-core/src/server/dom.ts`: `FrameExecutionContext.adoptIfNeeded`;
- `packages/playwright-core/src/server/javascript.ts`: argument adoption,
  exact-context validation, and final disposal;
- `packages/playwright-core/src/server/chromium/crPage.ts`:
  `_adoptElementHandle` and `_adoptBackendNodeId`;
- `tests/page/frame-evaluate.spec.ts`: same-process ElementHandle success,
  cross-process ElementHandle failure, and cross-Frame JSHandle rejection.

## Pwright decision

Adoption is a Chromium backend policy, not a portable Handle capability and not
an implicit relaxation of Handle identity. The portable wire continues to send
only the existing Handle ObjectId. The server may select an optional adoption
component after resolving that ObjectId.

The first implementation boundary is a low-level, independently testable CDP
primitive. It:

1. accepts only a retained ElementHandle with a native object ID;
2. requires source and target to share the exact CDP client/session;
3. validates a positive target numeric execution-context ID before browser I/O;
4. describes the source object and resolves its backend node in the target
   context;
5. bounds and validates the returned RemoteObject as an ElementHandle; and
6. returns a non-cloneable, must-use temporary owner with explicit async
   release.

The default Frame/Handle command installation remains exact-world. A separate
opt-in installation now composes the route, production-resource,
cancellation-safe temporary-owner, and public-command transaction below:

- Frame routes retain both Chromium's numeric `executionContextId` and stable
  `uniqueContextId`;
- every persistent Handle route records its source execution-world identity;
- a production batch profile declares one `RemoteHandle` plus bounded native-ID
  and preview bytes per temporary object, and reserves the whole worst-case
  batch before the first DOM request;
- the composition owner pre-admits one caller-owned continuation slot before
  adoption, holds it through use and release, and continues started adoption or
  release futures after caller cancellation;
- invalid batches and partial adoption failures release all acquired temporary
  objects and production leases;
- JSHandles and cross-session ElementHandles still fail before adoption I/O.

This staging prevents a compatibility feature from introducing an unbounded or
unrecoverable RemoteObject leak. A caller cannot infer full Playwright adoption
support merely because the backend primitive is installed; it must select the
adoption-aware installation and declare its additional peak explicitly.

## Implemented primitive checkpoint

`pwright-backend-cdp-handle-evaluation` now exposes
`CdpElementHandleAdoptionService`, its explicit request, and a non-cloneable
`CdpAdoptedElementHandle` owner. The shared CDP trait gained only the two exact
describe/resolve operations required by Chromium; existing fake and real CDP
clients implement them without adding a capability, runtime, transport, or
browser dependency.

## Execution-world route checkpoint

`CdpFrameExecutionRoute` now exposes both the numeric `executionContextId` and
stable `uniqueContextId` already owned by topology. The bounded Chromium Handle
composition requires a positive numeric ID when a Frame route is bound and
retains both identities in its evaluation service. Every published Handle
payload records the exact source-world pair alongside its native object.

This does not perform adoption. It strengthens the existing exact-world rule:
if a Frame rebinds to another world in the same CDP session, a Handle from the
old world is rejected before another Runtime call. AAA tests cover invalid
numeric IDs without route mutation and same-session world replacement without
browser I/O.

Five AAA tests cover exact call parameters and release, pre-I/O invalid-context,
JSHandle and cross-session rejection, and malformed-result compensation. A
dedicated Chrome 152 proof creates a main-world ElementHandle, resolves it into
a newly created isolated world, evaluates `tagName` through the temporary
wrapper, releases both native owners, destroys only the test Context, and
confirms that the externally owned browser remains reachable.

## Production batch and continuation checkpoint

`CdpElementHandleAdoptionBatchService` is the opt-in production boundary. It
prevalidates the complete deduplicated request batch, synchronously reserves
the declared peak for every temporary object before browser I/O, enforces a
bounded concurrent-batch count, shrinks retained bytes to the actual native ID
and preview, and owns every resulting lease until exhaustive release. Partial
failure releases all earlier objects before returning.

`ContinuableElementHandleAdoptionService` wraps that raw boundary for command
composition. One reserved cleanup permit spans the first adoption poll through
the final release. Dropping a started adoption finishes it and releases any
result; dropping the batch or even an unpolled explicit-release future submits
the same cleanup exactly once. Exhausted continuation capacity fails before
DOM I/O or production admission.

Four backend AAA tests cover production-peak declaration, two-object ownership
and release, whole-batch prevalidation, and partial-failure compensation. Four
composition AAA tests cover pre-I/O continuation rejection and every dropped
future/owner boundary. A second dedicated Chrome 152 proof uses the production
batch through its continuation wrapper, consumes the adopted main-world node in
an isolated utility world, then observes `RemoteHandle`, backend bytes, active
leases, and the continuation reservation all return to zero while external
Chrome remains alive.

## Opt-in public-command checkpoint

`ChromiumHandleEvaluationInstallation::install_with_element_adoption` is the
only entry point that enables cross-world ElementHandle arguments. Its resource
requirements are exposed separately through
`requirements_with_element_adoption`; the existing `install` method and profile
remain exact-world and pay no adoption cost.

Before any DOM request, the shared command resolver deduplicates portable IDs,
keeps same-world arguments unchanged, rejects JSHandles and cross-session
ElementHandles, and confirms that each source Handle still belongs to its
Frame's current execution world. Compatible ElementHandles from another Frame
of the same Page and exact CDP session are adopted as one batch. The temporary
batch remains owned across the Runtime call, is released before command
completion, and is also covered when evaluation, publication, topology
revalidation, or caller ownership fails.

Three command-level AAA tests prove same-Page child-Frame success, JSHandle
pre-I/O rejection, and OOPIF-shaped cross-session rejection without CDP I/O. A
dedicated Chrome 152 test creates a real child-Frame ElementHandle through the
public command service, passes its portable ID into the parent Frame, observes
`CHILD`, releases all temporary and published owners back to baseline, and
preserves the externally owned browser.

A second public-command Chrome 152 proof uses recursive target auto-attach and
a `serve-lib` cross-site fixture to create a real OOPIF session. It creates the
source ElementHandle inside that session, rejects its use in the parent Page
before temporary-resource admission, and preserves external browser ownership.
Chrome 152 also exposed that an OOPIF session's `Page.getFrameTree` root legally
retains its embedding parent ID; topology validation now permits that only for
non-Page sessions and continues rejecting it for Page-session roots.

The aggregate Chromium root still contains an older parallel Handle command
implementation. Adoption is intentionally not copied into that path. Its
future opt-in exposure must first delegate aggregate Handle evaluation to this
leaf, so Pwright has one argument/adoption policy rather than two drifting
implementations.

## Acceptance status

- same-world ElementHandle and JSHandle arguments perform no adoption calls;
- cross-world JSHandles fail before browser I/O;
- same-session ElementHandles use exactly one describe and one resolve per
  deduplicated portable Handle;
- cross-session or unresolvable ElementHandles return a typed failure;
- command composition substitutes each adopted native argument into the target
  call while leaving same-world arguments untouched;
- every temporary object is released exactly once after success, evaluation
  failure, cancellation, dropped future, Result-delivery abandonment, or
  topology loss;
- no temporary adopted Handle enters the portable object graph or emits a
  lifecycle `Create`;
- dedicated-Chrome tests cover main/utility worlds, same-process child Frames,
  and real auto-attached OOPIF rejection while preserving external browser
  ownership.
