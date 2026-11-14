#!/usr/bin/env python3
"""Capture one unchanged official BrowserType.connect close-error test.

Each run uses a pinned ABI121 wheel, clean upstream source, owned official
fixture bound to the verified Tailscale IP, and a port-scoped macOS sandbox.
Only the literal localhost WebSocket URL is forwarded to full MagicDNS.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import secrets
import subprocess
import sys
from pathlib import Path
from typing import NoReturn

from run_playwright_python_ws_transport_strict import (
    _artifact,
    _clean_ambient,
    _fresh_result,
    _fixture_port,
    _probe_sandbox,
    _selected_python,
    _tailscale_fixture,
)
from verify_playwright_python_connect_close_strict import (
    COMMIT,
    GUARD,
    KIND,
    NAME,
    OFFICIAL,
    POLICY,
    ROOT,
    SOURCE,
    TEST_IDS,
    StrictBrowserConnectEvidenceError,
    _method_probe,
    _policy,
    _runtime,
    _sources,
    _upstream,
    report_outcome,
    sandbox_profile,
    sha256_file,
    verify_provenance,
)


def _fail(message: str) -> NoReturn:
    raise StrictBrowserConnectEvidenceError(message)


def run(args: argparse.Namespace) -> Path:
    if sys.platform != "darwin" or not Path("/usr/bin/sandbox-exec").is_file():
        _fail("strict BrowserType.connect capture requires the macOS sandbox")
    _clean_ambient()
    if POLICY.is_symlink() or not POLICY.is_file():
        _fail("strict BrowserType.connect policy is missing or symlinked")
    policy_digest = sha256_file(POLICY)
    policy = _policy(json.loads(POLICY.read_text()))
    upstream = args.upstream.resolve()
    _upstream(policy, upstream)
    _sources(ROOT, policy["supportSources"], "support")
    _sources(ROOT, policy["captureSourceSha256"], "capture")
    test_id = TEST_IDS[args.case]
    engine = test_id.rsplit("[", 1)[1].removesuffix("]")
    python, _, site = _selected_python(args.python, policy["pythonRuntimeSha256"])
    wheel = args.wheel.absolute()
    artifact = _artifact(args.mode, python, site, wheel)
    if artifact["wheelSha256"] != policy["wheelSha256"]:
        _fail("strict BrowserType.connect capture requires the frozen ABI121 wheel")
    _runtime(policy, {**artifact, "mode": args.mode})
    present = _method_probe({"pythonExecutable": str(python), "mode": args.mode})
    if present != (args.mode == "oracle"):
        _fail("public BrowserType.connect presence differs from the frozen lanes")
    bind, host = _tailscale_fixture()
    port = _fixture_port(bind)
    profile, allowed = sandbox_profile(
        python, Path(artifact["driverNode"]), args.mode, port
    )
    probe = _probe_sandbox(profile, python, bind, host, port)
    result, provenance, receipt = _fresh_result(args.result)
    nonce = secrets.token_hex(16)
    paths = (
        [OFFICIAL / "shim", upstream, OFFICIAL / "runtime", GUARD]
        if args.mode == "candidate"
        else [OFFICIAL / "runtime", GUARD]
    )
    environment = {
        "PATH": "/opt/homebrew/bin:/usr/bin:/bin",
        "PYTHONPATH": ":".join(map(str, paths)),
        "PYTHONNOUSERSITE": "1",
        "PYTEST_DISABLE_PLUGIN_AUTOLOAD": "1",
        "PWRIGHT_OFFICIAL_MODE": args.mode,
        "PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned",
        "PWRIGHT_OFFICIAL_BROWSER_CHANNEL": "",
        "PWRIGHT_OFFICIAL_UPSTREAM_TESTS": str(upstream / "tests"),
        "PWRIGHT_OFFICIAL_TAILSCALE_IP": bind,
        "PWRIGHT_OFFICIAL_TAILSCALE_HOST": host,
        "PWRIGHT_PYTHON_CONNECT_EXPECTED_TEST": test_id,
        "PWRIGHT_PYTHON_CONNECT_UPSTREAM": str(upstream),
        "PWRIGHT_PYTHON_CONNECT_PORT": str(port),
        "PWRIGHT_PYTHON_CONNECT_RECEIPT": str(receipt),
        "PWRIGHT_PYTHON_CONNECT_NONCE": nonce,
    }
    command = [
        "/usr/bin/sandbox-exec",
        "-p",
        profile,
        str(python),
        "-m",
        "pytest",
        "-q",
        "--import-mode=importlib",
        "-c",
        str(upstream / "pyproject.toml"),
        "-p",
        "pytest_asyncio.plugin",
        "-p",
        "pytest_timeout",
        "-p",
        "pwright_official_server_runtime",
        "-p",
        "pwright_candidate_runtime"
        if args.mode == "candidate"
        else "pwright_official_oracle_runtime",
        "-p",
        "pwright_official_connect_guard",
        f"--browser={engine}",
        "--timeout=20",
        "--timeout-method=signal",
        f"{upstream / SOURCE}::{NAME}",
        f"--junitxml={result}",
    ]
    completed = subprocess.run(
        command, cwd=ROOT, env=environment, check=False, timeout=75
    )
    if not result.is_file() or result.is_symlink():
        _fail("strict BrowserType.connect pytest produced no regular JUnit result")
    if sha256_file(POLICY) != policy_digest:
        _fail("strict BrowserType.connect policy changed during execution")
    _upstream(policy, upstream)
    _sources(ROOT, policy["supportSources"], "support")
    _sources(ROOT, policy["captureSourceSha256"], "capture")
    _runtime(policy, {**_artifact(args.mode, python, site, wheel), "mode": args.mode})
    payload = {
        "schema": 1,
        "kind": KIND,
        "mode": args.mode,
        "testId": test_id,
        "manifest": str(POLICY),
        "manifestSha256": policy_digest,
        "upstream": str(upstream),
        "commit": COMMIT,
        "toolingRoot": str(ROOT),
        "toolingSources": policy["captureSourceSha256"],
        **artifact,
        "publicConnectPresent": present,
        "fixtureBind": bind,
        "fixtureHost": host,
        "fixturePort": port,
        "fixtureTransportAdaptation": policy["fixtureTransportAdaptation"],
        "environment": environment,
        "sandboxProfile": profile,
        "sandboxProfileSha256": hashlib.sha256(profile.encode()).hexdigest(),
        "sandboxAllowedExecutables": allowed,
        "sandboxProbe": probe,
        "command": command,
        "runNonce": nonce,
        "exitCode": completed.returncode,
        "outcome": "pending",
        "result": str(result),
        "resultSha256": sha256_file(result),
        "callReceipt": str(receipt),
        "callReceiptSha256": sha256_file(receipt),
        "verifierSha256": sha256_file(
            ROOT / "scripts/verify_playwright_python_connect_close_strict.py"
        ),
    }
    payload["outcome"] = report_outcome(payload)
    with provenance.open("x", encoding="utf-8") as stream:
        json.dump(payload, stream, indent=2, sort_keys=True)
        stream.write("\n")
    verify_provenance(provenance)
    print(
        f"verified strict BrowserType.connect {args.mode}/{test_id}: {payload['outcome']}"
    )
    return provenance


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("case", type=int, choices=(0, 1, 2))
    parser.add_argument("upstream", type=Path)
    parser.add_argument("--python", type=Path, required=True)
    parser.add_argument("--wheel", type=Path, required=True)
    parser.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    try:
        provenance = run(args)
    except (
        OSError,
        ValueError,
        subprocess.CalledProcessError,
        subprocess.TimeoutExpired,
        json.JSONDecodeError,
    ) as error:
        parser.error(str(error))
    if json.loads(provenance.read_text())["outcome"] != "passed":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
