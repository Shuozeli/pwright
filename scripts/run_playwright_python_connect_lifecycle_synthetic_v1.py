#!/usr/bin/env python3
"""Capture a browser-free official-derived Python connect lifecycle pair.

Four upstream test bodies are unchanged and expanded for all three engines.
Only their launch_server fixture's RemoteServer binding is synthetic. This
diagnostic never writes to the official conformance ledger.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import signal
import subprocess
from pathlib import Path

from verify_playwright_python_connect_lifecycle_synthetic_v1 import (
    CORE,
    ENGINES,
    HARNESS_INPUTS,
    MANIFEST,
    PLUGIN_DIR,
    RESULT_PARENT,
    ROOT,
    SERVER,
    UPSTREAM,
    verify_directory,
    verify_sources,
)

RUNTIME = ROOT / "packages/pwright-python/tests/official/runtime"
SHIM = ROOT / "packages/pwright-python/tests/official/shim"
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
    "PWRIGHT_OFFICIAL_SERVER_PREFIX",
    "PWRIGHT_PYTHON_LIFECYCLE_MODE",
    "PWRIGHT_PYTHON_LIFECYCLE_RECEIPT",
    "PWRIGHT_PYTHON_LIFECYCLE_SERVER",
)
TEST_BODIES = (
    "tests/async/test_browsertype_connect.py::test_browser_type_disconnected_event_should_have_browser_as_argument",
    "tests/async/test_browsertype_connect.py::test_browser_type_connect_set_browser_connected_state",
    "tests/sync/test_browsertype_connect.py::test_browser_type_disconnected_event_should_have_browser_as_argument",
    "tests/sync/test_browsertype_connect.py::test_browser_type_connect_set_browser_connected_state",
)


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _command(
    command: list[str], *, env: dict[str, str] | None = None, timeout: int = 120
) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        env=env,
        text=True,
        capture_output=True,
        check=False,
        timeout=timeout,
    )
    if completed.returncode:
        raise RuntimeError(
            f"setup command failed ({completed.returncode}): {command[:4]}\n"
            f"{completed.stderr[-3000:]}"
        )
    return completed


def _fixture_address() -> tuple[str, str]:
    ip = _command(["tailscale", "ip", "-4"]).stdout.strip()
    host = json.loads(_command(["tailscale", "status", "--json"]).stdout)["Self"]["DNSName"]
    host = host.removesuffix(".")
    if not ip.startswith("100.") or not host.endswith(".ts.net"):
        raise ValueError("synthetic fixture requires this Mac's full Tailscale address")
    return ip, host


def _run_pytest(
    python: Path,
    result_dir: Path,
    mode: str,
    engine: str,
    bind: str,
    host: str,
) -> dict[str, object]:
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
            "PWRIGHT_PYTHON_LIFECYCLE_MODE": mode,
            "PWRIGHT_PYTHON_LIFECYCLE_ENGINE_FILTER": engine,
            "PWRIGHT_PYTHON_LIFECYCLE_RECEIPT": str(receipt),
            "PWRIGHT_PYTHON_LIFECYCLE_SERVER": str(SERVER),
            "PWRIGHT_PYTHON_LIFECYCLE_UPSTREAM": str(UPSTREAM),
            "PWRIGHT_PYTHON_LIFECYCLE_CORE_UPSTREAM": str(CORE),
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
        "pwright_python_connect_lifecycle_synthetic_v1",
    ]
    if mode == "candidate":
        command.extend(["-p", "pwright_candidate_runtime"])
    else:
        command.extend(["-p", "pwright_official_oracle_runtime"])
    command.extend(
        [
            f"--browser={engine}",
            "--timeout=30",
            "--timeout-method=signal",
            f"--junitxml={xml}",
            *(
                str(UPSTREAM / body.split("::", maxsplit=1)[0])
                + "::"
                + body.split("::", maxsplit=1)[1]
                for body in TEST_BODIES
            ),
        ]
    )
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
        raise RuntimeError(f"{prefix} pytest exceeded bounded timeout; see {log}") from None
    with log.open("x") as stream:
        stream.write(stdout)
    if process.returncode not in (0, 1) or not xml.is_file() or not receipt.is_file():
        raise RuntimeError(
            f"{prefix} produced incomplete diagnostic (exit {process.returncode}); see {log}"
        )
    record: dict[str, object] = {
        "schema": 1,
        "mode": mode,
        "engine": engine,
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
        "xmlSha256": _sha256(xml),
        "receiptSha256": _sha256(receipt),
        "logSha256": _sha256(log),
    }
    with provenance.open("x") as stream:
        json.dump(record, stream, indent=2, sort_keys=True)
        stream.write("\n")
    return record


def capture(result_dir: Path, wheel: Path) -> dict[str, object]:
    if any(os.environ.get(key) for key in FORBIDDEN_INHERITED):
        raise ValueError("synthetic lifecycle diagnostic rejects inherited overrides")
    verify_sources()
    result_dir = result_dir.absolute()
    if result_dir.parent.resolve() != RESULT_PARENT.resolve() or not result_dir.name.startswith(
        "python-connect-lifecycle-synthetic-v1-"
    ):
        raise ValueError("result destination is outside the versioned diagnostic directory")
    if result_dir.exists() or result_dir.is_symlink():
        raise ValueError("diagnostic destination exists; use a new immutable path")
    if wheel.is_symlink() or not wheel.is_file() or wheel.suffix != ".whl":
        raise ValueError("selected candidate wheel is missing or symlinked")
    inputs_before = {str(path.relative_to(ROOT)): _sha256(path) for path in HARNESS_INPUTS}
    wheel_before = _sha256(wheel)
    bind, host = _fixture_address()
    result_dir.mkdir(parents=True, exist_ok=False)
    frozen_wheel = result_dir / wheel.name
    with frozen_wheel.open("xb") as output, wheel.open("rb") as source:
        shutil.copyfileobj(source, output)
    if _sha256(frozen_wheel) != wheel_before or _sha256(wheel) != wheel_before:
        raise RuntimeError("selected wheel changed during freeze")
    venv = result_dir / "venv"
    _command(["uv", "venv", "--python", "3.12", str(venv)])
    python = venv / "bin/python"
    tooling = json.loads(MANIFEST.read_text())["toolingVersions"]
    tooling_specs = [
        f"{name}=={version}" for name, version in sorted(tooling.items()) if name != "python"
    ]
    install_output = _command(["uv", "pip", "install", "--python", str(python), *tooling_specs])
    wheel_output = _command(
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
        stream.write(install_output.stdout + install_output.stderr)
        stream.write(wheel_output.stdout + wheel_output.stderr)
    freeze = _command(["uv", "pip", "freeze", "--python", str(python)]).stdout
    with (result_dir / "tooling-freeze.txt").open("x") as stream:
        stream.write(freeze)
    for mode in ("oracle", "candidate"):
        for engine in ENGINES:
            _run_pytest(python, result_dir, mode, engine, bind, host)
    if inputs_before != {str(path.relative_to(ROOT)): _sha256(path) for path in HARNESS_INPUTS}:
        raise RuntimeError("diagnostic harness sources changed during capture")
    if _sha256(frozen_wheel) != wheel_before:
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
        "fixture-adapted Python connect lifecycle: "
        f"oracle {attestation['lanes']['oracle']['passed']}/12; "
        f"candidate {attestation['lanes']['candidate']['passed']}/12; "
        f"attestation={args.result_dir / 'attestation.json'}"
    )


if __name__ == "__main__":
    main()
