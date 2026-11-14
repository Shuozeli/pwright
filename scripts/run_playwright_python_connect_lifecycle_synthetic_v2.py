#!/usr/bin/env python3
"""Capture an immutable oracle-first pair for six unchanged Python state cases.

Only the upstream launch_server fixture's RemoteServer binding is synthetic.
This diagnostic never writes to the official conformance ledger.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import signal
import subprocess
from pathlib import Path

import run_playwright_python_connect_lifecycle_synthetic_v1 as v1_runner
from verify_playwright_python_connect_lifecycle_synthetic_v2 import (
    CORE,
    ENGINES,
    HARNESS_INPUTS,
    LEGACY_PLUGIN_DIR,
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
    harness_hashes,
    sha256,
    verify_directory,
    verify_sources,
)

FORBIDDEN_INHERITED = (
    *v1_runner.FORBIDDEN_INHERITED,
    "PWRIGHT_PYTHON_LIFECYCLE_ENGINE_FILTER",
    "PWRIGHT_PYTHON_LIFECYCLE_UPSTREAM",
    "PWRIGHT_PYTHON_LIFECYCLE_CORE_UPSTREAM",
    "PWRIGHT_OFFICIAL_TAILSCALE_IP",
    "PWRIGHT_OFFICIAL_TAILSCALE_HOST",
)


def run_pytest(
    python: Path,
    result_dir: Path,
    mode: str,
    engine: str,
    bind: str,
    host: str,
) -> None:
    prefix = f"{mode}-{engine}"
    receipt = result_dir / f"{prefix}.receipt.jsonl"
    xml = result_dir / f"{prefix}.xml"
    log = result_dir / f"{prefix}.log"
    provenance_path = result_dir / f"{prefix}.provenance.json"
    env = {key: value for key, value in os.environ.items() if key not in FORBIDDEN_INHERITED}
    env.update(
        {
            "PYTHONNOUSERSITE": "1",
            "PYTHONPATH": os.pathsep.join(
                [str(SHIM), str(RUNTIME), str(PLUGIN_DIR), str(LEGACY_PLUGIN_DIR)]
                if mode == "candidate"
                else [str(RUNTIME), str(PLUGIN_DIR), str(LEGACY_PLUGIN_DIR)]
            ),
            "PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned",
            "PWRIGHT_OFFICIAL_BROWSER_CHANNEL": "",
            "PWRIGHT_OFFICIAL_TAILSCALE_IP": bind,
            "PWRIGHT_OFFICIAL_TAILSCALE_HOST": host,
            "PWRIGHT_PYTHON_LIFECYCLE_MODE": mode,
            "PWRIGHT_PYTHON_LIFECYCLE_ENGINE_FILTER": engine,
            "PWRIGHT_PYTHON_LIFECYCLE_RECEIPT": str(receipt),
            "PWRIGHT_PYTHON_LIFECYCLE_SERVER": str(SERVER),
            "PWRIGHT_PYTHON_LIFECYCLE_UPSTREAM": str(UPSTREAM),
            "PWRIGHT_PYTHON_LIFECYCLE_CORE_UPSTREAM": str(CORE),
        }
    )
    command = expected_command(python, engine, xml, mode)
    process = subprocess.Popen(
        command,
        cwd=ROOT,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        start_new_session=True,
    )
    try:
        stdout, _ = process.communicate(timeout=130)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        stdout, _ = process.communicate()
        with log.open("x") as stream:
            stream.write(stdout)
        raise RuntimeError(f"{prefix} exceeded bounded timeout; see {log}") from None
    with log.open("x") as stream:
        stream.write(stdout)
    if process.returncode not in (0, 1) or not xml.is_file() or not receipt.is_file():
        raise RuntimeError(
            f"{prefix} produced incomplete evidence (exit {process.returncode}); see {log}"
        )
    provenance = {
        "schema": 2,
        "mode": mode,
        "engine": engine,
        "wheelSha256": WHEEL_SHA256,
        "command": command,
        "exitCode": process.returncode,
        "environment": {
            key: env[key]
            for key in (
                "PYTHONPATH",
                "PWRIGHT_OFFICIAL_LAUNCH_MODE",
                "PWRIGHT_OFFICIAL_TAILSCALE_IP",
                "PWRIGHT_OFFICIAL_TAILSCALE_HOST",
                "PWRIGHT_PYTHON_LIFECYCLE_MODE",
                "PWRIGHT_PYTHON_LIFECYCLE_ENGINE_FILTER",
                "PWRIGHT_PYTHON_LIFECYCLE_RECEIPT",
                "PWRIGHT_PYTHON_LIFECYCLE_SERVER",
                "PWRIGHT_PYTHON_LIFECYCLE_UPSTREAM",
                "PWRIGHT_PYTHON_LIFECYCLE_CORE_UPSTREAM",
            )
        },
        "xmlSha256": sha256(xml),
        "receiptSha256": sha256(receipt),
        "logSha256": sha256(log),
    }
    with provenance_path.open("x") as stream:
        json.dump(provenance, stream, indent=2, sort_keys=True)
        stream.write("\n")


def capture(result_dir: Path, wheel: Path) -> dict[str, object]:
    if any(os.environ.get(key) for key in FORBIDDEN_INHERITED):
        raise ValueError("synthetic lifecycle diagnostic rejects inherited overrides")
    verify_sources()
    result_dir = result_dir.absolute()
    if result_dir.parent.resolve() != RESULT_PARENT.resolve() or not result_dir.name.startswith(
        "python-connect-lifecycle-synthetic-v2-"
    ):
        raise ValueError("destination is outside the versioned diagnostic directory")
    if result_dir.exists() or result_dir.is_symlink():
        raise ValueError("diagnostic destination exists; use a new immutable path")
    if wheel.is_symlink() or not wheel.is_file() or wheel.name != WHEEL_NAME:
        raise ValueError("selected wheel is missing, symlinked, or unexpected")
    wheel_before = sha256(wheel)
    if wheel_before != WHEEL_SHA256:
        raise ValueError("selected wheel is not the pinned closed-target ABI125 artifact")
    inputs_before = harness_hashes()
    bind, host = v1_runner._fixture_address()
    result_dir.mkdir(parents=True, exist_ok=False)
    frozen = result_dir / WHEEL_NAME
    with frozen.open("xb") as output, wheel.open("rb") as source:
        shutil.copyfileobj(source, output)
    if sha256(frozen) != wheel_before or sha256(wheel) != wheel_before:
        raise RuntimeError("selected wheel changed during freeze")
    venv = result_dir / "venv"
    v1_runner._command(["uv", "venv", "--python", "3.12", str(venv)])
    python = venv / "bin/python"
    tooling = json.loads(MANIFEST.read_text())["toolingVersions"]
    tooling_specs = [
        f"{name}=={version}" for name, version in sorted(tooling.items()) if name != "python"
    ]
    install = v1_runner._command(
        ["uv", "pip", "install", "--python", str(python), *tooling_specs]
    )
    wheel_install = v1_runner._command(
        ["uv", "pip", "install", "--python", str(python), "--no-deps", "--reinstall", str(frozen)]
    )
    with (result_dir / "install.log").open("x") as stream:
        stream.write(install.stdout + install.stderr + wheel_install.stdout + wheel_install.stderr)
    freeze = v1_runner._command(["uv", "pip", "freeze", "--python", str(python)]).stdout
    with (result_dir / "tooling-freeze.txt").open("x") as stream:
        stream.write(freeze)
    for mode in ("oracle", "candidate"):
        for engine in ENGINES:
            run_pytest(python, result_dir, mode, engine, bind, host)
    if inputs_before != harness_hashes():
        raise RuntimeError("diagnostic harness sources changed during capture")
    if sha256(frozen) != wheel_before:
        raise RuntimeError("frozen candidate wheel changed during capture")
    attestation = verify_directory(result_dir)
    attestation["harnessInputs"] = inputs_before
    with (result_dir / "attestation.json").open("x") as stream:
        json.dump(attestation, stream, indent=2, sort_keys=True)
        stream.write("\n")
    return attestation


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    parser.add_argument("--wheel", type=Path, required=True)
    args = parser.parse_args()
    attestation = capture(args.result_dir, args.wheel)
    print(
        "fixture-adapted Python connect state: "
        f"oracle {attestation['lanes']['oracle']['passed']}/6; "
        f"candidate {attestation['lanes']['candidate']['passed']}/6; "
        f"attestation={args.result_dir / 'attestation.json'}"
    )


if __name__ == "__main__":
    main()
