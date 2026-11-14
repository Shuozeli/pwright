"""Pure evidence and mutation controls for the strict Core v11 pair."""

from __future__ import annotations

import json
from pathlib import Path
from unittest.mock import patch

import promote_playwright_core_select_option32_paired_v11 as promoter
import pytest
import verify_playwright_core_select_option32_paired_v11 as verifier


def _files() -> tuple[dict[str, object], dict[str, bytes]]:
    inputs = verifier.verify_inputs()
    return inputs, promoter.prepare_pair(inputs)


def test_strict_pair_has_exact_new_outcomes_and_preserves_predecessor() -> None:
    # Arrange
    inputs, files = _files()

    # Act
    receipt = verifier.verify_payloads(files, inputs)

    # Assert
    assert len(inputs["targets"]) == 32
    assert list(inputs["targets"]) == list(range(40893, 40987, 3))
    assert receipt["candidateExpectedRows"] == 31
    assert receipt["candidateUnexpectedRows"] == 1
    assert receipt["oracleExpectedRows"] == 32
    assert receipt["browserExecutionEvidence"] is True
    assert receipt["publicApiParityEvidence"] is False
    assert receipt["unchangedRawRowsPerLane"] == 48445
    assert json.loads(files["candidate.summary.json"])["outcomes"] == {
        "expected": 4612,
        "not-run": 43788,
        "skipped": 21,
        "unexpected": 56,
    }


def test_non_target_raw_line_mutation_is_rejected() -> None:
    # Arrange
    inputs, files = _files()
    changed = dict(files)
    old = changed["candidate.jsonl"]
    changed["candidate.jsonl"] = old.replace(b'"attempts":0', b'"attempts":1', 1)
    assert changed["candidate.jsonl"] != old

    # Act / Assert
    with pytest.raises(ValueError, match="non-target raw row"):
        verifier.verify_payloads(changed, inputs)


def test_historical_candidate_red_cannot_be_greenwashed() -> None:
    # Arrange
    inputs, files = _files()
    changed = dict(files)
    lines = files["candidate.jsonl"].splitlines(keepends=True)
    red_index = next(
        index
        for index, row in inputs["targets"].items()
        if row["specId"] == verifier.RED_SPEC_ID
    )
    red = json.loads(lines[red_index])
    assert red["outcome"] == "unexpected"
    red["outcome"] = "expected"
    lines[red_index] = verifier.canonical_line(red)
    changed["candidate.jsonl"] = b"".join(lines)

    # Act / Assert
    with pytest.raises(ValueError, match="target row"):
        verifier.verify_payloads(changed, inputs)


def test_receipt_cannot_gain_an_unverified_parity_claim() -> None:
    # Arrange
    inputs, files = _files()
    changed = dict(files)
    receipt = json.loads(files["promotion-receipt.json"])
    receipt["publicApiParityEvidenceScope"] = "all browsers"
    changed["promotion-receipt.json"] = (json.dumps(receipt) + "\n").encode()

    # Act / Assert
    with pytest.raises(ValueError, match="receipt field set"):
        verifier.verify_payloads(changed, inputs)


def test_full_file_report_rejects_a_missing_execution_attempt() -> None:
    # Arrange
    inputs = verifier.verify_inputs()
    report = json.loads(verifier.report_path("candidate").read_bytes())
    report["suites"][0]["specs"][0]["tests"][0]["results"] = []

    # Act / Assert
    with pytest.raises(ValueError, match="unexecuted invocation"):
        verifier._report_rows(
            "candidate",
            json.dumps(report).encode(),
            {row["specId"] for row in inputs["targets"].values()},
        )


def test_report_or_provenance_hash_drift_is_rejected(tmp_path: Path) -> None:
    # Arrange
    artifact = tmp_path / "evidence.json"
    artifact.write_bytes(b"changed")

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 changed"):
        verifier.sealed(
            artifact, verifier.REPORT_SHA256["candidate"], "candidate report"
        )


def test_publish_refuses_an_existing_successor(tmp_path: Path) -> None:
    # Arrange
    destination = tmp_path / "already-published"
    destination.mkdir()
    marker = destination / "keep"
    marker.write_text("owned by an earlier publication")

    # Act / Assert
    with (
        patch.object(verifier, "V11", destination),
        pytest.raises(ValueError, match="already exists"),
    ):
        promoter.publish({})
    assert marker.read_text() == "owned by an earlier publication"


def test_publish_refuses_a_malformed_bundle_before_creating_destination(
    tmp_path: Path,
) -> None:
    # Arrange
    destination = tmp_path / "not-created"

    # Act / Assert
    with (
        patch.object(verifier, "V11", destination),
        pytest.raises(ValueError, match="bundle files"),
    ):
        promoter.publish({})
    assert not destination.exists()
