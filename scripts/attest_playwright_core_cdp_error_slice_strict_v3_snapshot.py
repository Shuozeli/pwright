#!/usr/bin/env python3
"""Reverify the post-fix Core CDP pair after the live addon is rebuilt.

The v3 candidate ran from a frozen package snapshot. Its original promoter
also checks the then-current addon path; this attestor substitutes only that
artifact check with the already hash-verified snapshot artifact. It does not
alter the official reports, strict sidecars, promoter, or paired ledgers.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import promote_playwright_core_cdp_error_slice_strict_v3 as promotion
import run_playwright_core_cdp_error_slice_strict_v3 as capture
from official_candidate_package_snapshot_v1 import sha256, verify_snapshot

RECEIPT_SHA256 = "1f65ad5e2d26ec874fa7c386439c2ef28be58335233cfd5bf4d1f397356d555d"
SNAPSHOT_MANIFEST_SHA256 = "c90f2f9d9b7904610abfeb3a37ee720e988b37748bedf8849cf36c7658534115"
SOURCE_SHA256 = "c4d7f9022e10ef17445bab1edacf01931d526a69d5e8415a0bf770140164dfe7"
PROMOTER_SHA256 = "582a8d633d27a2ac7945b6481090792a4b15867a8416512c5a88cee2a3b22977"
VERIFIER_SHA256 = "35903482deaec3d4d03fcbe51775fd4a9fc5bb9c5bed90f0015c8d3edcb054da"
ATTESTATION = (
    capture.ROOT
    / ".work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v3-seal-v1"
    / "attestation.json"
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def expected_attestation() -> dict[str, Any]:
    return {
        "schema": 1,
        "scope": "post-fix-candidate-package-snapshot",
        "upstreamCommit": promotion.source.COMMIT,
        "snapshotPackage": str(capture.SNAPSHOT_PACKAGE),
        "candidateSnapshotManifestSha256": SNAPSHOT_MANIFEST_SHA256,
        "candidateSourceSha256": SOURCE_SHA256,
        "candidateProfile": capture.PROFILE,
        "candidateArtifactSha256": capture.ARTIFACT_SHA256,
        "pairedReceiptSha256": RECEIPT_SHA256,
        "promoterSha256": PROMOTER_SHA256,
        "strictVerifierSha256": VERIFIER_SHA256,
        "attestorSha256": sha256(Path(__file__)),
        "attestorTestsSha256": sha256(
            capture.ROOT
            / "scripts/test_attest_playwright_core_cdp_error_slice_strict_v3_snapshot.py"
        ),
        "recordsPerLane": 48_477,
        "targetZeroBasedIndexes": sorted(promotion.TARGETS),
        "publicApiParityEvidence": False,
    }


def verify_attestation() -> None:
    require(ATTESTATION.is_file() and not ATTESTATION.is_symlink(), "attestation missing")
    require(json.loads(ATTESTATION.read_text()) == expected_attestation(), "attestation drift")


def verify_capture_snapshot() -> None:
    require(
        sha256(capture.SNAPSHOT_MANIFEST) == SNAPSHOT_MANIFEST_SHA256,
        "snapshot manifest SHA drift",
    )
    manifest = verify_snapshot(
        capture.SNAPSHOT_PACKAGE,
        capture.SNAPSHOT_MANIFEST,
        source_package=capture.SOURCE_PACKAGE,
        profile=capture.PROFILE,
        artifact_sha256=capture.ARTIFACT_SHA256,
    )
    require(manifest["candidateSourceSha256"] == SOURCE_SHA256, "snapshot source SHA drift")
    require(
        sha256(promotion.OUTPUT / "promotion-receipt.json") == RECEIPT_SHA256,
        "paired receipt SHA drift",
    )
    require(sha256(Path(promotion.__file__)) == PROMOTER_SHA256, "promoter SHA drift")
    require(sha256(promotion.VERIFIER) == VERIFIER_SHA256, "strict verifier SHA drift")


def verify_pair_from_snapshot() -> dict[str, Any]:
    original_artifact = promotion.ARTIFACT_PATH
    promotion.ARTIFACT_PATH = capture.SNAPSHOT_PACKAGE / "native" / capture.PROFILE / "pwright.node"
    try:
        receipt = promotion.verify_existing()
    finally:
        promotion.ARTIFACT_PATH = original_artifact
    require(receipt["publicApiParityEvidence"] is False, "internal slice misclassified")
    return receipt


def verify() -> dict[str, Any]:
    verify_attestation()
    verify_capture_snapshot()
    return verify_pair_from_snapshot()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    try:
        receipt = verify()
    except (AssertionError, OSError, ValueError, KeyError, TypeError) as error:
        parser.error(f"post-fix Core CDP error pair attestation rejected: {error}")
    print(
        "verified post-fix Core CDP error pair "
        f"candidateSourceSha256={SOURCE_SHA256} "
        f"targetRows={receipt['targetZeroBasedIndexes']}"
    )


if __name__ == "__main__":
    main()
