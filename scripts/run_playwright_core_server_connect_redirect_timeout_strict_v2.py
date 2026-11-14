"""Rerun the exact nine official redirect-timeout rows with a new isolated addon.

This v2 diagnostic reuses the immutable v1 selection/guard and preserves its
RED attestation. It neither changes canonical ledgers nor launches a browser.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
from pathlib import Path

import run_playwright_core_server_connect_redirect_timeout_strict_v1 as baseline
from official_candidate_package_snapshot_v1 import sha256, verify_snapshot
from run_playwright_core_server_connect_timeouts_strict_v1 import _write_json_exclusive

ROOT = baseline.ROOT
UPSTREAM = baseline.UPSTREAM
MANIFEST = baseline.MANIFEST
PRELOAD = baseline.PRELOAD
PROFILE = baseline.PROFILE
PROJECTS_BY_SUITE = baseline.PROJECTS_BY_SUITE
SOURCE = baseline.SOURCE
LINE = baseline.LINE
COMMIT = baseline.COMMIT
SOURCE_SHA256 = baseline.SOURCE_SHA256
IDENTITIES_SHA256 = baseline.IDENTITIES_SHA256
IDENTITIES_SHA256_BY_SUITE = baseline.IDENTITIES_SHA256_BY_SUITE
ARTIFACT_SHA256 = "0f1c8b1c6140206bdb4692d88ff1d1b9fb235b883739f176f720c3b59361523e"
CANDIDATE_SOURCE_SHA256 = (
    "bddad5646a880efc80a94d7bf4b975e4eb06691562ac6602a8df29c3cf97afe2"
)
ARTIFACT_DIR = (
    ROOT
    / ".work/official-conformance/artifacts/playwright-server-connect-http-header-order-v1-20261012"
)
SOURCE_PACKAGE = ARTIFACT_DIR / "candidate-package"
SNAPSHOT = ARTIFACT_DIR / "candidate-package-snapshot"
SNAPSHOT_MANIFEST = ARTIFACT_DIR / "candidate-package-snapshot.files.json"
SNAPSHOT_MANIFEST_SHA256 = (
    "a79c22bc1d160cc46379806820ee10d06ad4762aee3c037522a798cb129599b8"
)
PREDECESSOR = (
    ROOT
    / ".work/official-conformance/results/core-server-connect-redirect-timeout-strict-v1/core-server-connect-redirect-timeout-strict-v1-second-20261012/attestation.json"
)
PREDECESSOR_SHA256 = "35950e9ed84316f518a6541b7b8ebd803ff27b74fb70b3f9c54b28cb9b9656dc"
RESULT_PARENT = (
    ROOT
    / ".work/official-conformance/results/core-server-connect-redirect-timeout-strict-v2"
)
SCOPE = "official-derived-core-server-connect-redirect-timeout-fixture-host-adapted-internal-v2"
INPUT_FILES = baseline.INPUT_FILES + (
    PREDECESSOR,
    SNAPSHOT_MANIFEST,
    Path(__file__).resolve(),
    ROOT
    / "scripts/verify_playwright_core_server_connect_redirect_timeout_strict_v2.py",
    ROOT
    / "scripts/test_verify_playwright_core_server_connect_redirect_timeout_strict_v2.py",
)


def check_sources() -> dict[str, object]:
    baseline.check_sources()
    assert sha256(PREDECESSOR) == PREDECESSOR_SHA256
    prior = json.loads(PREDECESSOR.read_text())
    assert prior["scope"] == baseline.SCOPE
    assert prior["lanes"]["oracle"]["passed"] == 9
    assert prior["lanes"]["candidate"]["passed"] == 0
    assert prior["lanes"]["candidate"]["unexpected"] == 9
    assert prior["promotedToCoreLedger"] is False
    assert sha256(SNAPSHOT_MANIFEST) == SNAPSHOT_MANIFEST_SHA256
    snapshot = verify_snapshot(
        SNAPSHOT,
        SNAPSHOT_MANIFEST,
        source_package=SOURCE_PACKAGE,
        profile=PROFILE,
        artifact_sha256=ARTIFACT_SHA256,
    )
    assert snapshot["candidateSourceSha256"] == CANDIDATE_SOURCE_SHA256
    return snapshot


def capture(result_dir: Path) -> dict[str, object]:
    if any(os.environ.get(key) for key in baseline.FORBIDDEN_OVERRIDES):
        raise ValueError(
            "strict redirect timeout diagnostic rejects inherited runtime overrides"
        )
    source_snapshot = check_sources()
    baseline.fixture_dns_preflight()
    result_dir = result_dir.absolute()
    if (
        result_dir.parent.resolve() != RESULT_PARENT.resolve()
        or not result_dir.name.startswith(
            "core-server-connect-redirect-timeout-strict-v2-"
        )
        or result_dir.exists()
        or result_dir.is_symlink()
    ):
        raise ValueError("redirect timeout v2 destination must be fresh and versioned")
    inputs = {str(path.relative_to(ROOT)): sha256(path) for path in INPUT_FILES}
    result_dir.mkdir(parents=True, exist_ok=False)
    for mode in ("oracle", "candidate"):
        for suite_id in PROJECTS_BY_SUITE:
            result = result_dir / f"{mode}-{suite_id}.json"
            receipt = result_dir / f"{mode}-{suite_id}.receipt.jsonl"
            log = result_dir / f"{mode}-{suite_id}.log"
            receipt.open("x").close()
            env = os.environ.copy()
            env["NODE_OPTIONS"] = f"--require={PRELOAD}"
            env["PWRIGHT_CORE_SERVER_REDIRECT_TIMEOUT_V1_MODE"] = mode
            env["PWRIGHT_CORE_SERVER_REDIRECT_TIMEOUT_V1_RECEIPT"] = str(receipt)
            command = [
                "uv",
                "run",
                "--with",
                "pytest",
                str(ROOT / "scripts/run_playwright_official_conformance.py"),
                mode,
                str(UPSTREAM),
                "--suite",
                suite_id,
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
                    str(SNAPSHOT),
                    "--candidate-profile",
                    PROFILE,
                ]
            completed = subprocess.run(
                command,
                cwd=ROOT,
                env=env,
                capture_output=True,
                text=True,
                check=False,
                timeout=240,
            )
            log.write_text(completed.stdout + completed.stderr)
            if (
                not result.is_file()
                or not result.with_suffix(".provenance.json").is_file()
            ):
                raise RuntimeError(
                    f"{mode}/{suite_id} produced no JSON/provenance (exit {completed.returncode}): "
                    f"{log.read_text()[-3000:]}"
                )
            selected = baseline.selected_tests(json.loads(result.read_text()), suite_id)
            assert completed.returncode == (
                0 if all(item["status"] == "expected" for item in selected) else 1
            )
    assert inputs == {str(path.relative_to(ROOT)): sha256(path) for path in INPUT_FILES}
    assert check_sources() == source_snapshot
    from verify_playwright_core_server_connect_redirect_timeout_strict_v2 import (
        verify_directory,
    )

    attestation = verify_directory(result_dir, require_attestation=False)
    _write_json_exclusive(result_dir / "attestation.json", attestation)
    verify_directory(result_dir)
    return attestation


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    attestation = capture(args.result_dir)
    for mode in ("oracle", "candidate"):
        lane = attestation["lanes"][mode]
        print(f"{mode}: {lane['passed']}/9 expected, {lane['unexpected']} unexpected")
    print(f"diagnostic only: {args.result_dir / 'attestation.json'}")


if __name__ == "__main__":
    main()
