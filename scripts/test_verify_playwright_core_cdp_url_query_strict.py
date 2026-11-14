"""Negative controls for pinned browser-free Core CDP URL-query evidence."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from verify_playwright_core_cdp_url_query_strict import verify_strict

ROOT = Path(__file__).parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"


@pytest.mark.parametrize("mode", ["candidate-39", "oracle"])
def test_verifies_immutable_official_pair(mode: str) -> None:
    # Arrange
    sidecar = OUTCOMES / f"core-cdp-url-query-strict-{mode}-v1.strict.json"

    # Act
    record = verify_strict(sidecar)

    # Assert
    assert len(record["selected"]) == 2


@pytest.mark.parametrize(
    "field,value",
    [
        ("upstreamSourceSha256", "0" * 64),
        ("resultSha256", "0" * 64),
        ("guardReceiptSha256", "0" * 64),
        ("candidateProfile", "default"),
    ],
)
def test_rejects_tampered_attestation(tmp_path: Path, field: str, value: str) -> None:
    # Arrange
    original = OUTCOMES / "core-cdp-url-query-strict-candidate-39-v1.strict.json"
    record = json.loads(original.read_text())
    record[field] = value
    tampered = tmp_path / "tampered.strict.json"
    tampered.write_text(json.dumps(record))

    # Act / Assert
    with pytest.raises(AssertionError):
        verify_strict(tampered)


def test_rejects_fabricated_official_status(tmp_path: Path) -> None:
    # Arrange
    original = OUTCOMES / "core-cdp-url-query-strict-candidate-39-v1.strict.json"
    record = json.loads(original.read_text())
    record["selected"][0]["status"] = (
        "unexpected" if record["selected"][0]["status"] == "expected" else "expected"
    )
    tampered = tmp_path / "fabricated-pass.strict.json"
    tampered.write_text(json.dumps(record))

    # Act / Assert
    with pytest.raises(AssertionError):
        verify_strict(tampered)
