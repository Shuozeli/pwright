#!/usr/bin/env python3
"""Freeze independent oracle/candidate lanes for one fixture-adapted Python case."""

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

from verify_playwright_python_connect_close_events_synthetic_v1 import (
    CASE_BODY,
    CORE,
    HARNESS_INPUTS,
    MANIFEST,
    PLUGIN_DIR,
    RESULT_PARENT,
    ROOT,
    SERVER,
    SHIM,
    UPSTREAM,
    WHEEL_NAME,
    WHEEL_SHA256,
    sha256,
    verify_directory,
    verify_sources,
)

RUNTIME = ROOT / "packages/pwright-python/tests/official/runtime"
FORBIDDEN_INHERITED = (
    "CI",
    "NODE_OPTIONS",
    "NODE_PATH",
    "PYTHONHOME",
    "PYTHONPATH",
    "PYTHONUSERBASE",
    "PYTEST_ADDOPTS",
    "PYTEST_PLUGINS",
    "PWRIGHT_CDP_ENDPOINT",
    "PWRIGHT_CDP_WS_URL",
    "PWRIGHT_PYTHON_CLOSE_EVENTS_MODE",
    "PWRIGHT_PYTHON_CLOSE_EVENTS_RECEIPT",
    "PWRIGHT_PYTHON_CLOSE_EVENTS_SERVER",
)


def _command(command: list[str], *, timeout: int = 120) -> str:
    result = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
        timeout=timeout,
    )
    if result.returncode:
        raise RuntimeError(
            f"setup command failed ({result.returncode}): {command[:4]}\n{result.stderr[-3000:]}"
        )
    return result.stdout + result.stderr


def _fixture_address() -> tuple[str, str]:
    bind = _command(["tailscale", "ip", "-4"]).strip()
    status = json.loads(_command(["tailscale", "status", "--json"]))
    host = str(status["Self"]["DNSName"]).removesuffix(".")
    if ipaddress.ip_address(bind) not in ipaddress.ip_network("100.64.0.0/10"):
        raise ValueError("synthetic fixture did not receive this Mac's Tailscale IPv4")
    if not host.endswith(".ts.net") or "/" in host or ":" in host:
        raise ValueError("synthetic fixture requires the full Tailscale MagicDNS host")
    resolved = {item[4][0] for item in socket.getaddrinfo(host, 80, type=socket.SOCK_STREAM)}
    if bind not in resolved:
        raise ValueError("MagicDNS does not resolve to the Tailscale bind")
    return bind, host


def _run_pytest(
    python: Path,
    result_dir: Path,
    mode: str,
    bind: str,
    host: str,
) -> None:
    receipt = result_dir / f"{mode}.receipt.jsonl"
    xml = result_dir / f"{mode}.xml"
    log = result_dir / f"{mode}.log"
    provenance = result_dir / f"{mode}.provenance.json"
    env = {key: value for key, value in os.environ.items() if key not in FORBIDDEN_INHERITED}
    env.update(
        {
            "PYTHONNOUSERSITE": "1",
            "PYTHONPATH": os.pathsep.join(
                [str(SHIM), str(RUNTIME), str(PLUGIN_DIR)]
                if mode == "candidate"
                else [str(RUNTIME), str(PLUGIN_DIR)]
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
        }
    )
    command = [
        str(python),
        "-m",
        "pytest",
        "-q",
        "--tb=short",
        "--override-ini",
        "addopts=",
        "--import-mode=importlib",
        "-c",
        str(UPSTREAM / "pyproject.toml"),
        "-p",
        "pwright_official_server_runtime",
        "-p",
        "pwright_python_connect_close_events_synthetic_v1",
        "-p",
        "pwright_candidate_runtime" if mode == "candidate" else "pwright_official_oracle_runtime",
        "--browser=chromium",
        "--timeout=30",
        "--timeout-method=signal",
        f"--junitxml={xml}",
        str(UPSTREAM / CASE_BODY.split("::", 1)[0]) + "::" + CASE_BODY.split("::", 1)[1],
    ]
    process = subprocess.Popen(
        command,
        cwd=ROOT,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        start_new_session=True,
    )
    try:
        stdout, _ = process.communicate(timeout=120)
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
        "schema": 1,
        "mode": mode,
        "command": command,
        "exitCode": process.returncode,
        "wheelSha256": WHEEL_SHA256,
        "environment": {
            key: env[key]
            for key in (
                "PYTHONPATH",
                "PWRIGHT_OFFICIAL_LAUNCH_MODE",
                "PWRIGHT_OFFICIAL_TAILSCALE_IP",
                "PWRIGHT_OFFICIAL_TAILSCALE_HOST",
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
    if process.returncode:
        raise RuntimeError(f"{mode} pinned close-event case failed; see {log}")


def capture(result_dir: Path, wheel: Path) -> dict[str, Any]:
    if any(os.environ.get(key) for key in FORBIDDEN_INHERITED):
        raise ValueError("synthetic close-events diagnostic rejects inherited overrides")
    verify_sources()
    result_dir = result_dir.absolute()
    if result_dir.parent.resolve() != RESULT_PARENT.resolve() or not result_dir.name.startswith(
        "python-connect-close-events-synthetic-v1-"
    ):
        raise ValueError("result destination is outside the versioned diagnostic parent")
    if result_dir.exists() or result_dir.is_symlink():
        raise ValueError("diagnostic destination exists; use a new immutable path")
    if wheel.is_symlink() or not wheel.is_file() or wheel.name != WHEEL_NAME:
        raise ValueError("selected candidate wheel is missing or symlinked")
    if sha256(wheel) != WHEEL_SHA256:
        raise ValueError("selected wheel does not match frozen ABI 125 build")
    inputs_before = {str(path.relative_to(ROOT)): sha256(path) for path in HARNESS_INPUTS}
    bind, host = _fixture_address()
    result_dir.mkdir(parents=True, exist_ok=False)
    frozen_wheel = result_dir / WHEEL_NAME
    with frozen_wheel.open("xb") as output, wheel.open("rb") as source:
        shutil.copyfileobj(source, output)
    if sha256(frozen_wheel) != WHEEL_SHA256 or sha256(wheel) != WHEEL_SHA256:
        raise RuntimeError("candidate wheel changed during freeze")
    venv = result_dir / "venv"
    _command(["uv", "venv", "--python", "3.12", str(venv)])
    python = venv / "bin/python"
    tooling = json.loads(MANIFEST.read_text())["toolingVersions"]
    specs = [f"{name}=={version}" for name, version in sorted(tooling.items()) if name != "python"]
    install_log = _command(["uv", "pip", "install", "--python", str(python), *specs])
    install_log += _command(
        [
            "uv",
            "pip",
            "install",
            "--python",
            str(python),
            "--no-deps",
            "--reinstall",
            str(frozen_wheel),
        ]
    )
    with (result_dir / "install.log").open("x") as stream:
        stream.write(install_log)
    with (result_dir / "tooling-freeze.txt").open("x") as stream:
        stream.write(_command(["uv", "pip", "freeze", "--python", str(python)]))
    for mode in ("oracle", "candidate"):
        _run_pytest(python, result_dir, mode, bind, host)
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
        "fixture-adapted Python close events: "
        f"oracle {result['lanes']['oracle']['passed']}/1; "
        f"candidate {result['lanes']['candidate']['passed']}/1; "
        f"attestation={args.result_dir / 'attestation.json'}"
    )


if __name__ == "__main__":
    main()
