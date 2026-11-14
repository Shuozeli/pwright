"""Fail closed on provenance, artifact, fixture-shim and result tampering."""

from __future__ import annotations

import copy
import json
from pathlib import Path

import pytest
from verify_playwright_core_browser_type_owner_strict_v2 import verify_strict

ROOT = Path(__file__).parents[1].resolve()
RESULTS = ROOT / ".work/official-conformance/results"
CANDIDATE = RESULTS / "playwright-core-candidate-browsertype-owner-strict-v2.strict.json"
ORACLE = RESULTS / "playwright-core-oracle-browsertype-owner-strict-v2.strict.json"


@pytest.mark.parametrize("source", [CANDIDATE, ORACLE])
def test_authentic_strict_evidence_verifies(source: Path) -> None:
    # Arrange
    sidecar = source
    # Act
    verified = verify_strict(sidecar)
    # Assert
    assert len(verified["selected"]) == 3


@pytest.mark.parametrize(
    "field",
    ["resultSha256", "provenanceSha256", "ownerReceiptSha256", "candidateArtifactSha256"],
)
def test_changed_digest_is_rejected(tmp_path: Path, field: str) -> None:
    # Arrange
    strict = copy.deepcopy(json.loads(CANDIDATE.read_text()))
    strict[field] = "0" * 64
    forged = tmp_path / "forged.strict.json"
    forged.write_text(json.dumps(strict))
    # Act / Assert
    with pytest.raises(AssertionError):
        verify_strict(forged)


def test_changed_selected_outcome_is_rejected(tmp_path: Path) -> None:
    # Arrange
    strict = copy.deepcopy(json.loads(CANDIDATE.read_text()))
    strict["selected"][0]["status"] = "unexpected"
    forged = tmp_path / "forged.strict.json"
    forged.write_text(json.dumps(strict))
    # Act / Assert
    with pytest.raises(AssertionError):
        verify_strict(forged)


def test_changed_fixture_input_is_rejected(tmp_path: Path) -> None:
    # Arrange
    strict = copy.deepcopy(json.loads(CANDIDATE.read_text()))
    name = "tests/official/playwright-core-browser-type-owner-v2-preload.cjs"
    strict["inputs"][name] = "0" * 64
    forged = tmp_path / "forged.strict.json"
    forged.write_text(json.dumps(strict))
    # Act / Assert
    with pytest.raises(AssertionError):
        verify_strict(forged)
