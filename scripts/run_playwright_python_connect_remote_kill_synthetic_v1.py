#!/usr/bin/env python3
"""Capture four unchanged upstream remote-kill cases with a synthetic fixture.

This diagnostic never promotes rows to the official conformance ledger.  A
candidate failure is retained as RED evidence, not treated as capture failure.
"""

from __future__ import annotations

import argparse
import ipaddress
import json
import os
import shutil
import signal
import socket
import subprocess
from pathlib import Path
from typing import Any

from verify_playwright_python_connect_remote_kill_synthetic_v1 import (
    CORE,
    HARNESS_INPUTS,
    LEGACY_DIR,
    MANIFEST,
    PLUGIN_DIR,
    RESULT_PARENT,
    ROOT,
    RUNTIME,
    SERVER,
    SHIM,
    UPSTREAM,
    WHEEL_NAME,
    WHEEL_SHA256,
    expected_command,
    junit,
    sha256,
    verify_directory,
    verify_sources,
)

FORBIDDEN_INHERITED = (
    "CI", "NODE_OPTIONS", "NODE_PATH", "PYTHONHOME", "PYTHONPATH",
    "PYTHONUSERBASE", "PYTEST_ADDOPTS", "PYTEST_PLUGINS",
    "PWRIGHT_CDP_ENDPOINT", "PWRIGHT_CDP_WS_URL",
    "PWRIGHT_PYTHON_CLOSE_EVENTS_MODE", "PWRIGHT_PYTHON_CLOSE_EVENTS_RECEIPT",
    "PWRIGHT_PYTHON_CLOSE_EVENTS_SERVER", "PWRIGHT_PYTHON_CLOSE_EVENTS_UPSTREAM",
    "PWRIGHT_PYTHON_CLOSE_EVENTS_CORE_UPSTREAM",
    "PWRIGHT_PYTHON_CLOSE_EVENTS_ENGINE", "PWRIGHT_PYTHON_CLOSE_EVENTS_ENGINE_FILTER",
    "PWRIGHT_OFFICIAL_TAILSCALE_IP", "PWRIGHT_OFFICIAL_TAILSCALE_HOST",
)


def setup_command(command: list[str], *, timeout: int = 120) -> str:
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False, timeout=timeout)
    if result.returncode:
        raise RuntimeError(
            f"setup command failed ({result.returncode}): {command[:4]}\n{result.stderr[-3000:]}"
        )
    return result.stdout + result.stderr


def fixture_address() -> tuple[str, str]:
    bind = setup_command(["tailscale", "ip", "-4"]).strip()
    status = json.loads(setup_command(["tailscale", "status", "--json"]))
    host = str(status["Self"]["DNSName"]).removesuffix(".")
    if ipaddress.ip_address(bind) not in ipaddress.ip_network("100.64.0.0/10"):
        raise ValueError("synthetic fixture requires this Mac's Tailscale IPv4")
    if not host.endswith(".ts.net") or "/" in host or ":" in host:
        raise ValueError("synthetic fixture requires a full Tailscale MagicDNS hostname")
    addresses = {item[4][0] for item in socket.getaddrinfo(host, 80, type=socket.SOCK_STREAM)}
    if bind not in addresses:
        raise ValueError("MagicDNS does not resolve to the Tailscale bind")
    return bind, host


def run_lane(python: Path, result_dir: Path, mode: str, bind: str, host: str) -> dict[str, str]:
    receipt = result_dir / f"{mode}.receipt.jsonl"
    xml = result_dir / f"{mode}.xml"
    log = result_dir / f"{mode}.log"
    provenance = result_dir / f"{mode}.provenance.json"
    env = {key: value for key, value in os.environ.items() if key not in FORBIDDEN_INHERITED}
    env.update({
        "PYTHONNOUSERSITE": "1",
        "PYTHONPATH": os.pathsep.join(
            [str(SHIM), str(RUNTIME), str(PLUGIN_DIR), str(LEGACY_DIR)]
            if mode == "candidate" else [str(RUNTIME), str(PLUGIN_DIR), str(LEGACY_DIR)]
        ),
        "PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned",
        "PWRIGHT_OFFICIAL_BROWSER_CHANNEL": "",
        "PWRIGHT_OFFICIAL_TAILSCALE_IP": bind,
        "PWRIGHT_OFFICIAL_TAILSCALE_HOST": host,
        "PWRIGHT_PYTHON_CLOSE_EVENTS_MODE": mode,
        "PWRIGHT_PYTHON_CLOSE_EVENTS_ENGINE_FILTER": "chromium",
        "PWRIGHT_PYTHON_CLOSE_EVENTS_ENGINE": "chromium",
        "PWRIGHT_PYTHON_CLOSE_EVENTS_RECEIPT": str(receipt),
        "PWRIGHT_PYTHON_CLOSE_EVENTS_SERVER": str(SERVER),
        "PWRIGHT_PYTHON_CLOSE_EVENTS_UPSTREAM": str(UPSTREAM),
        "PWRIGHT_PYTHON_CLOSE_EVENTS_CORE_UPSTREAM": str(CORE),
    })
    command = expected_command(python, xml, mode)
    process = subprocess.Popen(
        command, cwd=ROOT, env=env, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, text=True, start_new_session=True,
    )
    try:
        stdout, _ = process.communicate(timeout=240)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        stdout, _ = process.communicate()
        with log.open("x") as stream:
            stream.write(stdout)
        raise RuntimeError(f"{mode} diagnostic exceeded bounded timeout; see {log}") from None
    with log.open("x") as stream:
        stream.write(stdout)
    if not xml.is_file() or not receipt.is_file():
        raise RuntimeError(f"{mode} diagnostic produced incomplete evidence; see {log}")
    file_hashes = {
        suffix: sha256(result_dir / f"{mode}.{suffix}")
        for suffix in ("xml", "log", "receipt.jsonl")
    }
    record = {
        "schema": 1, "mode": mode, "command": command,
        "exitCode": process.returncode, "wheelSha256": WHEEL_SHA256,
        "environment": {
            key: env[key] for key in (
                "PYTHONPATH", "PWRIGHT_OFFICIAL_LAUNCH_MODE",
                "PWRIGHT_OFFICIAL_TAILSCALE_IP", "PWRIGHT_OFFICIAL_TAILSCALE_HOST",
                "PWRIGHT_PYTHON_CLOSE_EVENTS_MODE",
                "PWRIGHT_PYTHON_CLOSE_EVENTS_ENGINE_FILTER",
                "PWRIGHT_PYTHON_CLOSE_EVENTS_RECEIPT",
                "PWRIGHT_PYTHON_CLOSE_EVENTS_SERVER",
                "PWRIGHT_PYTHON_CLOSE_EVENTS_UPSTREAM",
                "PWRIGHT_PYTHON_CLOSE_EVENTS_CORE_UPSTREAM",
            )
        },
        "fileSha256": file_hashes,
    }
    with provenance.open("x") as stream:
        json.dump(record, stream, indent=2, sort_keys=True)
        stream.write("\n")
    outcomes = junit(xml)
    if process.returncode != (0 if all(value == "passed" for value in outcomes.values()) else 1):
        raise RuntimeError(f"{mode} exit code does not match four selected JUnit results; see {log}")
    return outcomes


def capture(result_dir: Path, wheel: Path) -> dict[str, Any]:
    if any(os.environ.get(key) for key in FORBIDDEN_INHERITED):
        raise ValueError("synthetic remote-kill diagnostic rejects inherited overrides")
    verify_sources()
    result_dir = result_dir.absolute()
    if result_dir.parent.resolve() != RESULT_PARENT.resolve() or not result_dir.name.startswith(
        "python-connect-remote-kill-synthetic-v1-"
    ):
        raise ValueError("result destination is outside the versioned diagnostic parent")
    if result_dir.exists() or result_dir.is_symlink():
        raise ValueError("diagnostic destination exists; use a new immutable path")
    if wheel.is_symlink() or not wheel.is_file() or wheel.name != WHEEL_NAME:
        raise ValueError("selected candidate wheel is missing or symlinked")
    if sha256(wheel) != WHEEL_SHA256:
        raise ValueError("selected wheel does not match the frozen ABI 125 build")
    inputs_before = {str(path.relative_to(ROOT)): sha256(path) for path in HARNESS_INPUTS}
    bind, host = fixture_address()
    result_dir.mkdir(parents=True, exist_ok=False)
    frozen_wheel = result_dir / WHEEL_NAME
    with frozen_wheel.open("xb") as output, wheel.open("rb") as source:
        shutil.copyfileobj(source, output)
    if sha256(frozen_wheel) != WHEEL_SHA256 or sha256(wheel) != WHEEL_SHA256:
        raise RuntimeError("candidate wheel changed during freeze")
    venv = result_dir / "venv"
    setup_command(["uv", "venv", "--python", "3.12", str(venv)])
    python = venv / "bin/python"
    tooling = json.loads(MANIFEST.read_text())["toolingVersions"]
    specs = [f"{name}=={version}" for name, version in sorted(tooling.items()) if name != "python"]
    install_log = setup_command(["uv", "pip", "install", "--python", str(python), *specs])
    install_log += setup_command([
        "uv", "pip", "install", "--python", str(python), "--no-deps", "--reinstall",
        str(frozen_wheel),
    ])
    with (result_dir / "install.log").open("x") as stream:
        stream.write(install_log)
    with (result_dir / "tooling-freeze.txt").open("x") as stream:
        stream.write(setup_command(["uv", "pip", "freeze", "--python", str(python)]))
    oracle = run_lane(python, result_dir, "oracle", bind, host)
    if any(outcome != "passed" for outcome in oracle.values()):
        raise RuntimeError(f"synthetic fixture did not validate original oracle assertions; see {result_dir / 'oracle.log'}")
    run_lane(python, result_dir, "candidate", bind, host)
    if inputs_before != {str(path.relative_to(ROOT)): sha256(path) for path in HARNESS_INPUTS}:
        raise RuntimeError("diagnostic harness sources changed during capture")
    attestation = verify_directory(result_dir)
    with (result_dir / "attestation.json").open("x") as stream:
        json.dump(attestation, stream, indent=2, sort_keys=True)
        stream.write("\n")
    return attestation


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    parser.add_argument("--wheel", type=Path, required=True)
    args = parser.parse_args()
    result = capture(args.result_dir, args.wheel)
    print(
        "fixture-adapted Python remote kill: "
        f"oracle {result['lanes']['oracle']['passed']}/4; "
        f"candidate {result['lanes']['candidate']['passed']}/4; "
        f"attestation={args.result_dir / 'attestation.json'}"
    )


if __name__ == "__main__":
    main()
