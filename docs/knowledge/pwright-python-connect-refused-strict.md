<!-- agent-updated: 2026-10-12T08:26:31Z -->

# Official Python closed-endpoint connect slice

Status: **six candidate and six oracle cases passed** on 2026-10-12 in
separate isolated CPython 3.12.14 environments. The versioned policy at
`packages/pwright-python/tests/official/browser-connect-refused-strict-v1-manifest.json`
is capture-ready and frozen at SHA-256
`87f44977cc9d66b641e03b999e0178c518c50e9ef5014bd2e897afce1b42976b`.
The selected release wheel has SHA-256
`3b86f0b56c6dbc2654a3326cd91b62aed0ed2b8d22537e69476226565bfed6d1`
and reports ABI123 with `playwright-server-initialize` selected. The 12
single-case JUnit files, call receipts, and provenance sidecars are under
`.work/official-conformance/results/python-connect-refused-strict-v1/`.
All 12 sidecars independently reverified with the frozen policy and exactly
six expanded IDs per lane. The immutable 6,526-row [paired successor](../../.work/official-conformance/outcomes/python-connect-refused-strict-paired-v1/promotion-receipt.json)
has receipt SHA-256 `4979cdc982cf716e5124bf00b972594c27a65fc26104c193ed12957cf5d4c060`.
It promotes only these six rows per lane; the other 6,520 are byte-identical
to the preserved connect-close v2 predecessor. Candidate is 1,542 passed,
102 failed, three errors, 11 skipped and 4,868 not run (ledger SHA-256
`3095771a0928ecc076ecdec8eee5d2946d70de9dbd1ea15e00f61ad7fa79e836`).
Oracle is 1,309 passed, four failed, 11 skipped and 5,202 not run (ledger
SHA-256 `11afbc5f482a3d9cefa36c59598f1771903b74f93749dcd11e8b5b178a7a6208`).
The existing ledger row schema does not encode the fixture variant. Its
`passed` outcome must be read with the receipt's fixture-only scope and
`publicApiParityEvidence=false`, not as full server compatibility.

The pinned upstream repository is `microsoft/playwright-python` v1.58.0 at
`47a5d35ef4f815a2021349f86ae391f7c20c02d6`. The exact six expanded IDs
are `test_connect_to_closed_server_without_hangs[chromium|firefox|webkit]`
in each of `tests/async/test_browsertype_connect.py` and
`tests/sync/test_browsertype_connect.py`. The six were `not-run` in both
predecessor 6,526-row ledgers before this capture. The official assertion
requires the public `BrowserType.connect` error message to contain
`WebSocket error: `; the test body and assertion are not edited.

The original upstream `launch_server` fixture constructs `RemoteServer`,
which launches Node `cli.js launch-server --browser ...` before the test
immediately calls `kill()`. A browser-free run is therefore **not** the
unmodified upstream fixture end to end. This slice replaces only that
`RemoteServer` class in the isolated pytest process with a real TCP listener
bound to the verified local Tailscale IPv4 address. It publishes a full
MagicDNS `ws://` endpoint, proves it accepts a connection, then closes it
when the unchanged test calls `kill()`. Both immediately after `kill()` and
immediately before the unchanged public `connect` call, the guard requires a
real socket attempt to return `ECONNREFUSED`. The guard records the original
public method's actual error without manufacturing a replacement exception.
The refusal probes prove endpoint state, not by themselves that the native
client sent a TCP SYN; the candidate's typed transport path must also be
checked in source and focused runtime tests.
The harness pins the source, wheel, native module, packaged driver and Python
runtime, but does not separately hash every pytest plugin or `.pth`/`.pyc`
file; this is conformance evidence, not an adversarially hermetic execution
attestation.
This is evidence only for connection-refusal behavior, not BrowserServer
launch, a successful remote Browser, or general Playwright Server support.

`scripts/run_playwright_python_connect_refused_strict_v1.py` runs exactly one
selected official ID per process. It requires separate installed-wheel
candidate and official-oracle Python 3.12 environments, pinned source and
artifact hashes, a fresh no-clobber XML destination, and a macOS sandbox
allowing only the fixture network port plus the selected Python executable
and oracle driver Node. Browser executable startup is not permitted. Its
nonce-bound receipt records, in order, listener reachability, listener
shutdown/refusal, the public call/refusal preflight, and the returned public
Error. The independent
`scripts/verify_playwright_python_connect_refused_strict_v1.py` checks the
source/artifact pins, sandbox, exact single-case JUnit identity, ordered
receipt, and no skipped tests. A red candidate result may be preserved as
evidence; only the passing paired results were promoted by
`scripts/promote_playwright_python_connect_refused_strict_v1.py`. Its 15
synthetic controls and the read-only `--verify-existing` run pass.

The portable synthetic verifier/guard checks are in
`scripts/test_verify_playwright_python_connect_refused_strict_v1.py`. They
exercise policy fail-closed behavior, no-clobber outputs, async/sync JUnit
identity, receipt tamper rejection, and fixture/public-call sequencing. They
are synthetic controls distinct from the 12 official case captures.
