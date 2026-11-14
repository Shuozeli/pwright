#!/usr/bin/env python3
"""Capture a sealed post-fix paired rerun of three official Core CDP errors."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
from pathlib import Path
from typing import Any

import run_playwright_core_cdp_error_slice_strict_v1 as base
import run_playwright_core_cdp_error_slice_strict_v2 as worker_aware
from official_candidate_package_snapshot_v1 import create_snapshot, sha256, verify_snapshot

ROOT = base.ROOT
MANIFEST = ROOT / "tests/official/playwright-core-cdp-error-slice-strict-v3-manifest.json"
SUITE = "library-chromium-cdp-error-slice-strict-v3"
PROFILE = "official-conformance-page-cdp-errors"
ARTIFACT_SHA256 = "0d7dcb931c42d5e912b594ec20fd12e202cc76590c7cab93187fe6829388f5c6"
SCOPE = base.SCOPE + "-post-fix-paired-v3"
RESULT_PARENT = ROOT / ".work/official-conformance/results/core-cdp-error-slice-strict-v3"
ORACLE_SIDECAR = RESULT_PARENT / "oracle-v1.strict.json"
SOURCE_PACKAGE = ROOT / "packages/pwright"
SNAPSHOT_PACKAGE = RESULT_PARENT / "candidate-package-snapshot-v1"
SNAPSHOT_MANIFEST = RESULT_PARENT / "candidate-package-snapshot-v1.files.json"
SELECTED_LINES = base.SELECTED_LINES
INPUT_FILES = (
    MANIFEST,
    base.PRELOAD,
    base.COMMON_PRELOAD,
    Path(base.__file__).resolve(),
    Path(worker_aware.__file__).resolve(),
    ROOT / "scripts/official_candidate_package_snapshot_v1.py",
    Path(__file__).resolve(),
    ROOT / "scripts/verify_playwright_core_cdp_error_slice_strict_v3.py",
)


def check_manifest() -> None:
    base.check_source()
    assert json.loads(MANIFEST.read_text()) == {
        "schema": 1,
        "repository": "https://github.com/microsoft/playwright.git",
        "tag": "v1.58.0",
        "commit": base.COMMIT,
        "suites": [
            {
                "id": SUITE,
                "role": "browser-independent-official-subset",
                "runner": "packages/playwright/cli.js",
                "config": "tests/library/playwright.config.ts",
                "args": [f"{base.SOURCE}:{line}" for line in SELECTED_LINES],
                "candidateAdapter": "test-mode-playwright-core",
                "project": "chromium-library",
                "requiresBrowser": False,
                "expectedTestsPerProject": 3,
                "expectedFiles": 1,
            }
        ],
    }


def check_pin() -> None:
    if re.fullmatch(r"[0-9a-f]{64}", ARTIFACT_SHA256) is None:
        raise ValueError("strict v3 capture awaits a pinned final candidate artifact SHA256")


def check_artifact(path: Path | None = None) -> Path:
    check_pin()
    artifact = path or ROOT / "packages/pwright/native" / PROFILE / "pwright.node"
    if not artifact.is_file() or artifact.is_symlink():
        raise ValueError("strict v3 candidate artifact is missing or not a regular file")
    if base.sha256(artifact) != ARTIFACT_SHA256:
        raise ValueError("strict v3 candidate artifact SHA256 changed")
    return artifact


def _check_environment() -> None:
    forbidden = (
        "NODE_OPTIONS",
        "PWTEST_MODE",
        "PWPAGE_IMPL",
        "PWTEST_USE_BIDI_EXPECTATIONS",
        "PWRIGHT_CANDIDATE_CDP_ENDPOINT",
        "PWRIGHT_ORACLE_CDP_ENDPOINT",
        "PWRIGHT_NATIVE_PROFILE",
        "PWRIGHT_NODE_CANDIDATE",
        "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM",
        "PWRIGHT_OFFICIAL_SERVER_HOST",
        "PWRIGHT_CORE_CDP_ERROR_SLICE_GUARD_RECEIPT",
    )
    if any(os.environ.get(key) for key in forbidden):
        raise ValueError("strict v3 CDP error slice rejects inherited runtime overrides")


def run(mode: str, result: Path) -> dict[str, Any]:
    _check_environment()
    check_manifest()
    check_pin()
    if mode == "candidate":
        check_artifact()
    result = result.absolute()
    if result != RESULT_PARENT / f"{mode}-v1.json":
        raise ValueError("strict v3 CDP error slice requires its exact mode-specific result path")
    RESULT_PARENT.mkdir(parents=True, exist_ok=True)
    outputs = [
        result,
        result.with_suffix(".provenance.json"),
        result.with_suffix(".strict.json"),
        result.with_suffix(".guard-receipt.jsonl"),
        result.with_suffix(".log"),
    ]
    if any(path.exists() or path.is_symlink() for path in outputs):
        raise ValueError("strict v3 CDP error slice result destination is not empty")
    oracle_sha256 = None
    snapshot_source_sha256 = None
    snapshot_manifest_sha256 = None
    if mode == "candidate":
        from verify_playwright_core_cdp_error_slice_strict_v3 import verify_strict

        oracle = verify_strict(ORACLE_SIDECAR)
        assert oracle["mode"] == "oracle"
        assert all(item["status"] == "expected" for item in oracle["selected"])
        oracle_sha256 = base.sha256(ORACLE_SIDECAR)
        snapshot = create_snapshot(
            SOURCE_PACKAGE,
            SNAPSHOT_PACKAGE,
            SNAPSHOT_MANIFEST,
            profile=PROFILE,
            artifact_sha256=ARTIFACT_SHA256,
        )
        snapshot_source_sha256 = snapshot["candidateSourceSha256"]
        snapshot_manifest_sha256 = sha256(SNAPSHOT_MANIFEST)
    receipt = result.with_suffix(".guard-receipt.jsonl")
    receipt.open("x").close()
    inputs = {str(path.relative_to(ROOT)): base.sha256(path) for path in INPUT_FILES}
    env = os.environ.copy()
    env["NODE_OPTIONS"] = f"--require={base.PRELOAD}"
    env["PWRIGHT_CORE_CDP_ERROR_SLICE_GUARD_RECEIPT"] = str(receipt)
    command = [
        "uv",
        "run",
        "--with",
        "pytest",
        str(ROOT / "scripts/run_playwright_official_conformance.py"),
        mode,
        str(base.UPSTREAM),
        "--suite",
        SUITE,
        "--manifest",
        str(MANIFEST),
        "--workers",
        "1",
        "--result",
        str(result),
    ]
    if mode == "candidate":
        command += [
            "--candidate",
            str(SNAPSHOT_PACKAGE),
            "--candidate-profile",
            PROFILE,
        ]
    completed = subprocess.run(
        command, check=False, cwd=ROOT, env=env, capture_output=True, text=True
    )
    log = completed.stdout + completed.stderr
    log_path = result.with_suffix(".log")
    log_path.write_text(log)
    assert all(base.sha256(ROOT / name) == digest for name, digest in inputs.items())
    check_manifest()
    if mode == "candidate":
        check_artifact()
        snapshot = verify_snapshot(
            SNAPSHOT_PACKAGE,
            SNAPSHOT_MANIFEST,
            source_package=SOURCE_PACKAGE,
            profile=PROFILE,
            artifact_sha256=ARTIFACT_SHA256,
        )
        assert snapshot["candidateSourceSha256"] == snapshot_source_sha256
        assert sha256(SNAPSHOT_MANIFEST) == snapshot_manifest_sha256
    provenance = result.with_suffix(".provenance.json")
    if not result.is_file() or not provenance.is_file():
        raise RuntimeError(
            "official runner produced no result/provenance "
            f"(exit {completed.returncode}): {log[-3000:]}"
        )
    selected = worker_aware.selected_tests(json.loads(result.read_text()))
    assert completed.returncode == (
        0 if all(item["status"] == "expected" for item in selected) else 1
    )
    if mode == "oracle":
        assert all(item["status"] == "expected" for item in selected)
    if mode == "candidate":
        assert "[pwright-candidate]" in log
    else:
        assert "[pwright-candidate]" not in log
    provenance_data = json.loads(provenance.read_text())
    if mode == "candidate":
        assert provenance_data["candidate"] == str(SNAPSHOT_PACKAGE)
        assert provenance_data["candidateSourceSha256"] == snapshot_source_sha256
        assert provenance_data["candidateArtifactSha256"] == ARTIFACT_SHA256
    worker_aware.checked_receipts(receipt, selected, provenance_data["fixtureHost"])
    strict = {
        "schema": 4,
        "scope": SCOPE,
        "mode": mode,
        "upstreamCommit": base.COMMIT,
        "upstreamSourceSha256": base.SOURCE_SHA256,
        "serverFixtureSha256": base.SERVER_FIXTURE_SHA256,
        "commonPreloadSha256": base.COMMON_PRELOAD_SHA256,
        "inputs": inputs,
        "candidateProfile": PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": ARTIFACT_SHA256,
        "pairedOracleStrictSha256": oracle_sha256,
        "candidateSnapshot": str(SNAPSHOT_PACKAGE) if mode == "candidate" else None,
        "candidateSnapshotManifestSha256": snapshot_manifest_sha256,
        "candidateSourceSha256": snapshot_source_sha256,
        "result": str(result),
        "resultSha256": base.sha256(result),
        "provenanceSha256": base.sha256(provenance),
        "guardReceiptSha256": base.sha256(receipt),
        "logSha256": base.sha256(log_path),
        "exitCode": completed.returncode,
        "selected": selected,
    }
    base.write_json_atomic(result.with_suffix(".strict.json"), strict)
    return strict


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    try:
        strict = run(args.mode, args.result)
    except (AssertionError, OSError, ValueError, RuntimeError, json.JSONDecodeError) as error:
        parser.error(str(error))
    print(
        json.dumps(
            {
                "mode": strict["mode"],
                "exitCode": strict["exitCode"],
                "selected": strict["selected"],
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
