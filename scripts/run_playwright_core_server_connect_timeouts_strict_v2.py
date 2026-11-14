"""Rerun the v1 upstream Core timeout slice with an isolated fixed addon.

The v1 manifest, source guard, assertions, and RED evidence are unchanged.
This wrapper stages a distinct candidate package and adds its own pinned input
hashes to the v1 capture/verification machinery. Results remain diagnostic.
"""

from __future__ import annotations

import argparse
import os
import shutil
import tempfile
from pathlib import Path

import run_playwright_core_server_connect_timeouts_strict_v1 as v1
from official_candidate_package_snapshot_v1 import sha256
from run_playwright_official_conformance import _candidate_source_sha256

ROOT = Path(__file__).parents[1].resolve()
ARTIFACT_ROOT = (
    ROOT
    / ".work/official-conformance/artifacts/playwright-server-connect-session-timeout-map-v2-20261012"
)
ARTIFACT = ARTIFACT_ROOT / "pwright.node"
ARTIFACT_SHA256 = "a2e668d626749e6799f4888ee089bf318925c6929d5bc032adc4e3577f85448c"
SOURCE_PACKAGE = ARTIFACT_ROOT / "candidate-package"
RESULT_PARENT = (
    ROOT / ".work/official-conformance/results/core-server-connect-timeouts-strict-v2"
)
SCOPE = "official-derived-core-server-connect-timeouts-fixture-host-adapted-internal-v2"
INPUT_FILES = (
    *v1.INPUT_FILES,
    Path(__file__).resolve(),
    ROOT / "scripts/verify_playwright_core_server_connect_timeouts_strict_v2.py",
    ARTIFACT,
)


def configure() -> None:
    if not ARTIFACT.is_file() or ARTIFACT.is_symlink():
        raise ValueError("isolated addon is missing or a symlink")
    if sha256(ARTIFACT) != ARTIFACT_SHA256:
        raise ValueError("isolated addon differs from its pinned digest")
    v1.ARTIFACT_SHA256 = ARTIFACT_SHA256
    v1.SOURCE_PACKAGE = SOURCE_PACKAGE
    v1.RESULT_PARENT = RESULT_PARENT
    v1.SCOPE = SCOPE
    v1.INPUT_FILES = INPUT_FILES


def stage_source_package() -> None:
    """Publish one source snapshot without touching the prior profile addon."""
    if SOURCE_PACKAGE.exists() or SOURCE_PACKAGE.is_symlink():
        raise ValueError("isolated source-package destination already exists")
    source = ROOT / "packages/pwright"
    before = _candidate_source_sha256(source)
    ARTIFACT_ROOT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(
        prefix=".candidate-package-v2.", dir=ARTIFACT_ROOT
    ) as temp:
        staged = Path(temp) / "package"
        staged.mkdir()
        shutil.copyfile(source / "package.json", staged / "package.json")
        shutil.copytree(source / "src", staged / "src")
        addon_destination = staged / "native" / v1.PROFILE / "pwright.node"
        addon_destination.parent.mkdir(parents=True)
        shutil.copyfile(ARTIFACT, addon_destination)
        after = _candidate_source_sha256(source)
        if before != after or _candidate_source_sha256(staged) != before:
            raise ValueError("candidate source changed during isolated staging")
        if sha256(addon_destination) != ARTIFACT_SHA256:
            raise ValueError("isolated addon changed during staging")
        if SOURCE_PACKAGE.exists() or SOURCE_PACKAGE.is_symlink():
            raise ValueError("isolated source-package destination appeared")
        os.rename(staged, SOURCE_PACKAGE)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    configure()
    stage_source_package()
    attestation = v1.capture(args.result_dir)
    for mode in ("oracle", "candidate"):
        lane = attestation["lanes"][mode]
        print(f"{mode}: {lane['passed']}/36 expected, {lane['unexpected']} unexpected")
    print(f"diagnostic only: {args.result_dir / 'attestation.json'}")


if __name__ == "__main__":
    main()
