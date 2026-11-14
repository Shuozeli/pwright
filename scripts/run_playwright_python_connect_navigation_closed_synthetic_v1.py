#!/usr/bin/env python3
"""Capture immutable oracle/candidate evidence for the pinned navigation case."""

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

from verify_playwright_python_connect_navigation_closed_synthetic_v1 import (
    CASE_BODY,
    CORE,
    ENGINES,
    MANIFEST,
    PLUGIN_DIR,
    RESULT_PARENT,
    ROOT,
    SERVER,
    SHIM,
    UPSTREAM,
    WHEEL_NAME,
    WHEEL_SHA256,
    harness_hashes,
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
    "PWRIGHT_PYTHON_NAV_CLOSED_MODE",
    "PWRIGHT_PYTHON_NAV_CLOSED_ENGINE",
    "PWRIGHT_PYTHON_NAV_CLOSED_RECEIPT",
    "PWRIGHT_PYTHON_NAV_CLOSED_SERVER",
)


def _command(command: list[str], *, timeout: int = 120) -> str:
    result = subprocess.run(
        command, cwd=ROOT, text=True, capture_output=True, check=False, timeout=timeout
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
        raise ValueError("diagnostic requires this Mac's Tailscale IPv4")
    if not host.endswith(".ts.net") or "/" in host or ":" in host:
        raise ValueError("diagnostic requires the full Tailscale MagicDNS host")
    resolved = {item[4][0] for item in socket.getaddrinfo(host, 80, type=socket.SOCK_STREAM)}
    if bind not in resolved:
        raise ValueError("MagicDNS does not resolve to the Tailscale bind")
    return bind, host


def _run_pytest(
    python: Path, result_dir: Path, mode: str, engine: str, bind: str, host: str
) -> None:
    prefix = f"{mode}-{engine}"
    receipt = result_dir / f"{prefix}.receipt.jsonl"
    xml = result_dir / f"{prefix}.xml"
    log = result_dir / f"{prefix}.log"
    provenance = result_dir / f"{prefix}.provenance.json"
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
            "PWRIGHT_PYTHON_NAV_CLOSED_MODE": mode,
            "PWRIGHT_PYTHON_NAV_CLOSED_ENGINE": engine,
            "PWRIGHT_PYTHON_NAV_CLOSED_RECEIPT": str(receipt),
            "PWRIGHT_PYTHON_NAV_CLOSED_SERVER": str(SERVER),
            "PWRIGHT_PYTHON_NAV_CLOSED_UPSTREAM": str(UPSTREAM),
            "PWRIGHT_PYTHON_NAV_CLOSED_CORE_UPSTREAM": str(CORE),
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
        "pwright_python_connect_navigation_closed_synthetic_v1",
        "-p",
        "pwright_candidate_runtime" if mode == "candidate" else "pwright_official_oracle_runtime",
        f"--browser={engine}",
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
        raise RuntimeError(f"{prefix} exceeded bounded timeout; see {log}") from None
    with log.open("x") as stream:
        stream.write(stdout)
    if process.returncode not in (0, 1) or not xml.is_file() or not receipt.is_file():
        raise RuntimeError(f"{prefix} produced incomplete evidence; see {log}")
    record: dict[str, object] = {
        "schema": 1,
        "mode": mode,
        "engine": engine,
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
                "PWRIGHT_PYTHON_NAV_CLOSED_MODE",
                "PWRIGHT_PYTHON_NAV_CLOSED_ENGINE",
                "PWRIGHT_PYTHON_NAV_CLOSED_RECEIPT",
                "PWRIGHT_PYTHON_NAV_CLOSED_SERVER",
                "PWRIGHT_PYTHON_NAV_CLOSED_UPSTREAM",
                "PWRIGHT_PYTHON_NAV_CLOSED_CORE_UPSTREAM",
            )
        },
        "fileSha256": {
            suffix: sha256(result_dir / f"{prefix}.{suffix}")
            for suffix in ("xml", "log", "receipt.jsonl")
        },
    }
    with provenance.open("x") as stream:
        json.dump(record, stream, indent=2, sort_keys=True)
        stream.write("\n")


def capture(result_dir: Path, wheel: Path) -> dict[str, object]:
    if any(os.environ.get(key) for key in FORBIDDEN_INHERITED):
        raise ValueError("diagnostic rejects inherited browser/pytest/Node overrides")
    verify_sources()
    result_dir = result_dir.absolute()
    if result_dir.parent.resolve() != RESULT_PARENT.resolve() or not result_dir.name.startswith(
        "python-connect-navigation-closed-synthetic-v1-"
    ):
        raise ValueError("result destination is outside the versioned diagnostic parent")
    if result_dir.exists() or result_dir.is_symlink():
        raise ValueError("diagnostic destination exists; use a new immutable path")
    if wheel.is_symlink() or not wheel.is_file() or wheel.name != WHEEL_NAME:
        raise ValueError("candidate wheel is missing or symlinked")
    if sha256(wheel) != WHEEL_SHA256:
        raise ValueError("candidate wheel differs from pinned ABI125 build")
    inputs_before = harness_hashes()
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
        ["uv", "pip", "install", "--python", str(python), "--no-deps", str(frozen_wheel)]
    )
    with (result_dir / "install.log").open("x") as stream:
        stream.write(install_log)
    with (result_dir / "tooling-freeze.txt").open("x") as stream:
        stream.write(_command(["uv", "pip", "freeze", "--python", str(python)]))
    for mode in ("oracle", "candidate"):
        for engine in ENGINES:
            _run_pytest(python, result_dir, mode, engine, bind, host)
    if inputs_before != harness_hashes() or sha256(frozen_wheel) != WHEEL_SHA256:
        raise RuntimeError("harness or wheel changed during capture")
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
    print(result["result"])
    print(args.result_dir / "attestation.json")


if __name__ == "__main__":
    main()
