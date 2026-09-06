<!-- agent-updated: 2026-09-05T18:15:35Z -->

# Chromium Handle evaluation composition

This crate is the independently installable Chromium server composition for
`pwright.handle_evaluation` 1.2. It implements Frame `evaluateHandle` plus
Handle `evaluate`, `evaluateHandle`, `jsonValue`, `getProperty`, and bounded
`getProperties` over the separate Handle-lifecycle leaf.

The caller supplies the controlled runtime, ObjectId source, production
resource ledger, CDP clients and execution-world IDs, and bounded continuation
queues. Installation does not launch or download Chromium, start an executor,
or select a transport.

Creation reserves lifecycle capacity before CDP. A successful single result or
property batch publishes runtime object identity and every lifecycle route as
one all-or-nothing commit; failures compensate all returned native objects.
Handle arguments must belong to the exact same Frame, Page, and CDP client.

The installation exposes its exact application protocol and raw command
service, and the lifecycle installation exposes its raw disposer. Remote
servers can pass those three pieces to
`pwright-plugin-ordered-handle-delivery`, which adds bounded continuation,
ordered Create-before-Result, Result-abandonment cleanup, and ordered Dispose
without adding transport policy to this CDP composition leaf.
