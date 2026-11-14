#!/usr/bin/env python3
"""Capture immutable oracle-first Core remote-waiter RED evidence.

The selected upstream assertions are unchanged; only run-server is synthetic.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
from pathlib import Path

from official_candidate_package_snapshot_v1 import create_snapshot, sha256
from run_playwright_official_conformance import _candidate_source_sha256
from verify_playwright_core_server_connect_waiters_synthetic_v1 import (
    ARTIFACT_SHA256,
    ARTIFACT_SOURCE,
    BASE_ARTIFACT_SHA256,
    MANIFEST,
    PRELOAD,
    PROFILE,
    ROOT,
    SERVER,
    SOURCE_PACKAGE,
    SOURCE_PACKAGE_SHA256,
    SUITE,
    UPSTREAM,
    verify_directory,
    verify_lane,
    verify_upstream,
)

RESULT_PARENT = ROOT / ".work/official-conformance/results"
OFFICIAL_RUNNER = ROOT / "scripts/run_playwright_official_conformance.py"
INPUTS = (
    MANIFEST,
    PRELOAD,
    SERVER,
    ROOT / "tests/official/playwright-core-browser-type-owner-v2-preload.cjs",
    ROOT / "tests/official/playwright-core-server-runtime.cjs",
    ROOT / "tests/official/playwright-core-candidate-preload.cjs",
    OFFICIAL_RUNNER,
    ROOT / "scripts/official_candidate_package_snapshot_v1.py",
    ROOT / "scripts/verify_playwright_core_server_connect_waiters_synthetic_v1.py",
    Path(__file__).resolve(),
)
FORBIDDEN_OVERRIDES = (
    "NODE_OPTIONS",
    "PWTEST_MODE",
    "PWPAGE_IMPL",
    "PWTEST_USE_BIDI_EXPECTATIONS",
    "PWRIGHT_CANDIDATE_CDP_ENDPOINT",
    "PWRIGHT_ORACLE_CDP_ENDPOINT",
    "PWRIGHT_CDP_ENDPOINT",
    "PWRIGHT_CDP_WS_URL",
    "PWRIGHT_NATIVE_PROFILE",
    "PWRIGHT_NODE_CANDIDATE",
    "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM",
    "PWRIGHT_OFFICIAL_SERVER_IP",
    "PWRIGHT_OFFICIAL_SERVER_HOST",
    "PWRIGHT_CORE_WAITERS_SYNTHETIC_MODE",
    "PWRIGHT_CORE_WAITERS_SYNTHETIC_RECEIPT",
    "PWRIGHT_CORE_WAITERS_SYNTHETIC_SERVER",
)


def _run_lane(result_dir: Path, mode: str, snapshot: Path) -> None:
    result = result_dir / f"{mode}.json"
    receipt = result_dir / f"{mode}.receipt.jsonl"
    log = result_dir / f"{mode}.log"
    env = os.environ.copy()
    env["NODE_OPTIONS"] = f"--require={PRELOAD}"
    env["PWRIGHT_CORE_WAITERS_SYNTHETIC_MODE"] = mode
    env["PWRIGHT_CORE_WAITERS_SYNTHETIC_RECEIPT"] = str(receipt)
    env["PWRIGHT_CORE_WAITERS_SYNTHETIC_SERVER"] = str(SERVER)
    command = [
        "uv",
        "run",
        "--with",
        "pytest",
        str(OFFICIAL_RUNNER),
        mode,
        str(UPSTREAM),
        "--suite",
        SUITE,
        "--manifest",
        str(MANIFEST),
        "--grep",
        "run-server",
        "--workers",
        "1",
        "--result",
        str(result),
    ]
    if mode == "candidate":
        command.extend(["--candidate", str(snapshot), "--candidate-profile", PROFILE])
    completed = subprocess.run(
        command, cwd=ROOT, env=env, capture_output=True, text=True, check=False
    )
    with log.open("x") as stream:
        stream.write(completed.stdout)
        stream.write(completed.stderr)
    if not result.is_file() or not result.with_suffix(".provenance.json").is_file():
        raise RuntimeError(
            f"{mode} runner produced no JSON/provenance (exit {completed.returncode}); see {log}"
        )
    if completed.returncode != (0 if mode == "oracle" else 1):
        raise RuntimeError(f"{mode} runner exited unexpectedly: {completed.returncode}")


def capture(result_dir: Path) -> dict[str, object]:
    if any(os.environ.get(key) for key in FORBIDDEN_OVERRIDES):
        raise ValueError(
            "synthetic waiters diagnostic rejects inherited runtime overrides"
        )
    if os.environ.get("CI"):
        raise ValueError("synthetic waiters diagnostic forbids CI-mode retries")
    verify_upstream()
    result_dir = result_dir.absolute()
    if result_dir.parent.resolve() != RESULT_PARENT.resolve():
        raise ValueError("result directory must be directly under official results")
    if not result_dir.name.startswith("core-connect-waiters-synthetic-v1-red-"):
        raise ValueError(
            "result directory must carry the versioned RED diagnostic prefix"
        )
    if result_dir.exists() or result_dir.is_symlink():
        raise ValueError(
            "diagnostic destination already exists; choose a fresh versioned path"
        )
    artifact = SOURCE_PACKAGE / "native" / PROFILE / "pwright.node"
    if not artifact.is_file() or artifact.is_symlink():
        raise ValueError("selected-profile release addon is missing")
    if sha256(artifact) != BASE_ARTIFACT_SHA256:
        raise ValueError("base native profile artifact changed")
    if not ARTIFACT_SOURCE.is_file() or ARTIFACT_SOURCE.is_symlink():
        raise ValueError("pinned context-close-capable addon is missing")
    if sha256(ARTIFACT_SOURCE) != ARTIFACT_SHA256:
        raise ValueError("pinned context-close-capable addon changed")
    if _candidate_source_sha256(SOURCE_PACKAGE) != SOURCE_PACKAGE_SHA256:
        raise ValueError(
            "candidate package source changed from pinned diagnostic input"
        )
    inputs_before = {
        str(path.relative_to(ROOT)): sha256(path) for path in (*INPUTS, ARTIFACT_SOURCE)
    }
    result_dir.mkdir(parents=True, exist_ok=False)
    overlay = result_dir / "candidate-source-overlay"
    overlay.mkdir(exist_ok=False)
    shutil.copyfile(SOURCE_PACKAGE / "package.json", overlay / "package.json")
    shutil.copytree(SOURCE_PACKAGE / "src", overlay / "src", symlinks=False)
    overlay_artifact = overlay / "native" / PROFILE / "pwright.node"
    overlay_artifact.parent.mkdir(parents=True, exist_ok=False)
    shutil.copyfile(ARTIFACT_SOURCE, overlay_artifact)
    if (
        _candidate_source_sha256(overlay) != SOURCE_PACKAGE_SHA256
        or sha256(overlay_artifact) != ARTIFACT_SHA256
    ):
        raise ValueError("candidate source overlay copy changed")
    snapshot = result_dir / "candidate-package-snapshot"
    snapshot_manifest = result_dir / "candidate-package-snapshot.files.json"
    create_snapshot(
        overlay,
        snapshot,
        snapshot_manifest,
        profile=PROFILE,
        artifact_sha256=ARTIFACT_SHA256,
    )
    snapshot_data = json.loads(snapshot_manifest.read_text())

    _run_lane(result_dir, "oracle", snapshot)
    verify_lane(result_dir, "oracle", snapshot_data)
    _run_lane(result_dir, "candidate", snapshot)
    if inputs_before != {
        str(path.relative_to(ROOT)): sha256(path) for path in (*INPUTS, ARTIFACT_SOURCE)
    }:
        raise ValueError("diagnostic harness source changed during paired capture")
    attestation = verify_directory(result_dir)
    attestation["harnessInputs"] = inputs_before
    attestation_path = result_dir / "attestation.json"
    with attestation_path.open("x") as stream:
        json.dump(attestation, stream, indent=2, sort_keys=True)
        stream.write("\n")
    return attestation


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    attestation = capture(args.result_dir)
    print("fixture-adapted diagnostic: oracle 3/3; candidate 0/3")
    print(f"attestation={args.result_dir / 'attestation.json'}")
    print(f"candidate addon={attestation['candidateArtifactSha256']}")


if __name__ == "__main__":
    main()
