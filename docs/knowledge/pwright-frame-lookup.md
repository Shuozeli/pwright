<!-- agent-updated: 2026-09-09T03:44:00Z -->

# Frame lookup: versioned semantics and implementation gates

## NODE-063: public lookup over the Rust kernel

`page.frame(name | { name?, url? })` is now implemented with string globs,
the caller's actual RegExp and synchronous URL predicates. The facade only
extracts arguments and resolves wrapper identity. Rust owns branch selection,
traversal, glob policy and selected identity. N-API invokes foreign language
operations; no JS browser driver, transport, launcher or worker was added.
The selected addon must be rebuilt; historical default binaries reject lookup
explicitly instead of substituting a JS implementation.

Membership is pinned before RegExp constructors/tests or URL predicates run.
Names use strict JS equality against borrowed Rust metadata. URL strings use
the existing admitted Rust glob planner with the retained Rust Context baseURL.
RegExp execution retains `lastIndex`, custom `test` receiver and cross-realm
recognition. Predicates receive Node URL values; their results use synchronous
truthiness. Only URL-construction exceptions select the invalid-URL fallback;
predicate/test/getter exceptions, including null/undefined/primitives, propagate
unchanged. The facade deliberately does not feed caller exceptions through the
HostError message parser. This is bounded API support, not unrestricted parity
for arbitrary monkey-patching of all JavaScript builtins or boxed-string methods.
Existing glob limits, including explicit lone-surrogate rejection, still apply.

### Selected-path ownership and admission

One 32 KiB conversion reservation precedes native cache-ID copies and callbacks.
A compile-time proof covers 128 output `SelectedFrame` DTOs plus two 20-byte IDs
each, 256 numeric known-wrapper IDs, temporary input/selection IDs and 2048 bytes
of wrapper/bookkeeping slack. Glob input/preparation/output retain their own
existing reservations. V8 objects, Node URL/RegExp allocations and caller-owned
data remain outside this logical Rust ledger; this is not a process RSS bound.

The JS cache holds known wrappers strongly for the synchronous call. Rust walks
only selected-to-root ancestry, stopping at a known wrapper, and returns missing
bindings in parent-first order. `FrameTreeRead::bind` binds from the pinned
revision even if a callback has published a new tree and retired that node.
Each native binding keeps its own state/body lease until finalization. The cache
rechecks identity after callbacks, reusing wrappers produced by reentrant
enumeration and allowing redundant pre-admitted bindings to finalize. No-match
does not allocate a native binding. No topology mutex is held during callbacks.

### NODE-063 evidence and remaining work

Addon SHA-256:
`1b627f190a51b5e174fdb0b357eb2e8a37901c23d215f9483ba554b99888e3c0`.
`.work/node063-frame-lookup/comparison-final.json` passes **41/41** observations
against Playwright 1.58.2 and three native resource/lifetime groups. Cases cover
new and already-open nested topology, duplicate first-match order, name/glob/
RegExp/predicate forms, relative baseURL, custom/cross-realm/stateful regexes,
getter order, primitive exception identity, malformed-glob SyntaxError,
reentrancy, detach, closed Pages and disconnected lookup.

The ancestry check adds exactly two native bindings / 240 bytes for a previously
uncached nested selection and parent; the nonmatching sibling remains unbound.
One thousand cached repeats and oversized-glob rejection leave metrics unchanged.
Reentry reaches 126 in this fixture before admission rejects; this is an observed
depth, not a fixed public recursion limit. Last GC leaves one fixed index state,
zero revisions and 7672 bytes, with all host leases/operations/scopes zero.
The first run failed an incorrect test baseline (live imported Pages versus the
post-disconnect index); `comparison.json` preserves that failure. The corrected
37-, expanded 39-, and final 41-case reports are retained separately.

On this exact artifact, Page enumeration passes 7/7, Frame publication/name
regressions 18/18 and native glob conversion 1647/1647 plus seven checks.
Sequential typing passes 38/38 in `typing-correct-fixture.log`. The earlier
`typing.log` used `input.html` instead of `sequential-typing.html`, so 33 cases
failed on missing `resetTyping`; that setup error is preserved, not attributed
to the native implementation. The rerun used the verified correct fixture.
Selected/default Rust host tests pass 228/226 with 9/7 ignored; strict all-target
Clippy, 46 Node units, 14 pinned oracle characterizations and strict types pass.
Dependency/import checks are unchanged (host 234/235/235, addon 249/250).
The local foundation commit and historical default binary remain unchanged;
no push was performed. Broad Node compatibility, Frame.goto, Frame events and
intermittent connection reliability remain unfinished; these results do not
prove every Frame-order/OOPIF timing permutation or whole-package parity.

Run `pnpm test:frame-lookup` from `packages/pwright` with
`PWRIGHT_NATIVE_PROFILE=drag-runtime`, loopback `PWRIGHT_TEST_CDP`,
`PWRIGHT_FRAME_LOOKUP_FIXTURE` pointing to a verified serve-lib directory URL
containing the two `frame-lookup*.html` fixtures and `empty.html`, and an unused
absolute `PWRIGHT_FRAME_LOOKUP_REPORT`. The supervisor limits the child to
180 seconds, 4 MiB stdout and 1 MiB stderr; the report is capped at 256 KiB.
Metadata polling is an explicit five-second settlement barrier, not Frame-event
timing parity. Owned Contexts are closed without touching unrelated Pages.

## NODE-062: Context ownership verified, lookup still pending

Rust-owned Context baseURL snapshots now pass relative navigation, race identity,
closed/disconnected retention and last-GC tests. The extracted shared N-API URL
helper passes 1647/1647 native glob patterns plus seven boundaries on final addon
9c36434e; Page enumeration is 7/7. These results do not implement public page.frame:
matcher dispatch, selected Frame/ancestor binding and wrapper cache integration
remain the next work. Historical connection failures are still unresolved.
[Verified Context contract](pwright-context-creation.md#verification-status-and-remaining-gates).

## NODE-061: original-layout verification refreshed

Without changing the selected native addon or five-second deadline, the original
observer-session layout now passes **1647/1647** patterns and seven native
boundary/lifetime checks. Public Page enumeration also passes **7/7**. Reports
are `.work/node061-attachment-trace/glob-original.json` and `pages-original.json`;
they retain artifact `5f73372f843cf2f5ac9e29725dee43fa5463cb6a32829c7a453c1f668f12b3cc`.
This supersedes the last-run status, not the earlier failures or open reliability
investigation. The [reused bounded command collector](pwright-chromium-topology-events.md#node-061-reuse-the-bounded-attachment-collector)
adds no production instrumentation or timeout change.

The next actual API integration must own baseURL in Rust across Context creation,
enumeration and retained closed Context/Pages. Admit storage before native Context
creation; avoid a window where concurrent enumeration wraps a new Context without
its settings. Apply accepted baseURL to supported navigation as well as lookup,
rather than accepting and ignoring it. Complete matcher/exception/cache identity
delivery afterward. Passing this primitive does not complete those public gates.

## NODE-060: actual N-API URL-value conversion port

The internal `NativeFrameTree.globPattern` primitive now connects the Rust
`PreparedGlob` policy to Node's WHATWG URL constructor and returns the Rust
GlobPlan pattern through N-API. It is not a public `page.frame` method. Context
baseURL retention, matcher dispatch/callbacks, RegExp execution and selected
Frame/cache identity integration remain unfinished.

### Explicit language-value boundary

The Node profile selects a scoped Node URL value, like the already-designed
JS RegExp/URL predicate-value boundary. Rust still owns preparation, bypass,
failure fallback, normalization/restoration and pattern planning. No JS glob
implementation, driver, transport, launcher, retained constructor reference or
new runtime task is introduced. The earlier Rust `url` example remains a
diagnostic alternative, not a hidden production parser selection.

The native port calls the current global URL constructor, then `toString` and
`origin`, inside the pinned resolver try block. It clears only exceptions from
that block and selects the Rust parse-failure fallback; other native/conversion
failures propagate. Bypassed globs never invoke the constructor. Raw N-API uses
a two-element stack argument array instead of a temporary argument Vec. Handles
stay scoped to the synchronous callback. Node parser/URL storage, including any
Node-owned native allocation, is **outside the Pwright logical Rust ledger**;
this implementation does not prove a whole-process parser/RSS bound. Memory
attribution beyond native Pwright copies remains a separate verification gate.

### Admitted native copies and retention

The tree lends its existing metadata-only factory. Before copying glob input,
the port admits `2*n+1024` bytes plus the conversion guard: UTF-16 validation
requires at most `2*n+2`, then the non-overlapping UTF-8 copy uses `n+1`; slack
accounts for wrapper bodies and bounded conversion diagnostics. Input/baseURL
are capped at 16 KiB. BaseURL remains a borrowed JS value in this primitive and
is not copied into an untracked native String. Lone UTF-16 surrogates in glob
text reject explicitly as unsupported rather than being silently replaced.

After URL resolution, serialized and origin strings are each capped at 256 KiB
before native copying. For their UTF-8 lengths `s` and `o`, reserve
`s+o+2*max(s,o)+1024` plus the conversion guard before either copy; this covers
both resulting UTF-8 buffers and either temporary UTF-16 validation buffer.
The port rejects non-string resolver outputs explicitly. Rust preparation,
restoration and final-pattern storage retain their earlier independent leases.
Peak overlap is five read slots (input, preparation, foreign-output copies,
restoration scratch, final plan), not the host-only planner's three. The final
plan remains admitted through actual `env.create_string` delivery. No live
browser is required by the primitive once a retained tree exists.

### Artifact-specific evidence and open connection regression

`.work/node060-glob-ffi/comparison-late-observer.json` records **1647/1647** exact
patterns through the rebuilt native addon plus **7** boundary/lifetime checks.
The shared corpus is now `scripts/glob-base-cases.mjs`, used by both the earlier
Rust-parser comparison and this actual native boundary test. Checks cover zero
net conversion charges, bypass/pre-parser rejection, caught resolver exceptions,
oversized foreign output, reentrant budget exhaustion, retained closed-tree
use and finalization. Observed recursion reached 39 before shared admission
rejected; 39 is a sample, not a new fixed recursion policy. After finalization,
tree storage returns to one state / zero revisions / 7672 fixed index bytes;
URL/name bytes, native leases and active operations are zero. Exact test-owned
Context/target cleanup is asserted; unrelated shared Chrome Pages are preserved.

Run `pnpm test:glob-ffi` with the selected native profile, loopback-only
`PWRIGHT_CDP_ENDPOINT` and an unused absolute `PWRIGHT_GLOB_FFI_REPORT`. The
supervisor provides 180 seconds, 4 MiB stdout and 1 MiB stderr; the JSON report
is capped at 512 KiB and preserves partial failure state. Normal cleanup calls
have independent ten-second diagnostic deadlines. The `after` experiment in
`PWRIGHT_GLOB_OBSERVER_SESSION` delays only creation of the observer's extra
browser CDP session until after candidate attachment. Default remains `before`.
Process-memory snapshots, when present, are observations rather than pass limits.

**Connection reliability is not fixed.** Original-layout runs repeatedly fail
before executing the new primitive, at the existing five-second attachment
deadline. The first attachment matrix passes 10/10; its expanded 14-run version
adds explicit observer browser-session cases and passes only 3/14, with both
upstream and candidate timeouts. Thus neither observer ordering nor removing
the owned preflight Page is established as a causal fix. The original-layout
glob and public Page-enumeration regressions remain failing on this artifact.
Preserve those logs alongside the successful, labelled experiment; do not report
whole-package parity or a green default browser gate from this result.
The later `comparison-late-memory.json` repeat also times out before reaching
the corpus, despite using the same `after` layout. It captures no process-memory
samples. This further rules out treating that layout as an established fix.

The selected addon is
`5f73372f843cf2f5ac9e29725dee43fa5463cb6a32829c7a453c1f668f12b3cc`;
the historical default binary is unchanged. The real build also now copies host
NOTICE and both license texts beside the selected binary, verified byte-for-byte.
Host release tests pass 213 / 9 ignored, Node units 43, oracle-only tests 14,
strict TypeScript and host/addon all-target Clippy pass. The Rust-parser 1647-case
and resolved-glob 2810-case comparisons remain passing. Dependency closures
remain host 234/235/235 and addon 249/250, without runtime JS driver/CDP/launcher
imports. The foundation commit is unchanged and no push was made.

## NODE-059: bounded glob preprocessing and borrowed URL resolution

`PreparedGlob` now implements the pinned 1.58.2 preprocessing/restoration policy
in Rust. Leading stars bypass both query cleanup and URL parsing; lowercase
special schemes bypass parsing after cleanup. Other inputs use allocation-free
segment keys, preserve dot segments, split literal query boundaries, and restore
tokens in their original insertion order. Hostname normalization uses the parsed
origin, without lowercasing path components. Restoration deliberately preserves
the older JS first-occurrence replacement behavior, including `$$`, `$&`, prefix
and suffix substitutions. The newer upstream callback replacement is different.

The URL resolver is an explicit borrowed port: `relative_path()` supplies its
input and `GlobUrlResolution { serialized, origin }` supplies its output. No
resolution means the pinned parse-failure fallback. Parser calls are forbidden
when the prepared input selects bypass. The result retains no parser object,
Context, runtime, callback, browser or transport. This is **not yet** the complete
Node `page.frame` API. Production parser admission, Context baseURL ownership,
N-API matcher/exception/identity conversion and multi-Frame order remain gates.

### Preprocessing storage contract

All helper-owned heap storage uses existing Frame read/byte admission before
allocation; no token map/vector, task, new ledger or dependency is introduced.

- Input and final restored glob are independently capped at 16 KiB. Preparation
  reserves input bytes, at most `10*n` relative-path bytes and its complete body.
  Bypassed inputs reserve zero relative-path capacity. Five decimal index digits
  suffice for the maximum segment count; keys fit a ten-byte stack array.
- Borrowed parser output/origin each have a 256 KiB admission limit before copy.
  Restoration capacity is `max(16 KiB, serialized.len + source.len)`, rejected
  above 256 KiB. Two alternating String buffers are allocated once at this size;
  every replacement append rejects before exceeding capacity, with no growth.
- Temporary lowercase conversion reserves `24*n+32` bytes plus its String body.
  ASCII preserves width; non-ASCII consumes at least two UTF-8 bytes and emits
  at most three four-byte scalars, so output is bounded by `6*n`. The verified
  Rust 1.98 implementation uses one String with doubling growth: old/new overlap
  is bounded by `18*n` plus minimum-capacity slack. This conservative reservation
  preserves contextual Greek sigma, which per-character lowercasing would lose.
  Re-audit this allocation strategy on toolchain upgrades: pinned
  [lowercase implementation](https://github.com/rust-lang/rust/blob/88d9e12ae/library/alloc/src/str.rs#L433)
  and [growth implementation](https://github.com/rust-lang/rust/blob/88d9e12ae/library/alloc/src/raw_vec/mod.rs#L503).
- Preparation, restoration scratch and final GlobPlan overlap at a peak of three
  admitted read slots. Completion retains only the final compiler lease; all
  preparation/scratch charges drop on success and on every error path. Existing
  allocation guards account for String/owner bodies as well as logical bytes.

The caller must separately admit parser inputs, URL objects, origin conversion
and parser temporaries. The current diagnostic example uses the existing Rust
`url` dependency **outside** this production storage claim. These bounds are not
allocator metadata, V8 memory or process RSS bounds, nor unrestricted input parity.

### NODE-059 evidence

`pnpm test:glob-base` compares **1647/1647** actual Rust patterns to the installed
Playwright 1.58.2 helper. It includes ten baseURL choices, invalid/empty bases,
Unicode/case/scheme/dot/query behavior, placeholder collisions, replacement
metacharacters and exhaustive short query/segment/replacement interactions.
The child is bounded to 120 seconds, 512 KiB input, 4096 cases and 8 MiB output;
an optional unused absolute `PWRIGHT_GLOB_BASE_REPORT` records all mismatches
before asserting. The diagnostic parser's baseURL is capped at 16 KiB. The
earlier bridge field-name error and initial 450-case pass are preserved alongside
the expanded/final reports in `.work/node059-glob-base/`.

Sixteen new unit tests cover preprocessing, restoration, Unicode expansion,
maximum keys, byte/slot rejection and release of every consumed admission.
They are not browser or public Node lookup tests. The adapted source and host
NOTICE retain upstream Apache attribution; no addon rebuild, browser run or
new public API is implied by this offline checkpoint.

Final release verification passes **213 / 9 ignored** selected host tests and
**211 / 7 ignored** default host tests. Strict all-target host/addon Clippy,
43 Node unit tests, 14 oracle characterizations, TypeScript, formatting and diff
checks pass. The earlier resolved-glob comparison still passes **2810/2810**.
Dependency closures remain host 234/235/235 and addon 249/250, with no runtime
JS driver, CDP transport or process launcher imports. Final logs/reports use
the `*-final` names in the evidence directory; native artifact hashes and the
local foundation commit remain unchanged. No push was performed.

## NODE-058: Rust traversal and bounded resolved-glob compiler

The Rust host now exposes a metadata-only `FrameLookup` kernel and `GlobPlan`.
This is implementation of required core pieces, **not** the public Node
`page.frame` method. Public selector validation, baseURL resolution/ownership,
RegExp/predicate adaptation, N-API exception propagation and weak-cache identity
return still need integration and real-browser differential coverage.

`FrameLookup::new` acquires one existing `FrameTreeRead` admission and pins
published membership in its original order. A compile-time layout check proves
the lookup adds no body storage beyond that already-charged read. It borrows a
matching node instead of creating bindings for every candidate. Name matching is
Rust-owned; the generic URL matcher port retains its mutable state and stops on
the first match/error. `LookupError` keeps original matcher errors separate from
metadata failures so N-API can preserve the original pending exception later.
No topology/metadata lock is held while a callback runs. A borrowed URL view
retains its immutable metadata revision during conversion without copying a
second native string. The matcher port is not yet the final public URL predicate
adapter: invalid-URL skipping and JS truthiness remain integration requirements.

The pinned read can outlive the metadata book and Page tree while retaining only
Frame nodes, their URL/name snapshots and ledger leases. Reentrant publication
does not change this call's membership. A retained closed tree is still readable.
Nested lookups must acquire another read slot and reject at the existing limit.
Tests verify all read/node charges return to zero after the last owner drops.

`GlobPlan::compile_resolved` explicitly accepts an **already resolved** UTF-8
glob, capped at 16 KiB. It does not implement baseURL resolution, invoke a regex
engine, or pretend a Rust regex engine matches every JS RegExp. It reproduces
the 1.58.2 compiler including its boolean group state and malformed/nested-group
output, leaving eventual pattern parsing to the JS RegExp boundary.

### Proven planner storage bound

For `n` input UTF-8 bytes, reserve `7*n + 2` output bytes plus the String body
and FrameConversion body, before `String::with_capacity`. A compile-time check
accounts for the complete GlobPlan body. The plan shares the existing Frame
ledger's EventItem/read slots and byte class; there is no new ledger or task.
The bound is per token, not inferred from sample tests:

- One star emits seven bytes; deep-star alternatives consume at least two
  input bytes and emit at most eight.
- Escaped literals emit at most two output bytes per input byte.
- Ordinary UTF-8 literals preserve their bytes; group/comma branches emit at
  most two bytes per consumed byte. Anchors cost two bytes.

No token vector or input copy is allocated. The maximum output reservation is
114690 bytes, before the two owner bodies. The charge lasts until plan drop,
including actual foreign conversion; byte failure rolls back the acquired
read slot. BaseURL preprocessing can expand data differently and needs its own
separate proof before this compiler can be wired into full URL lookup.

### Evidence and scope

`.work/node058-lookup-core/` records **2810/2810** exact pattern strings matching
the actual Playwright 1.58.2 compiler, including exhaustive short syntax strings,
Unicode, malformed groups and maximum-size input. The bounded Rust example reads
at most 512 KiB/4096 cases, streams each output while its plan remains leased,
and verifies zero planner bytes/read slots after completion. The Node comparison
has a 120-second child deadline and 8 MiB output cap. Run `pnpm test:glob-plan`;
optional `PWRIGHT_GLOB_PLAN_REPORT` is an unused absolute report path.

Fourteen new Rust tests cover traversal, short-circuit state/error identity,
reentrant publication, borrowed URL lifetime, closed/dropped owners, nested-read
exhaustion, token behavior, maximum input and planner rollback/final release.
Selected host release tests pass **197 / 9 ignored**. Neither a real browser nor
the Node addon was needed/rebuilt for this offline kernel checkpoint; the prior
verified NODE-056/057 binary remains unchanged. Public Node lookup and its
versioned behavior suite are **not** yet satisfied by these tests.

Default host release tests pass **195 / 7 ignored**. Strict all-target host/addon
Clippy, **43** default Node unit tests, the **14** oracle-only characterizations,
strict TypeScript and the dependency/import boundary pass. Host closures remain
234/235/235 and addon closures 249/250. Final compiler comparison is recorded in
`glob-plan-final.json`; no API/browser or repository-wide conformance claim is
made from this lower-level compiler gate.

### Source attribution and packaging

The compiler adapts Playwright's Apache-2.0 algorithm. Its source retains
Microsoft attribution and a modification notice. The host crate contains
`LICENSE-MIT`, `LICENSE-APACHE-2.0` and upstream attribution in `NOTICE`;
its package metadata now reflects `MIT AND Apache-2.0`. Other workspace crates'
license metadata is unchanged. The Node build hook copies these files alongside
each newly built native profile before publishing its staged binary. A unit test
checks all copied bytes and attribution. Existing native binaries/notices were
not regenerated in this checkpoint; this is not a repository-wide license audit.

## Target and status

Implement the complete `page.frame(string | { name?, url? })` lookup contract
for the fixed differential oracle, **Playwright 1.58.2**, without implementing
browser selection or glob logic in the JS facade. NODE-063 implements the
declared matcher forms under the documented resource limits and passes the
41-case public workload above. This is not an unrestricted whole-package parity
claim. Frame names have Rust-owned, bounded published metadata (NODE-056).

The installed oracle, not the moving third-party checkout, is authoritative:
`tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/lib/client/page.js`
and `lib/utils/isomorphic/urlMatch.js`. The current third-party checkout lives
under `packages/isomorphic/urlMatch.ts` and has materially different behavior:
it resets RegExp lastIndex, rejects nested/unbalanced glob braces explicitly,
and supports URLPattern. Porting it blindly would fail the pinned contract.
Any oracle-version upgrade must be a recorded migration with both behaviors
compared, not a silent change to what “compatible” means.

## Observable requirements

1. Enumerate the same membership/order as `page.frames()`, return the first
   matching existing Frame object, and return null when nothing matches.
   Duplicate names/URLs, main Frame, newly attached Frames, removed Frames and
   retained closed Pages all need explicit cases.
2. Extract name before URL, but read the URL property even if name will win.
   A truthy name selects strict name equality and never evaluates the URL
   matcher. Missing/empty name with a truthy URL selects URL matching.
   Empty string, empty options, empty name alone and empty URL alone reject
   before Frame enumeration. Preserve getter/predicate thrown-value identity.
3. URL strings are Playwright globs, not substring matching or shell globs.
   Single `*` cannot cross a slash; deep stars, slash elision, escaping,
   alternatives, literal query question marks, explicit schemes, domain
   normalization and baseURL resolution must follow the pinned implementation.
   Malformed patterns must preserve the pinned conversion/RegExp outcome,
   rather than borrowing newer validation rules.
4. A supplied RegExp is the caller's actual object, including custom `test`
   behavior and observable `lastIndex`. In 1.58.2 the matcher does **not**
   reset lastIndex: repeated lookups with `/about/g` over one about:blank Frame
   produce match, null, match. Do not serialize/copy the RegExp into a fresh one.
5. A function receives a WHATWG URL object. Invalid Frame URLs skip the
   function. Return values are tested for JS truthiness synchronously:
   a Promise is truthy, not awaited. Unsupported matcher types are rejected
   lazily when a valid Frame URL is encountered, not eagerly for empty/invalid
   collections. URLPattern is not a supported matcher in this pinned version.
6. Matching must remain usable on a retained closed Page's frozen collection.
   Do not turn a metadata read into a live CDP/world/transport requirement.
   Reentrant callbacks must not run while holding a topology/metadata mutex.

## Rust / FFI ownership

- Rust owns collection traversal, branch selection after conversion, exact-name
  comparison, glob planning, short-circuiting and Frame identity selection.
  Use one admitted `FrameTreeRead` to pin published membership for the call;
  read the Frame's published metadata for each candidate. Never traverse live
  CDP topology or invoke browser evaluation to implement lookup.
- The N-API boundary invokes actual caller-supplied JS predicates and RegExp
  methods on the Node thread, and constructs their JS URL/string arguments.
  These are language-value/user-callback operations, not a second JS browser
  driver. Preserve pending JS exceptions instead of wrapping thrown objects
  into an unrelated HostError; coerce callback return values with JS truthiness.
- Keep callback handles scoped to the synchronous call. No ThreadsafeFunction,
  task, session-global matcher cache, retained FfiBudget, runtime registry or
  transport is required. Nested lookups must acquire their own read/conversion
  admission before any callback/copy, so recursion cannot evade the resource cap.
- Resolve the selected Frame through the JS weak identity cache. Do not force
  binding/allocation of every nonmatching Frame just to return one match.
  Resolve only the selected Frame and necessary ancestors, using identities
  from the pinned read. Hold known JS wrappers strongly through conversion so
  a callback/GC cannot invalidate the cache hints being used.
- Borrow published URL/name strings through N-API conversion, following the
  NODE-056 name read, without an unadmitted second Rust string allocation.
  Hold the read/temporary leases through actual Node conversion and failures.

## Original resource requirements and current boundary

Use an explicit lookup plan/admission boundary and reuse metadata ledger classes
rather than creating a runtime worker. NODE-060/063 cap glob/baseURL text at
16 KiB and separately prove temporary-plan/conversion maxima above. Name matching
borrows published Rust text and compares scoped JS values without copying the
caller name into Rust. The 16 KiB input alone does not bound replacement expansion
or token maps.
Reserve output-plan bytes before growing a buffer, account transient overlap,
and keep original JS RegExp/predicate objects JS-owned without native cloning.

The existing tree has 128 Frames per Page, 256 state and 256 revision/read slots,
and 8 MiB logical bytes. A lookup should not allocate wrappers for all 128 Frames
or acquire native bindings after an irreversible mutation; it is read-only.
Tests must cover failed plan admission, callback exception, empty/no-match result,
nested reads, retained selection and last-wrapper finalization. RSS/V8 storage
must remain distinct from logical Rust lease claims.

## Dependency gates, not silent fallbacks

NODE-062 supplies Rust-owned Context baseURL settings and navigation resolution;
other Context options remain unsupported. NODE-063 uses those settings in lookup
and tests first-match order on multiple/nested pre-existing Frames as well as
newly created topology. Those checks must not be generalized to every OOPIF
attachment permutation or event-delivery timing; broader coverage remains open.

## Verification sequence

1. Pin the public client semantics with offline characterization.
2. Implement bounded Rust traversal/glob/baseURL planning and fake callback-port
   tests, including version-specific malformed patterns and RegExp state.
3. Bind native selection, pending exceptions and JS identity cache resolution;
   add type and native resource/GC tests. Do not expose a fake JS-only `find`.
4. Reuse one public workload on oracle/candidate with multiple named/nested Frames,
   every matcher form, callbacks/getters, reentrancy, duplicate/order cases,
   navigation, detach/close and resource exhaustion. Verify exact owned cleanup.
5. Run Frame/name, Page enumeration and input regressions on the same artifact.
   Keep setup-sensitive Mouse traces and connection failures visible separately.

## Original characterization evidence (NODE-057)

`packages/pwright/test/frame-lookup-oracle.test.mjs` executes the **actual pinned
Page.frame/urlMatches implementation** with synthetic retained Frame views.
Fourteen tests pass, covering first-match identity, branch/getter ordering,
empty/missing selectors, callback arguments/truthiness/exceptions, RegExp state,
glob slash/query/baseURL behavior and lazy invalid-matcher checks. These are
client algorithm characterization, **not** candidate or real-browser parity.
Run `pnpm test:lookup-oracle` from the Node package; the installed test oracle
is required. Output: `.work/node057-mouse-isolation/lookup-oracle.log`.
