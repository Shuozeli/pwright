#!/usr/bin/env python3
"""Capture one unchanged official BrowserType.connect early-WS-close case.

This version stays disabled until its opt-in candidate wheel and source hashes
are pinned. It cannot launch a browser: the sandbox only admits the fixture
port and the selected Python/official driver executables.
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
    _fixture_port,
    _probe_sandbox,
    _selected_python,
    _tailscale_fixture,
)
from verify_playwright_python_connect_close_strict_v2 import (
    COMMIT,
    GUARD,
    KIND,
    NAME,
    OFFICIAL,
    POLICY,
    POLICY_SHA256,
    ROOT,
    SCOPE,
    SOURCE,
    TEST_IDS,
    StrictConnectCloseV2EvidenceError,
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
    raise StrictConnectCloseV2EvidenceError(message)


def _fresh_result_v2(path: Path) -> tuple[Path, Path, Path]:
    result = path.parent.resolve() / path.name
    if not result.is_absolute() or result.suffix != ".xml":
        _fail("strict connect-close v2 result must be an absolute XML destination")
    provenance = result.with_suffix(".provenance.json")
    receipt = result.with_suffix(".call.jsonl")
    for output in (result, provenance, receipt):
        if output.is_symlink() or output.exists():
            _fail(f"strict connect-close v2 output already exists: {output}")
    result.parent.mkdir(parents=True, exist_ok=True)
    # Reserve the JUnit path before pytest can write to it. A concurrent or
    # later run sees the reservation and must choose a new output basename.
    result.open("xb").close()
    receipt.open("xb").close()
    return result, provenance, receipt


def run(args: argparse.Namespace) -> Path:
    if sys.platform != "darwin" or not Path("/usr/bin/sandbox-exec").is_file():
        _fail("strict connect-close v2 capture requires the macOS sandbox")
    _clean_ambient()
    if POLICY.is_symlink() or not POLICY.is_file() or sha256_file(POLICY) != POLICY_SHA256:
        _fail("strict connect-close v2 policy is missing or not frozen")
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
    if artifact["wheelSha256"] != policy["candidateWheelSha256"]:
        _fail("strict connect-close v2 requires the frozen candidate wheel")
    _runtime(policy, {**artifact, "mode": args.mode})
    if args.mode == "candidate" and (
        artifact["pwrightPackageSha256"] != policy["candidatePackageTreeSha256"]
        or artifact["nativeSha256"] != policy["candidateNativeSha256"]
        or artifact["runtimeLibraries"] != policy["candidateRuntimeLibraries"]
    ):
        _fail("strict connect-close v2 installed wheel differs from policy")
    if not _method_probe({"pythonExecutable": str(python), "mode": args.mode}):
        _fail("strict connect-close v2 requires public BrowserType.connect in both lanes")
    bind, host = _tailscale_fixture()
    port = _fixture_port(bind)
    profile, allowed = sandbox_profile(python, Path(artifact["driverNode"]), args.mode, port)
    probe = _probe_sandbox(profile, python, bind, host, port)
    result, provenance, receipt = _fresh_result_v2(args.result)
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
        "PWRIGHT_PYTHON_CONNECT_V2_EXPECTED_TEST": test_id,
        "PWRIGHT_PYTHON_CONNECT_V2_UPSTREAM": str(upstream),
        "PWRIGHT_PYTHON_CONNECT_V2_PORT": str(port),
        "PWRIGHT_PYTHON_CONNECT_V2_RECEIPT": str(receipt),
        "PWRIGHT_PYTHON_CONNECT_V2_NONCE": nonce,
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
        "pwright_official_connect_guard_v2",
        f"--browser={engine}",
        "--timeout=20",
        "--timeout-method=signal",
        f"{upstream / SOURCE}::{NAME}",
        f"--junitxml={result}",
    ]
    completed = subprocess.run(command, cwd=ROOT, env=environment, check=False, timeout=75)
    if not result.is_file() or result.is_symlink():
        _fail("strict connect-close v2 produced no regular JUnit")
    if sha256_file(POLICY) != POLICY_SHA256:
        _fail("strict connect-close v2 policy changed during execution")
    _upstream(policy, upstream)
    _sources(ROOT, policy["supportSources"], "support")
    _sources(ROOT, policy["captureSourceSha256"], "capture")
    _runtime(policy, {**_artifact(args.mode, python, site, wheel), "mode": args.mode})
    payload = {
        "schema": 2,
        "kind": KIND,
        "evidenceScope": SCOPE,
        "mode": args.mode,
        "testId": test_id,
        "manifest": str(POLICY),
        "manifestSha256": POLICY_SHA256,
        "upstream": str(upstream),
        "commit": COMMIT,
        "toolingRoot": str(ROOT),
        "toolingSources": policy["captureSourceSha256"],
        **artifact,
        "publicConnectPresent": True,
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
            ROOT / "scripts/verify_playwright_python_connect_close_strict_v2.py"
        ),
    }
    payload["outcome"] = report_outcome(payload)
    with provenance.open("x", encoding="utf-8") as stream:
        json.dump(payload, stream, indent=2, sort_keys=True)
        stream.write("\n")
    verify_provenance(provenance)
    print(f"verified strict connect-close v2 {args.mode}/{test_id}: {payload['outcome']}")
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
