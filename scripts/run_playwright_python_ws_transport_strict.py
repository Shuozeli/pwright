#!/usr/bin/env python3
"""Capture one unchanged official Python CDP WebSocket case with strict evidence.

The fixture binds this Mac's Tailscale IP. A pinned guard normalizes only the
test's hard-coded loopback WebSocket destination to full MagicDNS. No browser is
started, and the selected official assertion is never changed or suppressed.
"""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import os
import secrets
import socket
import subprocess
import sys
from pathlib import Path
from typing import Any, NoReturn

from verify_playwright_python_ws_transport_strict import (
    COMMIT,
    GUARD,
    KIND,
    OFFICIAL,
    POLICY,
    ROOT,
    SOURCE,
    TEST_IDS,
    StrictWebSocketEvidenceError,
    _policy,
    _runtime,
    _sources,
    _upstream,
    report_outcome,
    sandbox_profile,
    sha256_file,
    sha256_tree,
    verify_provenance,
)

FORBIDDEN_AMBIENT = (
    "CI",
    "HEADFUL",
    "NODE_OPTIONS",
    "NODE_PATH",
    "PYTHONHOME",
    "PYTHONINSPECT",
    "PYTHONPATH",
    "PYTHONUSERBASE",
    "PYTEST_ADDOPTS",
    "PYTEST_PLUGINS",
    "PLAYWRIGHT_BROWSERS_PATH",
    "PWRIGHT_CDP_ENDPOINT",
    "PWRIGHT_CDP_WS_URL",
    "PWRIGHT_OFFICIAL_LAUNCH_MODE",
    "PWDEBUG",
)


def _fail(message: str) -> NoReturn:
    raise ValueError(message)


def _clean_ambient() -> None:
    if any(key in os.environ for key in FORBIDDEN_AMBIENT) or any(
        key.startswith(("DYLD_", "LD_", "PWTEST_", "PLAYWRIGHT_", "PWRIGHT_"))
        for key in os.environ
    ):
        _fail(
            "strict WebSocket capture forbids inherited browser, loader and test overrides"
        )


def _selected_python(path: Path, expected_sha256: str) -> tuple[Path, Path, Path]:
    python = path.absolute()
    if not python.is_symlink() or not python.is_file():
        _fail("strict WebSocket capture requires a venv Python symlink")
    runtime = python.resolve()
    if not runtime.is_file() or sha256_file(runtime) != expected_sha256:
        _fail("strict WebSocket capture requires pinned CPython 3.12.14")
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
        _fail("strict WebSocket capture requires CPython 3.12.14")
    site = python.parent.parent / "lib/python3.12/site-packages"
    return python, runtime, site


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
        resolved = {
            entry[4][0]
            for entry in socket.getaddrinfo(
                host, 443, family=socket.AF_INET, type=socket.SOCK_STREAM
            )
        }
    except (
        ValueError,
        KeyError,
        TypeError,
        json.JSONDecodeError,
        socket.gaierror,
    ) as error:
        raise ValueError(
            "strict WebSocket capture cannot resolve Tailscale fixture"
        ) from error
    if (
        ip not in ipaddress.ip_network("100.64.0.0/10")
        or not host.endswith(".ts.net")
        or resolved != {address}
    ):
        _fail(
            "strict WebSocket fixture requires verified local Tailscale IPv4 and MagicDNS"
        )
    return address, host


def _fixture_port(bind: str) -> int:
    for _ in range(8):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
            listener.bind((bind, 0))
            port = listener.getsockname()[1]
        if 1024 <= port < 65535:
            return port
    _fail("strict WebSocket capture could not reserve a usable fixture port")


def _probe_sandbox(
    profile: str, python: Path, bind: str, host: str, port: int
) -> dict[str, bool]:
    script = (
        "import errno, socket, subprocess\n"
        f"bind, host, port = {bind!r}, {host!r}, {port}\n"
        "resolved = {entry[4][0] for entry in socket.getaddrinfo(host, port, family=socket.AF_INET, type=socket.SOCK_STREAM)}\n"
        "assert resolved == {bind}, 'MagicDNS differs from fixture bind'\n"
        "with socket.socket() as listener:\n"
        "    listener.bind((bind, port))\n"
        "    listener.listen()\n"
        "    with socket.socket() as allowed:\n"
        "        assert allowed.connect_ex((host, port)) == 0, 'fixture port was denied'\n"
        "with socket.socket() as denied:\n"
        "    assert denied.connect_ex((bind, port + 1)) == errno.EPERM, 'other port was allowed'\n"
        "try:\n"
        "    subprocess.run(['/usr/bin/true'], check=False)\n"
        "except PermissionError:\n"
        "    pass\n"
        "else:\n"
        "    raise AssertionError('unlisted process exec was allowed')\n"
    )
    completed = subprocess.run(
        ["/usr/bin/sandbox-exec", "-p", profile, str(python), "-c", script],
        check=False,
        capture_output=True,
        text=True,
        timeout=20,
        env={"PATH": "/usr/bin:/bin", "PYTHONNOUSERSITE": "1"},
    )
    if completed.returncode:
        _fail(
            f"strict WebSocket sandbox preflight failed: {completed.stderr or completed.stdout}"
        )
    return {
        "allowedMagicDnsConnection": True,
        "otherPortDenied": True,
        "unlistedExecDenied": True,
    }


def _fresh_result(path: Path) -> tuple[Path, Path, Path]:
    result = path.parent.resolve() / path.name
    if not result.is_absolute() or result.suffix != ".xml":
        _fail("strict WebSocket result must be an absolute XML destination")
    provenance = result.with_suffix(".provenance.json")
    receipt = result.with_suffix(".call.jsonl")
    for output in (result, provenance, receipt):
        if output.is_symlink() or output.exists():
            _fail(f"strict WebSocket output destination already exists: {output}")
    result.parent.mkdir(parents=True, exist_ok=True)
    receipt.open("x").close()
    return result, provenance, receipt


def _artifact(mode: str, python: Path, site: Path, wheel: Path) -> dict[str, Any]:
    official = site / "playwright"
    driver = official / "driver/node"
    package = site / "pwright"
    native = package / "_native.abi3.so"
    dylibs = site / "shuozeli_pwright.dylibs"
    libraries = (
        {path.name: sha256_file(path) for path in sorted(dylibs.iterdir())}
        if mode == "candidate" and dylibs.is_dir()
        else {}
    )
    return {
        "pythonExecutable": str(python),
        "pythonRuntimeSha256": sha256_file(python.resolve()),
        "pythonVersion": "3.12.14",
        "wheelPath": str(wheel),
        "wheelSha256": sha256_file(wheel),
        "playwrightPackage": str(official),
        "playwrightPackageSha256": sha256_tree(official),
        "driverNode": str(driver),
        "driverNodeSha256": sha256_file(driver),
        "pwrightPackage": str(package) if mode == "candidate" else None,
        "pwrightPackageSha256": sha256_tree(package) if mode == "candidate" else None,
        "nativePath": str(native) if mode == "candidate" else None,
        "nativeSha256": sha256_file(native) if mode == "candidate" else None,
        "runtimeLibraries": libraries,
    }


def run(args: argparse.Namespace) -> Path:
    if sys.platform != "darwin" or not Path("/usr/bin/sandbox-exec").is_file():
        _fail("strict WebSocket capture requires the macOS sandbox")
    _clean_ambient()
    if POLICY.is_symlink() or not POLICY.is_file():
        _fail("strict WebSocket policy is missing or symlinked")
    policy_digest = sha256_file(POLICY)
    policy = _policy(json.loads(POLICY.read_text()))
    upstream = args.upstream.resolve()
    _upstream(policy, upstream)
    _sources(ROOT, policy["supportSources"], "support")
    _sources(ROOT, policy["captureSourceSha256"], "capture")
    test_id = TEST_IDS[args.case]
    python, _, site = _selected_python(args.python, policy["pythonRuntimeSha256"])
    wheel = args.wheel.absolute()
    artifact = _artifact(args.mode, python, site, wheel)
    _runtime(policy, {**artifact, "mode": args.mode})
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
        "PWRIGHT_PYTHON_WS_EXPECTED_TEST": test_id,
        "PWRIGHT_PYTHON_WS_UPSTREAM": str(upstream),
        "PWRIGHT_PYTHON_WS_PORT": str(port),
        "PWRIGHT_PYTHON_WS_RECEIPT": str(receipt),
        "PWRIGHT_PYTHON_WS_NONCE": nonce,
    }
    test_name = test_id.split("::", 1)[1].removesuffix("[chromium]")
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
        "pwright_official_ws_guard",
        "--browser=chromium",
        "--timeout=20",
        "--timeout-method=signal",
        f"{upstream / SOURCE}::{test_name}",
        f"--junitxml={result}",
    ]
    completed = subprocess.run(
        command, cwd=ROOT, env=environment, check=False, timeout=75
    )
    if not result.is_file() or result.is_symlink():
        _fail("strict WebSocket pytest did not produce a regular JUnit result")
    if sha256_file(POLICY) != policy_digest:
        _fail("strict WebSocket policy changed during execution")
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
            ROOT / "scripts/verify_playwright_python_ws_transport_strict.py"
        ),
    }
    payload["outcome"] = report_outcome(payload)
    with provenance.open("x", encoding="utf-8") as stream:
        json.dump(payload, stream, indent=2, sort_keys=True)
        stream.write("\n")
    verify_provenance(provenance)
    print(f"verified strict WebSocket {args.mode}/{test_id}: {payload['outcome']}")
    return provenance


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("case", type=int, choices=(0, 1))
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
        StrictWebSocketEvidenceError,
        subprocess.CalledProcessError,
        subprocess.TimeoutExpired,
        json.JSONDecodeError,
    ) as error:
        parser.error(str(error))
    if json.loads(provenance.read_text())["outcome"] != "passed":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
