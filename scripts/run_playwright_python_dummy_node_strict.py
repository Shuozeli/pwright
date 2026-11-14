#!/usr/bin/env python3
"""Capture one exact official Python dummy-launch case with strict receipts.

This intentionally cannot reuse the older diagnostic result. Each invocation
executes a fresh pinned upstream case in an exec/network-denying macOS sandbox.
The candidate identity is the installed ABI119 wheel, not current Rust source.
"""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import os
import secrets
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any, NoReturn

from verify_playwright_python_dummy_node_strict import (
    ASSET,
    COMMIT,
    KIND,
    POLICY,
    ROOT,
    SCOPE,
    TEST_NAME,
    StrictEvidenceError,
    _policy,
    _runtime,
    _tooling,
    _upstream,
    file_sha256,
    pwright_runtime_library_paths,
    tree_sha256,
    verify_provenance,
)

OFFICIAL = ROOT / "packages/pwright-python/tests/official"
GUARD_DIR = OFFICIAL / "dummy_guard_strict"
FORBIDDEN_AMBIENT = (
    "CI",
    "NODE_OPTIONS",
    "NODE_PATH",
    "PYTHONHOME",
    "PYTHONINSPECT",
    "PYTHONPATH",
    "PYTHONUSERBASE",
    "PYTEST_ADDOPTS",
    "PYTEST_PLUGINS",
    "HEADFUL",
    "PLAYWRIGHT_BROWSERS_PATH",
    "PWRIGHT_CDP_ENDPOINT",
    "PWRIGHT_CDP_WS_URL",
    "PWRIGHT_OFFICIAL_LAUNCH_MODE",
    "PWDEBUG",
)


def _fail(message: str) -> NoReturn:
    raise ValueError(message)


def _clean_ambient(environment: dict[str, str]) -> None:
    if any(key in environment for key in FORBIDDEN_AMBIENT) or any(
        key.startswith(("DYLD_", "LD_", "PWTEST_", "PLAYWRIGHT_", "PWRIGHT_"))
        for key in environment
    ):
        _fail("strict dummy run forbids inherited test, browser and loader overrides")


def _selected_python(path: Path, expected_sha256: str) -> tuple[Path, Path, Path, str]:
    python = path.absolute()
    if not python.is_symlink() or not python.is_file():
        _fail("strict dummy run requires a regular venv Python symlink")
    runtime = python.resolve()
    if not runtime.is_file() or not os.access(runtime, os.X_OK):
        _fail("strict dummy run Python runtime is missing")
    if file_sha256(runtime) != expected_sha256:
        _fail("strict dummy run refuses an unpinned Python runtime")
    version = subprocess.run(
        [
            str(python),
            "-c",
            "import sys; print('.'.join(map(str, sys.version_info[:3])))",
        ],
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
        env={"PATH": "/usr/bin:/bin", "PYTHONNOUSERSITE": "1"},
    ).stdout.strip()
    if version != "3.12.14":
        _fail("strict dummy run requires CPython 3.12.14")
    site = python.parent.parent / "lib/python3.12/site-packages"
    return python, runtime, site, version


def _system_node() -> Path:
    command = shutil.which("node", path="/opt/homebrew/bin:/usr/bin:/bin")
    if command is None:
        _fail("strict dummy run requires the pinned system Node")
    node = Path(command).resolve()
    if not node.is_absolute() or not node.is_file() or not os.access(node, os.X_OK):
        _fail("strict dummy run system Node is missing")
    return node


def _tailscale_fixture() -> tuple[str, str]:
    address = subprocess.run(
        ["tailscale", "ip", "-4"],
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
    ).stdout.strip()
    status = subprocess.run(
        ["tailscale", "status", "--json"],
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
    ).stdout
    try:
        ip = ipaddress.ip_address(address)
        host = str(json.loads(status)["Self"]["DNSName"]).rstrip(".")
    except (ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise ValueError("strict dummy run cannot resolve Tailscale fixture") from error
    if ip not in ipaddress.ip_network("100.64.0.0/10") or not host.endswith(".ts.net"):
        _fail("strict dummy run requires Tailscale IPv4 and full MagicDNS")
    return address, host


def _artifact_payload(
    policy: dict[str, Any],
    mode: str,
    python: Path,
    site: Path,
    version: str,
    wheel: Path,
    node: Path,
) -> dict[str, Any]:
    official = site / "playwright"
    driver = official / "driver/node"
    package = site / "pwright"
    native = package / "_native.abi3.so"
    libraries = (
        {
            path.name: file_sha256(path)
            for path in pwright_runtime_library_paths(package)
        }
        if mode == "candidate"
        else {}
    )
    payload = {
        "pythonExecutable": str(python),
        "pythonVersion": version,
        "wheelPath": str(wheel),
        "wheelSha256": file_sha256(wheel),
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
        "mode": mode,
    }
    _runtime(policy, payload)
    return payload


def _sandbox_profile(allow: list[str]) -> str:
    if len(allow) != len(set(allow)) or any(
        not Path(path).is_absolute() for path in allow
    ):
        _fail("strict dummy sandbox executable allowlist is invalid")
    return "\n".join(
        [
            "(version 1)",
            "(allow default)",
            "(deny process-exec)",
            "(deny network-outbound)",
        ]
        + [f"(allow process-exec (literal {json.dumps(path)}))" for path in allow]
    )


def _probe_sandbox(profile: str, python: Path) -> dict[str, bool]:
    script = (
        "import errno, socket, subprocess\n"
        "try:\n"
        "    subprocess.run(['/usr/bin/true'], check=False)\n"
        "except PermissionError:\n"
        "    pass\n"
        "else:\n"
        "    raise SystemExit('exec deny was ineffective')\n"
        "with socket.socket() as connection:\n"
        "    if connection.connect_ex(('127.0.0.1', 9)) != errno.EPERM:\n"
        "        raise SystemExit('network deny was ineffective')\n"
    )
    completed = subprocess.run(
        ["/usr/bin/sandbox-exec", "-p", profile, str(python), "-c", script],
        check=False,
        capture_output=True,
        text=True,
        timeout=15,
        env={"PATH": "/usr/bin:/bin", "PYTHONNOUSERSITE": "1"},
    )
    if completed.returncode:
        _fail(
            f"strict dummy sandbox preflight failed: {completed.stderr or completed.stdout}"
        )
    return {"execDenied": True, "networkDenied": True}


def _fresh_result(path: Path) -> tuple[Path, Path, Path, Path]:
    result = path.parent.resolve() / path.name
    if not result.is_absolute() or result.suffix != ".xml":
        _fail("strict dummy result must be an absolute XML destination")
    provenance = result.with_suffix(".provenance.json")
    public = result.with_suffix(".public-launch.jsonl")
    process = result.with_suffix(".dummy-process.jsonl")
    for output in (result, provenance, public, process):
        if output.is_symlink() or output.exists():
            _fail(f"strict dummy output destination already exists: {output}")
    result.parent.mkdir(parents=True, exist_ok=True)
    public.open("x").close()
    process.open("x").close()
    return result, provenance, public, process


def run(args: argparse.Namespace) -> Path:
    if sys.platform != "darwin" or not Path("/usr/bin/sandbox-exec").is_file():
        _fail("strict dummy evidence requires the macOS sandbox")
    _clean_ambient(dict(os.environ))
    if POLICY.is_symlink() or not POLICY.is_file():
        _fail("strict dummy policy is missing or linked")
    policy_digest = file_sha256(POLICY)
    policy = _policy(json.loads(POLICY.read_text()))
    upstream = args.upstream.resolve()
    _upstream(policy, upstream, check_checkout=True)
    sources = policy["captureSourceSha256"]
    _tooling(policy, {"toolingSources": sources}, ROOT)

    python, runtime, site, version = _selected_python(
        args.python, policy["pythonRuntimeSha256"]
    )
    wheel = args.wheel.absolute()
    node = _system_node()
    artifact = _artifact_payload(policy, args.mode, python, site, version, wheel, node)
    if file_sha256(runtime) != policy["pythonRuntimeSha256"]:
        _fail("strict dummy Python runtime differs from policy")
    bind, host = _tailscale_fixture()
    asset = upstream / ASSET
    allow = [
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
        allow.append(str(site / "playwright/driver/node"))
    profile = _sandbox_profile(allow)
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
        "pwright_official_dummy_guard",
        "--browser=chromium",
        "--timeout=20",
        "--timeout-method=signal",
        f"{selected}::{TEST_NAME}",
        f"--junitxml={result}",
    ]
    completed = subprocess.run(
        command, cwd=ROOT, env=environment, check=False, timeout=60
    )
    if not result.is_file() or result.is_symlink():
        _fail("strict dummy pytest did not produce a regular JUnit result")

    # Recheck every mutable input before emitting a sidecar; outputs are never
    # overwritten, so a failed run remains inspectable but not promotable.
    if file_sha256(POLICY) != policy_digest:
        _fail("strict dummy policy changed during execution")
    _upstream(policy, upstream, check_checkout=True)
    _tooling(policy, {"toolingSources": sources}, ROOT)
    _artifact_payload(policy, args.mode, python, site, version, wheel, node)
    payload = {
        "schema": 2,
        "kind": KIND,
        "evidenceScope": SCOPE,
        "mode": args.mode,
        "api": args.api,
        "testId": policy["tests"][args.api],
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
        "sandboxAllowedExecutables": allow,
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
            ROOT / "scripts/verify_playwright_python_dummy_node_strict.py"
        ),
    }
    with provenance.open("x", encoding="utf-8") as stream:
        json.dump(payload, stream, indent=2, sort_keys=True)
        stream.write("\n")
    verify_provenance(provenance)
    print(f"verified strict dummy launch {args.mode}/{args.api}: {provenance}")
    return provenance


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("api", choices=("async", "sync"))
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
