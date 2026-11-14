#!/usr/bin/env python3
"""Capture one pinned Firefox/WebKit dummy-launch case without a browser."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import secrets
import subprocess
import sys
from pathlib import Path
from typing import Any

from run_playwright_python_dummy_node_strict import (
    _artifact_payload,
    _clean_ambient,
    _fresh_result,
    _probe_sandbox,
    _sandbox_profile,
    _selected_python,
    _system_node,
    _tailscale_fixture,
)
from verify_playwright_python_dummy_engine_strict import (
    ASSET,
    COMMIT,
    KIND,
    POLICY,
    ROOT,
    SCOPE,
    TEST_NAME,
    StrictEvidenceError,
    _policy,
    _tooling,
    _upstream,
    file_sha256,
    verify_provenance,
)

OFFICIAL = ROOT / "packages/pwright-python/tests/official"
GUARD_DIR = OFFICIAL / "dummy_engine_guard_strict"


def run(args: argparse.Namespace) -> Path:
    if sys.platform != "darwin" or not Path("/usr/bin/sandbox-exec").is_file():
        raise ValueError("strict non-Chromium dummy evidence requires the macOS sandbox")
    _clean_ambient(dict(os.environ))
    if POLICY.is_symlink() or not POLICY.is_file():
        raise ValueError("strict non-Chromium dummy policy is missing or linked")
    policy_digest = file_sha256(POLICY)
    policy = _policy(json.loads(POLICY.read_text()))
    upstream = args.upstream.resolve()
    _upstream(policy, upstream, check_checkout=True)
    sources = policy["captureSourceSha256"]
    _tooling(policy, {"toolingSources": sources}, ROOT)

    python, runtime, site, version = _selected_python(args.python, policy["pythonRuntimeSha256"])
    wheel = args.wheel.absolute()
    node = _system_node()
    artifact = _artifact_payload(policy, args.mode, python, site, version, wheel, node)
    bind, host = _tailscale_fixture()
    asset = upstream / ASSET
    allowed = [
        str(python),
        str(runtime),
        str(asset),
        str(GUARD_DIR / "node"),
        "/usr/bin/env",
        "/usr/bin/python3",
        "/Library/Developer/CommandLineTools/usr/bin/python3",
        "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9",
        "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/Resources/Python.app/Contents/MacOS/Python",
        str(node),
    ]
    if args.mode == "oracle":
        allowed.append(str(site / "playwright/driver/node"))
    profile = _sandbox_profile(allowed)
    probe = _probe_sandbox(profile, python)
    result, provenance, public, process = _fresh_result(args.result)
    nonce = secrets.token_hex(16)

    python_paths = (
        [OFFICIAL / "shim", upstream, OFFICIAL / "runtime", GUARD_DIR]
        if args.mode == "candidate"
        else [OFFICIAL / "runtime", GUARD_DIR]
    )
    environment = {
        "PATH": f"{GUARD_DIR}:/opt/homebrew/bin:/usr/bin:/bin",
        "PYTHONPATH": ":".join(map(str, python_paths)),
        "PYTHONNOUSERSITE": "1",
        "PYTEST_DISABLE_PLUGIN_AUTOLOAD": "1",
        "PWRIGHT_OFFICIAL_MODE": args.mode,
        "PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned",
        "PWRIGHT_OFFICIAL_BROWSER_CHANNEL": "",
        "PWRIGHT_OFFICIAL_UPSTREAM_TESTS": str(upstream / "tests"),
        "PWRIGHT_OFFICIAL_TAILSCALE_IP": bind,
        "PWRIGHT_OFFICIAL_TAILSCALE_HOST": host,
        "PWRIGHT_PYTHON_DUMMY_UPSTREAM": str(upstream),
        "PWRIGHT_PYTHON_DUMMY_API": args.api,
        "PWRIGHT_PYTHON_DUMMY_BROWSER": args.browser,
        "PWRIGHT_PYTHON_DUMMY_LAUNCH_RECEIPT": str(public),
        "PWRIGHT_PYTHON_DUMMY_PROCESS_RECEIPT": str(process),
        "PWRIGHT_PYTHON_DUMMY_REAL_NODE": str(node),
        "PWRIGHT_PYTHON_DUMMY_NONCE": nonce,
    }
    selected = upstream / f"tests/{args.api}/test_launcher.py"
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
        (
            "pwright_candidate_runtime"
            if args.mode == "candidate"
            else "pwright_official_oracle_runtime"
        ),
        "-p",
        "pwright_official_dummy_engine_guard",
        f"--browser={args.browser}",
        "--timeout=20",
        "--timeout-method=signal",
        f"{selected}::{TEST_NAME}",
        f"--junitxml={result}",
    ]
    completed = subprocess.run(command, cwd=ROOT, env=environment, check=False, timeout=60)
    if not result.is_file() or result.is_symlink():
        raise ValueError("strict non-Chromium pytest produced no regular JUnit")

    if file_sha256(POLICY) != policy_digest:
        raise ValueError("strict non-Chromium policy changed during execution")
    _upstream(policy, upstream, check_checkout=True)
    _tooling(policy, {"toolingSources": sources}, ROOT)
    _artifact_payload(policy, args.mode, python, site, version, wheel, node)
    if file_sha256(runtime) != policy["pythonRuntimeSha256"]:
        raise ValueError("strict non-Chromium Python runtime changed during execution")
    payload: dict[str, Any] = {
        "schema": 2,
        "kind": KIND,
        "evidenceScope": SCOPE,
        "mode": args.mode,
        "api": args.api,
        "browser": args.browser,
        "testId": policy["tests"][args.api][args.browser],
        "manifest": str(POLICY),
        "manifestSha256": file_sha256(POLICY),
        "upstream": str(upstream),
        "commit": COMMIT,
        "toolingRoot": str(ROOT),
        "toolingSources": sources,
        **{key: value for key, value in artifact.items() if key != "mode"},
        "launchMode": "owned",
        "endpoint": None,
        "collectOnly": False,
        "bootstrapOnly": False,
        "fixtureBind": bind,
        "fixtureHost": host,
        "environment": environment,
        "sandboxProfile": profile,
        "sandboxProfileSha256": hashlib.sha256(profile.encode()).hexdigest(),
        "sandboxAllowedExecutables": allowed,
        "sandboxProbe": probe,
        "command": command,
        "runNonce": nonce,
        "exitCode": completed.returncode,
        "result": str(result),
        "resultSha256": file_sha256(result),
        "publicReceipt": str(public),
        "publicReceiptSha256": file_sha256(public),
        "processReceipt": str(process),
        "processReceiptSha256": file_sha256(process),
        "verifierSha256": file_sha256(
            ROOT / "scripts/verify_playwright_python_dummy_engine_strict.py"
        ),
    }
    with provenance.open("x", encoding="utf-8") as stream:
        json.dump(payload, stream, indent=2, sort_keys=True)
        stream.write("\n")
    verify_provenance(provenance)
    print(
        f"verified strict non-Chromium dummy {args.mode}/{args.api}/{args.browser}: "
        f"{policy['expectedOutcomes'][args.mode]}; {provenance}"
    )
    return provenance


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("api", choices=("async", "sync"))
    parser.add_argument("browser", choices=("firefox", "webkit"))
    parser.add_argument("upstream", type=Path)
    parser.add_argument("--python", type=Path, required=True)
    parser.add_argument("--wheel", type=Path, required=True)
    parser.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    try:
        run(args)
    except (
        OSError,
        ValueError,
        StrictEvidenceError,
        subprocess.CalledProcessError,
        subprocess.TimeoutExpired,
        json.JSONDecodeError,
    ) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
