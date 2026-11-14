"""Fail-closed controls for the post-fix frozen candidate attestation."""

from __future__ import annotations

import json
from pathlib import Path

import attest_playwright_core_cdp_error_slice_strict_v3_snapshot as attestor
import promote_playwright_core_cdp_error_slice_strict_v3 as promotion
import pytest
import run_playwright_core_cdp_error_slice_strict_v3 as capture


def test_attestation_rejects_wrong_receipt_hash(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    seal = attestor.expected_attestation()
    seal["pairedReceiptSha256"] = "0" * 64
    path = tmp_path / "attestation.json"
    path.write_text(json.dumps(seal))
    monkeypatch.setattr(attestor, "ATTESTATION", path)

    # Act
    with pytest.raises(ValueError, match="attestation drift"):
        attestor.verify_attestation()

    # Assert
    assert seal["pairedReceiptSha256"] != attestor.RECEIPT_SHA256


def test_attestation_rejects_missing_seal(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    path = tmp_path / "missing.json"
    monkeypatch.setattr(attestor, "ATTESTATION", path)

    # Act
    with pytest.raises(ValueError, match="attestation missing"):
        attestor.verify_attestation()

    # Assert
    assert not path.exists()


def test_attestation_rejects_changed_snapshot_manifest(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    path = tmp_path / "candidate-package-snapshot-v1.files.json"
    path.write_text("{}\n")
    monkeypatch.setattr(capture, "SNAPSHOT_MANIFEST", path)

    # Act
    with pytest.raises(ValueError, match="snapshot manifest SHA drift"):
        attestor.verify_capture_snapshot()

    # Assert
    assert attestor.sha256(path) != attestor.SNAPSHOT_MANIFEST_SHA256


def test_pair_reverification_uses_snapshot_artifact_and_restores_original(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    original = promotion.ARTIFACT_PATH
    expected = capture.SNAPSHOT_PACKAGE / "native" / capture.PROFILE / "pwright.node"

    def verify_existing() -> dict:
        assert promotion.ARTIFACT_PATH == expected
        return {"publicApiParityEvidence": False}

    monkeypatch.setattr(promotion, "verify_existing", verify_existing)

    # Act
    receipt = attestor.verify_pair_from_snapshot()

    # Assert
    assert receipt["publicApiParityEvidence"] is False
    assert promotion.ARTIFACT_PATH == original


def test_pair_reverification_restores_artifact_after_failure(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    original = promotion.ARTIFACT_PATH

    def fail() -> dict:
        raise ValueError("synthetic ledger mismatch")

    monkeypatch.setattr(promotion, "verify_existing", fail)

    # Act
    with pytest.raises(ValueError, match="synthetic ledger mismatch"):
        attestor.verify_pair_from_snapshot()

    # Assert
    assert promotion.ARTIFACT_PATH == original
