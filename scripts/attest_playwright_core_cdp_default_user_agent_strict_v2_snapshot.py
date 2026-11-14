#!/usr/bin/env python3
"""Reverify the UA v2 pair against the same capture-time candidate source."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import promote_playwright_core_cdp_default_user_agent_strict_v2 as promotion
import verify_playwright_core_cdp_default_user_agent_strict_v2 as original
from attest_playwright_core_cdp_error_slice_strict_v2_snapshot import (
    ATTESTATION,
    SNAPSHOT,
    SOURCE_SHA256,
    verify_attestation,
    verify_snapshot,
)
from verify_playwright_official_provenance import verify_provenance

ROOT = Path(__file__).resolve().parents[1]
ERROR_SEAL = ATTESTATION
RECEIPT_SHA256 = "1fccc683abb6a19505585c032449dab260b85f18312fd55f6ac6027e253b30cd"
ERROR_SEAL_SHA256 = "65e96b04c6e1cc0edcad15c63dd4ae67ecb4fca9441716a390de52016fef6d7b"
SEAL = (
    ROOT
    / ".work/official-conformance/outcomes/core-cdp-default-user-agent-strict-paired-v2-seal-v1"
    / "attestation.json"
)


def verify_seal() -> None:
    if not SEAL.is_file() or SEAL.is_symlink():
        raise ValueError("UA v2 snapshot seal missing")
    if promotion.sha256(ERROR_SEAL) != ERROR_SEAL_SHA256:
        raise ValueError("CDP error snapshot seal drift")
    expected = {
        "schema": 1,
        "scope": "historical-cdp-default-user-agent-candidate-source-snapshot",
        "snapshotPackage": str(SNAPSHOT),
        "candidateSourceSha256": SOURCE_SHA256,
        "candidateArtifactSha256": (
            "92f039f5530075841ca7d59ff523c1f6b7a5ec4884a29ff20042ef3c95826fb2"
        ),
        "priorPairedReceiptSha256": RECEIPT_SHA256,
        "snapshotSealSha256": ERROR_SEAL_SHA256,
        "attestorSha256": promotion.sha256(Path(__file__)),
        "publicApiParityEvidence": False,
    }
    if json.loads(SEAL.read_text()) != expected:
        raise ValueError("UA v2 snapshot seal fields or source pin drift")


def verify() -> dict[str, Any]:
    verify_snapshot()
    verify_attestation()
    verify_seal()
    if promotion.sha256(promotion.OUTPUT / "promotion-receipt.json") != RECEIPT_SHA256:
        raise ValueError("UA v2 paired receipt drift")
    old = original.verify_provenance

    def archived_provenance(payload: dict[str, Any], **kwargs: Any) -> dict[str, Any]:
        if payload["mode"] == "candidate":
            kwargs["candidate_override"] = SNAPSHOT
            if payload["candidateSourceSha256"] != SOURCE_SHA256:
                raise ValueError("UA v2 capture-time source digest drift")
        return verify_provenance(payload, **kwargs)

    original.verify_provenance = archived_provenance
    try:
        receipt = promotion.verify_existing()
    finally:
        original.verify_provenance = old
    if receipt["publicApiParityEvidence"] is not False:
        raise ValueError("internal UA test misclassified as public API parity")
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    try:
        receipt = verify()
    except (
        AssertionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"historical Core UA pair attestation rejected: {error}")
    print(
        f"verified archived Core UA pair sourceSha256={SOURCE_SHA256} "
        f"targetRow={receipt['targetZeroBasedIndex']}"
    )


if __name__ == "__main__":
    main()
