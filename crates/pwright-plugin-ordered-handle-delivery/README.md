<!-- agent-updated: 2026-09-05T18:34:04Z -->

# Ordered Handle delivery plugin

This optional, backend-neutral server plugin closes the remote ownership gap
for commands that return retained Handles. It composes three explicit fences:

1. reserve bounded caller-owned cleanup capacity before browser I/O;
2. send every Handle `Create` over the shared ordered control stream before
   the typed `Result` send begins;
3. keep the reservation until the `Result` send physically completes and run
   ordered `Handle.dispose` if execution or delivery is abandoned.

The plugin supports Frame `evaluateHandle`, Handle `evaluateHandle`,
`getProperty`, and multi-result `getProperties`. Value-only `evaluate` and
`jsonValue` pass through unchanged. A failed creation fence compensates every
returned Handle, and explicit disposal does not report success until the
matching `Dispose` crosses the same control stream.

It chooses no Chromium/CDP implementation, socket, async runtime, task, or
cleanup executor. The independent Chromium Handle-evaluation leaf exposes the
raw creation/operation service and exact application protocol, while the
Handle-lifecycle leaf exposes its raw disposer. A remote recipe composes them
with `OrderedHandleEvaluationHandler::new` and a caller-created
`OrderedRuntimeControl`. The aggregate Chromium remote-handler factory uses
this same plugin as its default delivery policy; its legacy public decorators
remain available only for compatibility.

The standalone normal closure is exactly 15 Pwright packages and contains no
browser backend, provider, launcher, installer, Tokio adapter, Node.js, or
browser download.
