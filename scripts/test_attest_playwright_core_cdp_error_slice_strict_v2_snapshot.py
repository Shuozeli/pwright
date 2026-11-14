"""Fail-closed controls for the archived candidate source snapshot."""

from __future__ import annotations

from pathlib import Path

import attest_playwright_core_cdp_error_slice_strict_v2_snapshot as attestor
import promote_playwright_core_cdp_error_slice_strict_v2 as promotion
import pytest
import run_playwright_core_cdp_error_slice_strict_v1 as source
from run_playwright_official_conformance import _candidate_source_sha256


def fixture_snapshot(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    package = tmp_path / "package"
    package.mkdir()
    (package / "package.json").write_text('{"name":"archive-fixture"}\n')
    (package / "src").mkdir()
    (package / "src/index.cjs").write_text("module.exports = {};\n")
    artifact = package / "native" / "profile" / "pwright.node"
    artifact.parent.mkdir(parents=True)
    artifact.write_bytes(b"test-native-artifact")
    outcome = tmp_path / "outcome"
    outcome.mkdir()
    receipt = outcome / "promotion-receipt.json"
    receipt.write_text("{}\n")
    monkeypatch.setattr(attestor, "SNAPSHOT", package)
    monkeypatch.setattr(attestor, "ARTIFACT", artifact)
    monkeypatch.setattr(attestor, "SOURCE_SHA256", _candidate_source_sha256(package))
    monkeypatch.setattr(attestor, "RECEIPT_SHA256", source.sha256(receipt))
    monkeypatch.setattr(promotion, "OUTPUT", outcome)
    monkeypatch.setattr(source, "ARTIFACT_SHA256", source.sha256(artifact))
    return package


def test_capture_time_snapshot_accepts_exact_source_and_artifact(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    package = fixture_snapshot(tmp_path, monkeypatch)

    # Act
    attestor.verify_snapshot()

    # Assert
    assert (package / "src/index.cjs").is_file()


def test_capture_time_snapshot_rejects_changed_source(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    package = fixture_snapshot(tmp_path, monkeypatch)
    (package / "src/index.cjs").write_text("module.exports = { changed: true };\n")

    # Act
    with pytest.raises(ValueError, match="snapshot source SHA drift"):
        attestor.verify_snapshot()

    # Assert
    assert _candidate_source_sha256(package) != attestor.SOURCE_SHA256


def test_capture_time_snapshot_rejects_symlink(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    package = fixture_snapshot(tmp_path, monkeypatch)
    (package / "src/link.cjs").symlink_to(package / "src/index.cjs")

    # Act
    with pytest.raises(ValueError, match="snapshot symlink"):
        attestor.verify_snapshot()

    # Assert
    assert (package / "src/link.cjs").is_symlink()


def test_capture_time_snapshot_rejects_changed_artifact(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    fixture_snapshot(tmp_path, monkeypatch)
    attestor.ARTIFACT.write_bytes(b"changed-native-artifact")

    # Act
    with pytest.raises(ValueError, match="snapshot artifact SHA drift"):
        attestor.verify_snapshot()

    # Assert
    assert source.sha256(attestor.ARTIFACT) != source.ARTIFACT_SHA256
