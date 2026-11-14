#!/usr/bin/env python3
"""Capture one pinned post-fix official Python dummy-engine case without Chrome."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import secrets
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

from run_playwright_python_dummy_node_strict import (
    _clean_ambient,
    _fresh_result,
    _probe_sandbox,
    _sandbox_profile,
    _selected_python,
    _system_node,
    _tailscale_fixture,
)
from verify_playwright_python_dummy_engine_strict_v2 import (
    ASSET,
    COMMIT,
    KIND,
    POLICY,
    POLICY_SHA256,
    RESULT_DIR,
    ROOT,
    SCOPE,
    TEST_NAME,
    StrictEvidenceError,
    _file,
    _policy,
    _runtime,
    _tooling,
    _upstream,
    file_sha256,
    pwright_runtime_library_paths,
    verify_provenance,
)
from verify_playwright_python_dummy_node_strict import tree_sha256

OFFICIAL = ROOT / "packages/pwright-python/tests/official"
GUARD_DIR = OFFICIAL / "dummy_engine_guard_strict_v2"


def _candidate_wheel_snapshot(result: Path, source: Path, expected: str) -> Path:
    """Publish a byte-frozen per-run wheel without replacing an existing output."""
    snapshot = result.with_suffix(".candidate-wheel.whl")
    if snapshot.is_symlink() or snapshot.exists():
        raise ValueError("v2 candidate wheel snapshot destination already exists")
    _file(source, expected, "v2 source candidate wheel")
    with source.open("rb") as incoming, snapshot.open("xb") as outgoing:
        shutil.copyfileobj(incoming, outgoing)
        outgoing.flush()
        os.fsync(outgoing.fileno())
    _file(source, expected, "v2 source candidate wheel")
    _file(snapshot, expected, "v2 candidate wheel snapshot")
    return snapshot


def _artifact_payload(
    policy: dict[str, Any],
    mode: str,
    python: Path,
    site: Path,
    version: str,
    wheel: Path | None,
    node: Path,
    result: Path,
) -> dict[str, Any]:
    official = site / "playwright"
    driver = official / "driver/node"
    package = site / "pwright"
    native = package / "_native.abi3.so"
    libraries = (
        {path.name: file_sha256(path) for path in pwright_runtime_library_paths(package)}
        if mode == "candidate"
        else {}
    )
    artifact = {
        "pythonExecutable": str(python),
        "pythonVersion": version,
        "wheelPath": str(wheel) if wheel is not None else None,
        "wheelSha256": file_sha256(wheel) if wheel is not None else None,
        "playwrightPackage": str(official),
        "playwrightPackageSha256": tree_sha256(official),
        "pwrightPackage": str(package) if mode == "candidate" else None,
        "pwrightPackageSha256": tree_sha256(package) if mode == "candidate" else None,
        "nativePath": str(native) if mode == "candidate" else None,
        "nativeSha256": file_sha256(native) if mode == "candidate" else None,
        "runtimeLibraries": libraries,
        "driverNode": str(driver),
        "driverNodeSha256": file_sha256(driver),
        "systemNode": str(node),
        "systemNodeSha256": file_sha256(node),
    }
    _runtime(policy, {**artifact, "mode": mode, "result": str(result)})
    return artifact


def run(args: argparse.Namespace) -> Path:
    if sys.platform != "darwin" or not Path("/usr/bin/sandbox-exec").is_file():
        raise ValueError("v2 non-Chromium dummy evidence requires the macOS sandbox")
    _clean_ambient(dict(os.environ))
    if not POLICY_SHA256 or POLICY.is_symlink() or not POLICY.is_file():
        raise ValueError("v2 non-Chromium policy awaits final artifact and source pins")
    _file(POLICY, POLICY_SHA256, "v2 non-Chromium strict policy")
    policy = _policy(json.loads(POLICY.read_text()))
    upstream = args.upstream.resolve()
    _upstream(policy, upstream, check_checkout=True)
    sources = policy["captureSourceSha256"]
    _tooling(policy, {"toolingSources": sources}, ROOT)
    if (args.mode == "candidate") != (args.wheel is not None):
        raise ValueError("v2 only the candidate lane accepts a wheel source")

    python, runtime, site, version = _selected_python(args.python, policy["pythonRuntimeSha256"])
    node = _system_node()
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
    requested = args.result.absolute()
    expected = RESULT_DIR / f"{args.mode}-{args.api}-{args.browser}-v1.xml"
    if requested != expected:
        raise ValueError("v2 non-Chromium result must occupy its exact immutable lane slot")
    if args.mode == "candidate":
        wheel_output = requested.with_suffix(".candidate-wheel.whl")
        if wheel_output.exists() or wheel_output.is_symlink():
            raise ValueError("v2 candidate wheel snapshot destination already exists")
    result, provenance, public, process = _fresh_result(requested)
    wheel_snapshot = (
        _candidate_wheel_snapshot(result, args.wheel.absolute(), policy["candidateWheelSha256"])
        if args.mode == "candidate"
        else None
    )
    artifact = _artifact_payload(
        policy, args.mode, python, site, version, wheel_snapshot, node, result
    )
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
        "pwright_candidate_runtime"
        if args.mode == "candidate"
        else "pwright_official_oracle_runtime",
        "-p",
        "pwright_official_dummy_engine_guard_v2",
        f"--browser={args.browser}",
        "--timeout=20",
        "--timeout-method=signal",
        f"{selected}::{TEST_NAME}",
        f"--junitxml={result}",
    ]
    completed = subprocess.run(command, cwd=ROOT, env=environment, check=False, timeout=60)
    if not result.is_file() or result.is_symlink():
        raise ValueError("v2 non-Chromium pytest produced no regular JUnit")
    _file(POLICY, POLICY_SHA256, "v2 non-Chromium strict policy")
    _upstream(policy, upstream, check_checkout=True)
    _tooling(policy, {"toolingSources": sources}, ROOT)
    _artifact_payload(policy, args.mode, python, site, version, wheel_snapshot, node, result)
    if args.mode == "candidate":
        _file(
            args.wheel.absolute(),
            policy["candidateWheelSha256"],
            "v2 source candidate wheel",
        )
    payload: dict[str, Any] = {
        "schema": 3,
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
        **artifact,
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
            ROOT / "scripts/verify_playwright_python_dummy_engine_strict_v2.py"
        ),
    }
    with provenance.open("x", encoding="utf-8") as stream:
        json.dump(payload, stream, indent=2, sort_keys=True)
        stream.write("\n")
    verify_provenance(provenance)
    print(f"verified v2 non-Chromium dummy {args.mode}/{args.api}/{args.browser}: {provenance}")
    return provenance


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("api", choices=("async", "sync"))
    parser.add_argument("browser", choices=("firefox", "webkit"))
    parser.add_argument("upstream", type=Path)
    parser.add_argument("--python", type=Path, required=True)
    parser.add_argument("--wheel", type=Path)
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
