#!/usr/bin/env python3
"""Capture oracle-first, fixture-adapted Core remote Page-event evidence."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
from pathlib import Path

from official_candidate_package_snapshot_v1 import create_snapshot, sha256
from run_playwright_official_conformance import _candidate_source_sha256
from verify_playwright_core_server_connect_page_events_synthetic_v2 import (
    ARTIFACT_SOURCE,
    INPUTS,
    MANIFEST,
    OFFICIAL_RUNNER,
    POSITIVE,
    PRELOAD,
    PROFILE,
    RESULT_PARENT,
    ROOT,
    SERVER,
    SOURCE_PACKAGE,
    SUITE,
    UPSTREAM,
    require,
    verify_directory,
    verify_lane,
    verify_upstream,
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
    "PWRIGHT_CORE_PAGE_EVENTS_SYNTHETIC_MODE",
    "PWRIGHT_CORE_PAGE_EVENTS_SYNTHETIC_RECEIPT",
    "PWRIGHT_CORE_PAGE_EVENTS_SYNTHETIC_SERVER",
    "PWRIGHT_CORE_PAGE_EVENTS_SYNTHETIC_PROBE_TOKEN",
)


def _run_official(result_dir: Path, mode: str, snapshot: Path) -> None:
    result = result_dir / f"{mode}.json"
    receipt = result_dir / f"{mode}.official.receipt.jsonl"
    log = result_dir / f"{mode}.log"
    env = os.environ.copy()
    env["NODE_OPTIONS"] = f"--require={PRELOAD}"
    env["PWRIGHT_CORE_PAGE_EVENTS_SYNTHETIC_MODE"] = mode
    env["PWRIGHT_CORE_PAGE_EVENTS_SYNTHETIC_RECEIPT"] = str(receipt)
    env["PWRIGHT_CORE_PAGE_EVENTS_SYNTHETIC_SERVER"] = str(SERVER)
    command = [
        "uv", "run", "--with", "pytest", str(OFFICIAL_RUNNER), mode,
        str(UPSTREAM), "--suite", SUITE, "--manifest", str(MANIFEST),
        "--grep", "run-server", "--workers", "1", "--result", str(result),
    ]
    if mode == "candidate":
        command.extend(["--candidate", str(snapshot), "--candidate-profile", PROFILE])
    completed = subprocess.run(
        command, cwd=ROOT, env=env, capture_output=True, text=True,
        check=False, timeout=180,
    )
    with log.open("x") as stream:
        stream.write(completed.stdout)
        stream.write(completed.stderr)
    require(result.is_file() and result.with_suffix(".provenance.json").is_file(),
            f"{mode} Core runner produced no JSON/provenance (exit {completed.returncode}); see {log}")
    require(completed.returncode == 0,
            f"{mode} original upstream assertions did not pass (exit {completed.returncode}); see {log}")


def _run_positive(result_dir: Path, mode: str, snapshot: Path) -> None:
    provenance = json.loads((result_dir / f"{mode}.provenance.json").read_text())
    env = os.environ.copy()
    env["PWRIGHT_CORE_PAGE_EVENTS_SYNTHETIC_MODE"] = mode
    env["PWRIGHT_CORE_PAGE_EVENTS_SYNTHETIC_RECEIPT"] = str(
        result_dir / f"{mode}.positive.receipt.jsonl"
    )
    env["PWRIGHT_CORE_PAGE_EVENTS_SYNTHETIC_SERVER"] = str(SERVER)
    env["PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM"] = str(UPSTREAM)
    env["PWRIGHT_OFFICIAL_SERVER_HOST"] = provenance["fixtureHost"]
    env["PWRIGHT_OFFICIAL_SERVER_IP"] = provenance["fixtureBind"]
    if mode == "candidate":
        env["PWRIGHT_NODE_CANDIDATE"] = str(snapshot)
        env["PWRIGHT_NATIVE_PROFILE"] = PROFILE
    completed = subprocess.run(
        ["node", str(POSITIVE)], cwd=ROOT, env=env, capture_output=True,
        text=True, check=False, timeout=45,
    )
    log = result_dir / f"{mode}.positive.log"
    with log.open("x") as stream:
        stream.write(completed.stderr)
        if completed.returncode != 0:
            stream.write(completed.stdout)
    require(completed.returncode == 0,
            f"{mode} positive Page-event probe failed (exit {completed.returncode}); see {log}")
    lines = completed.stdout.splitlines()
    require(len(lines) == 1, f"{mode} positive probe output was not one JSON object")
    observation = json.loads(lines[0])
    require(isinstance(observation, dict), f"{mode} positive probe output is not an object")
    output = result_dir / f"{mode}.positive.json"
    with output.open("x") as stream:
        json.dump(observation, stream, indent=2, sort_keys=True)
        stream.write("\n")


def capture(
    result_dir: Path, artifact_sha256: str, source_sha256: str
) -> dict[str, object]:
    if any(os.environ.get(key) for key in FORBIDDEN_OVERRIDES):
        raise ValueError("Page-events capture rejects inherited runtime overrides")
    if os.environ.get("CI"):
        raise ValueError("Page-events capture forbids CI-mode retries")
    require(bool(re.fullmatch(r"[0-9a-f]{64}", artifact_sha256)),
            "selected addon SHA256 must be explicit")
    require(bool(re.fullmatch(r"[0-9a-f]{64}", source_sha256)),
            "selected package source SHA256 must be explicit")
    verify_upstream()
    result_dir = result_dir.absolute()
    require(result_dir.parent.resolve() == RESULT_PARENT.resolve(),
            "result directory must be directly under official results")
    require(result_dir.name.startswith("core-connect-page-events-synthetic-v2-green-"),
            "result directory must carry the versioned positive profile prefix")
    require(not result_dir.exists() and not result_dir.is_symlink(),
            "capture destination already exists; choose a fresh path")
    require(ARTIFACT_SOURCE.is_file() and not ARTIFACT_SOURCE.is_symlink()
            and sha256(ARTIFACT_SOURCE) == artifact_sha256,
            "selected-profile addon is missing or changed from the proven build")
    source_digest = _candidate_source_sha256(SOURCE_PACKAGE)
    require(source_digest == source_sha256,
            "live candidate package source changed from the selected build")
    inputs_before = {str(path.relative_to(ROOT)): sha256(path) for path in INPUTS}
    result_dir.mkdir(parents=True, exist_ok=False)
    snapshot = result_dir / "candidate-package-snapshot"
    snapshot_manifest = result_dir / "candidate-package-snapshot.files.json"
    create_snapshot(SOURCE_PACKAGE, snapshot, snapshot_manifest,
                    profile=PROFILE, artifact_sha256=artifact_sha256)
    snapshot_data = json.loads(snapshot_manifest.read_text())
    require(snapshot_data["candidateSourceSha256"] == source_digest,
            "candidate source changed during snapshot creation")

    _run_official(result_dir, "oracle", snapshot)
    _run_positive(result_dir, "oracle", snapshot)
    verify_lane(result_dir, "oracle", snapshot_data)
    _run_official(result_dir, "candidate", snapshot)
    _run_positive(result_dir, "candidate", snapshot)
    require(inputs_before == {str(path.relative_to(ROOT)): sha256(path) for path in INPUTS}
            and sha256(ARTIFACT_SOURCE) == artifact_sha256
            and _candidate_source_sha256(SOURCE_PACKAGE) == source_digest,
            "harness, addon or live candidate source changed during paired capture")
    attestation = verify_directory(result_dir)
    with (result_dir / "attestation.json").open("x") as stream:
        json.dump(attestation, stream, indent=2, sort_keys=True)
        stream.write("\n")
    return attestation


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    parser.add_argument("--artifact-sha256", required=True)
    parser.add_argument("--source-sha256", required=True)
    args = parser.parse_args()
    attestation = capture(args.result_dir, args.artifact_sha256, args.source_sha256)
    print("fixture-adapted Core Page events: oracle 3/3 + positive; candidate 3/3 + positive")
    print(f"attestation={args.result_dir / 'attestation.json'}")
    print(f"selected addon={attestation['candidateArtifactSha256']}")


if __name__ == "__main__":
    main()
