#!/usr/bin/env python3
"""Capture a new fixed-wheel remote-kill diagnostic without changing RED history."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import signal
import subprocess
from pathlib import Path
from typing import Any

import run_playwright_python_connect_remote_kill_synthetic_v1 as prior_runner
import verify_playwright_python_connect_remote_kill_synthetic_v1 as prior
import verify_playwright_python_connect_remote_kill_synthetic_v2 as verify


def run_lane(
    python: Path, result_dir: Path, label: str, bind: str, host: str,
) -> dict[str, str]:
    mode = "oracle" if label == "oracle" else "candidate"
    receipt = result_dir / f"{label}.receipt.jsonl"
    xml = result_dir / f"{label}.xml"
    log = result_dir / f"{label}.log"
    provenance = result_dir / f"{label}.provenance.json"
    env = {key: value for key, value in os.environ.items()
           if key not in prior_runner.FORBIDDEN_INHERITED}
    env.update({
        "PYTHONNOUSERSITE": "1",
        "PYTHONPATH": os.pathsep.join(
            [str(prior.SHIM), str(prior.RUNTIME), str(prior.PLUGIN_DIR), str(prior.LEGACY_DIR)]
            if mode == "candidate" else
            [str(prior.RUNTIME), str(prior.PLUGIN_DIR), str(prior.LEGACY_DIR)]
        ),
        "PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned",
        "PWRIGHT_OFFICIAL_BROWSER_CHANNEL": "",
        "PWRIGHT_OFFICIAL_TAILSCALE_IP": bind,
        "PWRIGHT_OFFICIAL_TAILSCALE_HOST": host,
        "PWRIGHT_PYTHON_CLOSE_EVENTS_MODE": mode,
        "PWRIGHT_PYTHON_CLOSE_EVENTS_ENGINE_FILTER": "chromium",
        "PWRIGHT_PYTHON_CLOSE_EVENTS_ENGINE": "chromium",
        "PWRIGHT_PYTHON_CLOSE_EVENTS_RECEIPT": str(receipt),
        "PWRIGHT_PYTHON_CLOSE_EVENTS_SERVER": str(prior.SERVER),
        "PWRIGHT_PYTHON_CLOSE_EVENTS_UPSTREAM": str(prior.UPSTREAM),
        "PWRIGHT_PYTHON_CLOSE_EVENTS_CORE_UPSTREAM": str(prior.CORE),
    })
    command = prior.expected_command(python, xml, mode)
    process = subprocess.Popen(
        command, cwd=verify.ROOT, env=env, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, text=True, start_new_session=True,
    )
    try:
        stdout, _ = process.communicate(timeout=240)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        stdout, _ = process.communicate()
        with log.open("x") as stream:
            stream.write(stdout)
        raise RuntimeError(f"{label} diagnostic exceeded bounded timeout; see {log}") from None
    with log.open("x") as stream:
        stream.write(stdout)
    if not xml.is_file() or not receipt.is_file():
        raise RuntimeError(f"{label} diagnostic produced incomplete evidence; see {log}")
    hashes = {suffix: prior.sha256(result_dir / f"{label}.{suffix}")
              for suffix in ("xml", "log", "receipt.jsonl")}
    record = {
        "schema": 1, "mode": mode, "laneLabel": label,
        "command": command, "exitCode": process.returncode,
        "wheelSha256": verify.WHEEL_SHA256,
        "sourceWheelPath": str(verify.BUILD_WHEEL),
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
        "fileSha256": hashes,
    }
    with provenance.open("x") as stream:
        json.dump(record, stream, indent=2, sort_keys=True)
        stream.write("\n")
    outcomes = prior.junit(xml)
    if process.returncode != (0 if all(status == "passed" for status in outcomes.values()) else 1):
        raise RuntimeError(f"{label} exit code differs from original JUnit cases; see {log}")
    return outcomes


def capture(result_dir: Path, wheel: Path) -> dict[str, Any]:
    if any(os.environ.get(key) for key in prior_runner.FORBIDDEN_INHERITED):
        raise ValueError("fixed-wheel remote-kill diagnostic rejects inherited overrides")
    verify.verify_sources()
    result_dir = result_dir.absolute()
    if result_dir.parent.resolve() != verify.RESULT_PARENT.resolve() or not result_dir.name.startswith(
        verify.RESULT_PREFIX
    ):
        raise ValueError("result destination is outside the new versioned diagnostic parent")
    if result_dir.exists() or result_dir.is_symlink():
        raise ValueError("diagnostic destination exists; never overwrite immutable evidence")
    if wheel.resolve() != verify.BUILD_WHEEL.resolve() or wheel.name != verify.WHEEL_NAME:
        raise ValueError("not the selected independently built fixed wheel")
    if wheel.is_symlink() or not wheel.is_file() or prior.sha256(wheel) != verify.WHEEL_SHA256:
        raise ValueError("selected fixed wheel changed")
    inputs_before = verify.harness_hashes()
    bind, host = prior_runner.fixture_address()
    result_dir.mkdir(parents=True, exist_ok=False)
    frozen_wheel = result_dir / verify.WHEEL_NAME
    with frozen_wheel.open("xb") as output, wheel.open("rb") as source:
        shutil.copyfileobj(source, output)
    if prior.sha256(frozen_wheel) != verify.WHEEL_SHA256:
        raise RuntimeError("fixed wheel changed during capture")
    venv = result_dir / "venv"
    prior_runner.setup_command(["uv", "venv", "--python", "3.12", str(venv)])
    python = venv / "bin/python"
    tooling = prior.TOOLING
    specs = [f"{name}=={version}" for name, version in sorted(tooling.items())
             if name != "python"]
    install_log = prior_runner.setup_command([
        "uv", "pip", "install", "--python", str(python), *specs,
    ])
    install_log += prior_runner.setup_command([
        "uv", "pip", "install", "--python", str(python), "--no-deps",
        "--reinstall", str(frozen_wheel),
    ])
    with (result_dir / "install.log").open("x") as stream:
        stream.write(install_log)
    with (result_dir / "tooling-freeze.txt").open("x") as stream:
        stream.write(prior_runner.setup_command([
            "uv", "pip", "freeze", "--python", str(python),
        ]))
    oracle = run_lane(python, result_dir, "oracle", bind, host)
    if any(status != "passed" for status in oracle.values()):
        raise RuntimeError(f"synthetic oracle did not pass original assertions; see {result_dir / 'oracle.log'}")
    for index in range(1, verify.CANDIDATE_REPEATS + 1):
        run_lane(python, result_dir, f"candidate-{index}", bind, host)
    if inputs_before != verify.harness_hashes():
        raise RuntimeError("diagnostic harness changed during capture")
    attestation = verify.verify_directory(result_dir)
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
    scores = [result["lanes"][label]["passed"] for label in verify.lane_labels()]
    print(f"fixture-adapted fixed wheel: oracle {scores[0]}/4; "
          f"candidate repeats {scores[1:]}/4; "
          f"attestation={args.result_dir / 'attestation.json'}")


if __name__ == "__main__":
    main()
