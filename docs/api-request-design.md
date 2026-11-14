<!-- agent-updated: 2026-10-06T06:01:05Z -->

# Playwright-compatible API request design

Status: NODE-364 exposes Playwright's private response-log compatibility hook
as a delegation to the existing bounded Rust response log; NODE-363 composes
the Rust request owner with BrowserContext proxy and
proxy-bypass policy; NODE-362 composes target-origin TLS and exact-origin
client identities over established HTTP CONNECT and SOCKS5 tunnels; NODE-359 adds a separately
selectable Rust SOCKS5 connector with remote hostname resolution; NODE-358
composes HTTPS proxy-server transport
into the Rust HTTP CONNECT leaf added in NODE-357;
NODE-355 composes the Rust request stack with BrowserContext/Page
cookie and settings ownership plus `Route.fetch()`; NODE-223 selects the Node
stack with an optional Rust-owned
client-certificate capability; NODE-221/222 select the common request stack
through optional Node and Python native
profiles. Node passes the complete official global-request and global-cookie
files in both the Library and Bidi project matrices: 774/774 candidate and
774/774 independent oracle invocations. Python ABI 11 passes the complete
unchanged async and sync `test_fetch_global.py` matrices: 231/231 candidate and
138/138 plus 93/93 independent oracle invocations, plus a fixed
client-certificate selection with eight passes and four upstream macOS WebKit
skips in both lanes. The authenticated HTTP-proxy, HTTPS-proxy-server ALPN and
SOCKS5 rows pass 9/9 candidate and 9/9 oracle across the Library project
matrix, plus 18/18 in each lane across the Bidi matrix. The thin facades cover
all
seven verbs, Basic credentials, cookies, file/object storage state and the
official request-scoped resolver hook; Rust retains the context, transport,
cookies, origins and address list. Node also passes the exact official
client-certificate validation row 3/3 and seven direct-request behaviors 21/21
in both candidate and oracle Library lanes, plus validation 6/6 and direct
requests 42/42 in both Bidi lanes. The two certificate-through-proxy rows pass
3/3 per proxy in Library and 6/6 per proxy in Bidi, totaling 18/18 candidate
and 18/18 oracle. This is not full APIRequest or full Playwright conformance.
Authenticated SOCKS, multiple identities for one origin and the rest
of both upstream corpora remain open.

## NODE-364 response failure-log compatibility

`APIResponse._fetchLog()` is a compatibility alias over the public `log()`
adapter. Both paths read the same bounded Rust-owned response log and share
the same disposal and error mapping; Node stores no duplicate log or response
body. The three unchanged official `toBeOK` failure-printing rows now pass,
and the complete `page/expect-boolean.spec.ts` file passes 86/86 candidate and
86/86 independent oracle. This does not add request transport behavior or
change the remaining APIRequest/full-corpus boundaries below.

## NODE-363 BrowserContext proxy and request composition

`pwright-api-request-connector-proxy-bypass` is an independent injected
connector decorator. It owns bounded comma-separated parsing and implements
Playwright's hostname-suffix bypass rule without adding proxy, TLS or browser
dependencies. HTTP CONNECT and SOCKS5 proxy selection remain separate leaves;
the combined `context-api-request-proxy` profile is an explicit union rather
than a new monolith.

BrowserContext creation carries typed Rust proxy options through the existing
CDP continuation chain. The host normalizes missing schemes and strips proxy
paths, trims bypass entries, rejects browser SOCKS credentials, adds Chromium's
forced `<-loopback>` bypass and retains at most the fixed Context capacity.
`browserContext.request` then constructs the existing Rust request stack with
that exact retained proxy. It reads live Chromium cookies before an operation,
writes resulting cookies back after it, inherits the supported Context request
settings, and is disposed after successful Context close. `page.request`
returns that same owner.

Node validates only the Playwright-shaped public options and forwards them to
Rust; it does not retain sockets, cookies or an HTTP client. The selected
profile's dependency proof observes exactly the three proxy leaves, regex
closure and required frame-lifecycle leaf over the base request profile. The
unchanged official Chromium Library Context-proxy and bypass rows pass 2/2 in
both candidate and oracle. Twelve request crates pass 119/119 release tests,
the host passes 526 selected tests with eight explicit environment-gated
ignores, and Node passes 286/286 plus strict TypeScript and Clippy. Browser
page-traffic proxy authentication, non-Chromium backends and unsupported
nondefault Context settings remain separate work.

## NODE-362 composable target TLS over proxy tunnels

`ApiTunnelTlsUpgrader` is a transport-level port over an already-established
`ApiConnection`. HTTP CONNECT and SOCKS5 own only proxy negotiation, then
delegate an HTTPS request origin through that port. The Rustls implementation
uses target SNI, request-scoped verification and the existing exact-origin
client-identity provider; it does not resolve or open another socket. Without
an injected upgrader, both proxy leaves reject HTTPS explicitly.

The Node profiles
`api-request-client-certificates-http-proxy` and
`api-request-client-certificates-socks5-proxy` are exact unions of existing
features. An executable dependency proof compares full Cargo closures and
rejects an added or removed package, preserving the rule that neither proxy
leaf depends on Rustls. Both artifacts build and load in release mode.

The unchanged official HTTP-proxy+mTLS and SOCKS5+mTLS rows pass 3/3 per proxy
in Library and 6/6 per proxy in Bidi for both candidate and independent
oracle. Eleven request crates pass 113 release tests and Node passes 282/282.
The cumulative Core ledger is candidate 1,309 expected / one skipped / 31
unexpected / 47,136 not-run and oracle 992 expected / 47,485 not-run. Proxy
bypass, authenticated SOCKS, BrowserContext proxy/certificate composition and
the remaining corpora stay open.

## NODE-361 Bidi client-certificate evidence

The existing `api-request-client-certificates` profile runs its unchanged
input-validation row and seven direct-request behaviors through all six Bidi
projects without an ABI or runtime change. Candidate and independent oracle
each pass validation 6/6 plus direct requests 42/42. The selected release
artifact is rebuilt and load-verified before the candidate run. The two
certificate-through-proxy rows stay explicit `not-run` because they require
HTTPS target tunneling, which this evidence checkpoint does not imply. The
cumulative Core ledger is candidate 1,291 expected / one skipped / 31
unexpected / 47,154 not-run and oracle 974 expected / 47,503 not-run.

## NODE-360 Bidi proxy evidence

The same HTTP CONNECT and SOCKS5 profiles run the three global-request proxy
contracts under the six official Bidi projects without any runtime or ABI
change. Authenticated CONNECT, HTTPS proxy-server ALPN and SOCKS5 each pass
6/6 candidate and 6/6 independent oracle. Combined Library and Bidi proxy
evidence is 27/27 per lane. The cumulative Core ledger is candidate 1,243
expected / one skipped / 31 unexpected / 47,202 not-run and oracle 926
expected / 47,551 not-run.

## NODE-359 SOCKS5 composition

`api-request-socks5-proxy` is a separate native profile rather than an
extension of HTTP CONNECT. It adds exactly one Pwright dependency leaf over
`api-request`: `pwright-api-request-connector-socks5`. The leaf wraps the
injected direct connector, parses an explicit `socks5:` or `socks5h:` endpoint,
performs the bounded SOCKS5 no-auth negotiation and sends domain names to the
proxy instead of resolving them locally. IPv4 and IPv6 target forms use their
binary address encodings. Reply codes and address forms are parsed before the
tunnel is returned, and any already-read tunnel bytes remain owned by the
connection.

Node only validates the public proxy object, selects the exact native profile
and forwards server/credential values once. The first leaf rejects credentials
and HTTPS request origins explicitly. It does not add a Node HTTP stack, a
browser dependency or local DNS fallback. A real Tailscale-bound native smoke
verifies that an unresolved hostname reaches the proxy unchanged.

The unchanged official `tests/library/fetch-proxy.spec.ts:133` row passes 3/3
candidate and 3/3 independent oracle across the Library project matrix.
Eleven request crates pass 110 release tests, the Node package passes 279/279,
strict selected Clippy and TypeScript pass, and the dependency proof observes
exactly the SOCKS leaf. The cumulative Core ledger is candidate 1,225 expected
/ one skipped / 31 unexpected / 47,220 not-run and oracle 908 expected /
47,569 not-run.

## NODE-355 BrowserContext and route composition

`context-api-request` preserves the standalone request owner and supplies a
BrowserContext adapter rather than a second client. Each operation obtains the
current bounded Context cookies and applies the retained base URL, browser user
agent, HTTP credentials and TLS-error policy. An explicit `Cookie` header wins
over Context cookies. The owner is disposed with its Context and surfaces the
Context close reason on later calls. `page.request` references the same owner.

The ABI carries typed query pairs and typed bytes/JSON/form/multipart bodies.
Rust performs encoding, content headers, redirects, connection-reset retry and
response ownership. `route-fetch` reuses this owner to replay a paused route
with overrides and composes the retained `APIResponse` directly into
fulfillment, including a typed JSON override. No response body or HTTP wire
format is recreated in Python.

The pinned official selections pass candidate/oracle 108/108: Context request
51 async + 47 sync and route fetch/fulfill 6 async + 4 sync. The cumulative
Python ledger is 1,015 passed, zero failed, seven skipped and 5,504 not run.
This proves the selected browser-sharing and route-fetch contracts, not the
remaining complete Python or Core JS/TS corpora.

## Why this is a separate capability

Playwright exposes two related HTTP clients:

- `playwright.request.newContext()` creates a browser-independent
  `APIRequestContext` with its own cookie store.
- `browserContext.request` and `page.request` share cookie state with their
  owning `BrowserContext`.

Neither client needs CDP or a browser process to perform the request. Pwright
therefore must not hide HTTP inside the Chromium aggregate or force an HTTP
stack into installations that only need browser automation. The capability is
an optional Rust owner with a transport port. Browser-context cookie sharing is
a separate adapter over that same owner.

## Pinned Playwright 1.58.0 behavior

The implementation and official tests establish these initial invariants:

- request methods include `fetch`, `get`, `post`, `put`, `patch`, `delete` and
  `head`;
- context defaults may be overridden per request for timeout,
  `failOnStatusCode` and redirect limits;
- `timeout: 0` disables the timeout;
- `failOnStatusCode` fails for status `< 200` or `>= 400`, so a returned 3xx is
  not itself a status failure;
- `maxRedirects: 0` returns the redirect response instead of following it;
- retries apply only to connection reset in the pinned implementation;
- response bodies stay owned by the request context until
  `APIResponse.dispose()` or context disposal;
- context disposal cancels in-flight work, rejects later work and invalidates
  retained response bodies;
- the global context owns cookies and storage state; the BrowserContext form
  delegates cookies and storage state to that BrowserContext.

These are source-derived contracts, not evidence that Pwright already passes
the corresponding official tests.

## Package and dependency boundary

`pwright-api-request` owns policy and lifetime only:

```text
Node facade ─┐
             ├─ thin binding ─> pwright-api-request ─> ApiRequestTransport
Python facade┘                                      ├─ transport policy
                                                    │  ├─ HTTP/1 codec
                                                    │  └─ connector/TLS plugins
                                                    └─ test fake

BrowserContext cookie adapter ─> cookie port ─> pwright-api-request
```

The crate has no `reqwest`, Hyper, CDP, browser, Node or Python dependency. Its
`ApiRequestTransport` receives one validated `TransportRequest`, including a
request-scoped cancellation token and the response-body streaming limit. A
backend must enforce that limit while reading the stream and must not leave a
detached request task after its future is cancelled.

Request preparation also stays in this shared crate. Rust resolves encoded or
repeated query parameters, installs Playwright-shaped default headers, applies
case-insensitive context/request overrides, serializes bytes/JSON/form/multipart
bodies, infers file MIME types and owns `content-type`/`content-length`.
Multipart boundaries are deterministic context-owned identities and reject a
body collision after four bounded scans; no language binding supplies
randomness or constructs wire bytes.

`pwright-api-request-http1` is a separate optional, runtime-free codec. It
encodes one bounded HTTP/1.1 exchange, injects `Host` and connection ownership
when absent, and incrementally decodes informational, fixed-length, chunked and
close-delimited responses. It preserves response header spelling, duplicate
entries and order, which the official `headersArray()` assertions require.
Request values are emitted as Latin-1; non-Latin-1 language strings are
rejected before transport dispatch. The codec owns no socket, DNS, task,
executor, TLS, proxy, decompressor, cookie jar, redirect or retry policy.
Its event API exposes the final response head before consuming body bytes;
the original complete-response API remains a wrapper over those events.

`pwright-api-request-transport` is a third selectable layer. It drives one
codec exchange through injected `ApiConnector`/`ApiConnection` ports, drops
each connection after that exchange, follows 301/302/303/307/308 with pinned
method/body/header rewriting, distinguishes public `maxRedirects: 0` from an
exhausted recursive budget, and retries only classified connection resets with
250 ms exponential backoff. Cancellation is checked before connect and each
read; no detached task is created. Transport log retention is bounded and
truncated with a sentinel instead of turning an otherwise successful request
into a public failure.

The first concrete leaves remain independently selectable:

- `pwright-api-request-connector-tokio` owns bounded DNS and plain TCP only;
- `pwright-api-request-connector-rustls` adds HTTP/HTTPS, WebPKI roots,
  HTTP/1.1 ALPN and request-scoped verified versus `ignoreHTTPSErrors` config;
- `pwright-api-request-connector-http-proxy` wraps an injected connector with
  bounded HTTP CONNECT, optional redacted Basic proxy credentials and retained
  prefetched tunnel bytes; the first profile accepts plain-HTTP origins only;
- `pwright-api-request-decompress` implements the pinned empty-stream-safe
  gzip/x-gzip, zlib deflate and Brotli behavior while bounding decoded output;
- `pwright-api-request-cookie` owns the bounded global cookie jar, structured
  import/export and URL-specific request/response composition behind the base
  `ApiCookieStore` port;
- `pwright-api-request-auth-basic` owns bounded UTF-8 credentials, exact
  case-insensitive origin matching, Basic encoding and `unauthorized` versus
  `always` send policy behind the base `ApiAuthorizationProvider` port.

The transport asks the selected store for a Cookie header only when the caller
did not provide one. It consumes raw Set-Cookie values in wire order and clears
the prior Cookie header before recomposing a redirect. Cookie parsing preserves
values containing `=`, normalizes domain case, distinguishes host-only from
dotted domain cookies, applies path/secure/expiry rules and permits secure
cookies on HTTP localhost as pinned Playwright does. The current transport sees
Set-Cookie at the final-head event before body consumption. A followed redirect
or handled Basic challenge drops its pending connection, decoder and read
buffer without retaining the body. A terminal returned response still consumes
and decompresses its complete bounded body before registration.

Authorization composition never overwrites an explicit caller header. The
default policy waits for a trimmed `WWW-Authenticate` value beginning with
`Basic`, then performs one retry because the inserted header makes a second
challenge terminal. `send: always` is evaluated once before the redirect chain
and the resulting header follows redirects exactly as the pinned implementation
does. Origin comparison is case-insensitive but otherwise exact across scheme,
host and port; the configured string is not silently broadened or normalized.

The TCP leaf accepts at most 64 DNS results, preserves the family of the first
result, alternates IPv6/IPv4 families and starts one connection attempt every
300 ms until one succeeds. A winner or cancellation drops every other pending
future; no attempt task is spawned. IP literals bypass DNS and family
interleaving. The proxy leaf opens one CONNECT tunnel per exchange and accepts
HTTP or HTTPS proxy servers through the injected connector. It keeps TLS over
the established tunnel for HTTPS targets, SOCKS and bypass composition as
explicit future work.
Broader proxy, client-certificate and platform-specific trust behavior must
still be verified before a public compatibility claim.

A concrete backend belongs in a separately selectable crate. Node and Python
bindings may convert public values and error presentation, but may not perform
HTTP, retries, cookie mutation, timeout ownership or response retention.

## Current Rust ownership contract

`ApiRequestContext` owns:

- one fixed semaphore for concurrent request admission;
- one cancellation tree for context disposal and request-local timeout;
- validated context defaults for base URL, timeout, status policy and redirect
  count;
- context user-agent/extra headers plus request-level query/body composition;
- one optional cookie-store port for structured global state and wire headers;
- one optional authorization-provider port with no built-in credential type;
- one optional request-scoped list of validated resolver IP addresses while
  retaining the original URL host for Host and TLS SNI;
- one bounded registry of live `ApiResponse` bodies and transport logs;
- deterministic, idempotent context and response disposal;
- fixed metrics for in-flight calls, live responses and retained bytes.

Request validation completes before transport invocation. It bounds method,
URL, headers and body, accepts only HTTP(S), resolves relative URLs against the
context base URL and preserves `timeout: 0` as no deadline. The public deadline
includes time waiting for the in-flight permit. Timeout cancels the exact
request token; context disposal cancels the full tree and preserves its reason.

A successful response is validated again before registration. Status failures
produce a bounded text preview without registering an unreachable response.
Dropping or explicitly disposing an `ApiResponse` removes its exact registry
entry and returns both body and log bytes. Context disposal clears every entry.

## Default resource limits

| Resource | Default |
| --- | ---: |
| Concurrent requests | 8 |
| Live responses | 64 |
| Request body | 16 MiB |
| Query parameters | 1,024 |
| Request-scoped resolver addresses | 64 |
| Form or multipart fields | 1,024 |
| Response body | 16 MiB |
| Aggregate retained response bodies | 64 MiB |
| Headers | 256 |
| Aggregate header bytes per request/response | 256 KiB |
| Method | 1 KiB |
| URL | 64 KiB |
| Status text | 8 KiB |
| Log entries per response | 64 |
| Log bytes per response | 256 KiB |
| Aggregate retained logs | 1 MiB |
| Global cookies | 1,024 |
| One cookie | 4 KiB |
| Aggregate cookie bytes | 1 MiB |
| Basic username | 4 KiB |
| Basic password | 4 KiB |
| Basic origin | 64 KiB |
| Native global storage origins JSON | 8 MiB |

The limits are admission/rejection bounds, not preallocated memory. The HTTP/1
decoder enforces the response-body cap while consumed fragments arrive,
applies the header cap cumulatively across informational/final heads and
trailers, bounds chunk-size lines to 1 KiB, and rejects bytes after its one
owned exchange. To expose a final head independently of a tiny body limit, its
wire fragment allowance is at least 64 KiB; the production transport feeds it
through one fixed 32 KiB read buffer. Redirects and handled challenges are
serial and retain no body from the discarded exchange. DNS answers and active
Happy Eyeballs attempts are capped at 64, with one pending 300 ms launch delay.
Standard decompression reads at most the decoded-body cap plus one byte, uses a 4 KiB
Brotli input buffer and rejects expansion before response registration. The
global cookie leaf retains at most 1,024 entries, rejects/ignores entries above
4 KiB and evicts oldest entries before aggregate text exceeds 1 MiB. The Node
global context retains imported origins as at most 8 MiB of JSON. The optional
client-certificate leaf accepts at most 64 identities and 16 MiB for each
certificate, key or PFX material value. It decodes only the first matching
exact origin and caches the resulting TLS configuration; unmatched origins use
the certificate-free base configuration. Duplicate normalized origins are
currently rejected, so Playwright's multiple identities for one origin remain
an explicit discrepancy. Proxy state and BrowserContext-shared cookie/origin
adapters still require explicit bounds before selection in a public artifact.

## Verification completed through NODE-223

The base release-profile crate gate passes twenty-one AAA tests covering:

- successful response metadata, body/log retention and idempotent disposal;
- context defaults, custom HTTP methods and relative URL resolution;
- invalid headers rejected before the transport is called;
- exact 3xx versus 4xx `failOnStatusCode` behavior and bounded error preview;
- timeout cancellation and `timeout: 0` completion;
- context disposal of both retained and in-flight work;
- aggregate response-body rejection without a registry leak;
- serialization of concurrent transport calls at `max_in_flight = 1`;
- disposed-context precedence over input validation;
- permanent response-identity exhaustion without wraparound or ABA reuse;
- encoded/repeated query semantics, including empty encoded-query behavior;
- repeated form fields and URL form encoding under the body limit;
- ordered multipart fields, MIME inference and terminal boundary syntax;
- case-insensitive context/request header overrides with exact content length;
- rejection before a serialized body can exceed its configured limit;
- rejection of language header values that cannot be represented on an HTTP/1
  Latin-1 wire;
- structured cookie import, cookie snapshot export and explicit rejection when
  the cookie capability is not selected.

The raw HTTP/1 crate passes thirteen additional AAA tests covering:

- exact request target/body encoding plus injected `Host` and
  `Connection: close` ownership;
- original response-header case, duplicate order and Latin-1 values;
- every two-fragment split for fixed-length and chunked bodies;
- chunk extensions and bounded discarded trailers;
- skipped `100 Continue` before the final response;
- HEAD/204 no-body rules and EOF-only close-delimited completion;
- final-head delivery before already-buffered body bytes and before a declared
  oversized redirect body is consumed;
- header/body/chunk/trailer bounds, truncated bodies and rejection of bytes
  after the final exchange.

The connection-policy crate passes twenty-two release AAA tests covering:

- raw response metadata/body/log projection through an injected connection;
- cross-origin redirects, custom Host replacement, Cookie removal and exact
  POST-to-GET content-header cleanup;
- disabled redirects versus exhausted redirect budgets;
- reset-only retry with pinned backoff and no retry for DNS failure;
- cancellation before connect; and
- bounded log truncation without changing a successful public response;
- injected final-body decoding and an independent decoded-output cap;
- Set-Cookie persistence before redirect recomposition and explicit caller
  Cookie precedence over the selected store;
- challenge-only and preemptive Basic authorization, explicit Authorization
  precedence, exact origin mismatch and one-time pre-redirect policy selection;
- discarding followed-redirect and handled-challenge bodies at final headers,
  while retaining the terminal response-body limit.

The Tokio/Rustls/decompression leaves pass ten, four and six release tests:

- in-memory Tokio read/write/cancellation and strict reset classification;
- rejection of HTTPS from the plain connector before DNS work;
- first-family-preserving IPv6/IPv4 interleaving, injected 300 ms launch
  sequencing, delayed fallback after an immediate failure, first completed
  error selection and cancellation cleanup of pending attempts;
- distinct verified/insecure Rustls configs restricted to HTTP/1.1 ALPN;
- a complete in-memory self-signed TLS handshake through the request-scoped
  insecure config, with no listener or background server;
- gzip/x-gzip, deflate and Brotli round trips, empty/unknown pass-through,
  corrupt-stream errors and decoded-expansion rejection.

The bounded cookie leaf passes nine release tests covering values containing
equal signs, default path/domain, domain/path filtering, deterministic order,
secure localhost behavior, negative Max-Age and pre-epoch Expires deletion,
source-exact name-bucket ordering and replacement, domain case/subdomains and
bounded oldest eviction.

The HTTP Basic leaf passes six release tests covering exact UTF-8 Basic output,
challenge versus preemptive modes, case-insensitive exact origin matching,
scheme/host/port mismatches, arbitrary bounded nonmatching origins, redacted
debug output and pre-encoding input caps.

The client-certificate leaf passes four release tests covering PEM and PFX
decoding, exact-origin selection, bounded/duplicate input rejection, wrong PFX
passphrases, unsupported legacy PFX and proof that malformed material is not
decoded until its origin is actually requested. The Rustls connector selects
and caches each exact-origin TLS configuration without sending an identity to
unmatched origins.

The ten API-request crates pass 104 release tests. Targeted release Clippy
with warnings denied, Node package 276/276, Python unit 12/12, strict TypeScript
and Python type/lint gates, and a real native seven-verb HTTP smoke pass. The
complete official Node global-fetch and cookie files pass 774/774 candidate
invocations with 774/774 independent oracle outcomes across the Library and
Bidi suites. The complete official Python global-fetch files pass 231/231
candidate, with the required separate async 138/138 and sync 93/93 oracle
lanes. This remains selected-file evidence, not full Playwright conformance.
The fixed official Python client-certificate selection adds 12 invocations in
each lane: eight pass and four retain the upstream macOS WebKit skips.
The exact official `fetch › validate input` client-certificate row passes 3/3
candidate and 3/3 oracle across the Library projects. Its two malformed shapes
are rejected before the optional native profile is selected. Seven exact
direct-request behaviors pass 21/21 in both lanes: no certificate, HTTP
unaffected, untrusted certificate, trusted PEM, trusted PFX, wrong PFX
passphrase and unsupported legacy PFX. A real Tailscale-bound PEM/PFX
mutual-TLS smoke independently passes. The official HTTPS-proxy-server ALPN
row also passes 3/3 in each lane. The current Core ledgers are candidate 1,222
expected / one skipped / 31 unexpected / 47,223 not-run, SHA-256
`9d7b1785ed238f69fbf2931d604aa5bd25e19d0076ecbd9e2a1234f61ceed927`,
and oracle 905 expected / 47,572 not-run, SHA-256
`041648606bf73c419e2d40dd967983063616593bf81d1969756f912dbd873afc`.

## Remaining implementation sequence

1. Extend the bounded HTTP CONNECT proxy adapter with HTTPS target tunneling,
   SOCKS, bypass and client-certificate composition; HTTP/HTTPS proxy-server
   transport for plain-HTTP targets and direct Node PEM/PFX transport are
   implemented.
2. Add the separate BrowserContext cookie adapter and shared origin state
   without introducing a browser dependency into the base crate; global
   request storage-state cookie/origin import/export is implemented.
3. Add remaining raw-header edge cases and the complete response API; shared
   JSON, form, multipart and query serialization are implemented.
4. Keep extending the implemented default-off Node and Python profiles beyond
   the now-passing global request files while preserving deterministic
   disposal and thin language conversion. Add Python client certificates and
   Playwright-compatible multiple identities for one normalized origin.
5. Run every relevant unchanged official test in both oracle and candidate
   lanes, merge every result into the full 48,477-Core and 6,526-Python
   ledgers, and keep unrelated implementation-internal cases explicit.
6. Close packaging, ABI, dependency-proof, cleanup, memory and CI gates before
   changing the compatibility table to supported.

The primary Core behavior files include `global-fetch.spec.ts`,
`global-fetch-cookie.spec.ts`, `browsercontext-fetch.spec.ts`,
`browsercontext-fetch-algorithms.spec.ts`,
`browsercontext-fetch-happy-eyeballs.spec.ts` and `fetch-proxy.spec.ts`. The
primary Python files are the async and sync
`test_fetch_global.py`/`test_fetch_browser_context.py` pairs. This list is a
work queue, not an exclusion filter: applicability and outcome continue to be
decided from the complete upstream ledgers.
