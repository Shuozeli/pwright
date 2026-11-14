"""Pure replay and mutation controls for the strict Core v12 pair."""

from __future__ import annotations

import json
from pathlib import Path
from unittest.mock import patch

import promote_playwright_core_select_option32_paired_v12 as promoter
import pytest
import verify_playwright_core_select_option32_paired_v12 as verifier


@pytest.fixture(scope="module")
def pair() -> tuple[dict[str, object], dict[str, bytes]]:
    # Arrange
    inputs = verifier.verify_inputs()

    # Act
    files = promoter.prepare_pair(inputs)

    # Assert
    assert files
    return inputs, files


def test_exact_green_successor_preserves_all_other_raw_rows(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, files = pair
    old_candidate = inputs["oldLines"]["candidate"]
    new_candidate = files["candidate.jsonl"].splitlines(keepends=True)
    red_index = inputs["redIndex"]

    # Act
    receipt = verifier.verify_payloads(files, inputs)

    # Assert
    assert len(inputs["targets"]) == 32
    assert (
        sum(
            before != after
            for before, after in zip(old_candidate, new_candidate, strict=True)
        )
        == 1
    )
    assert all(
        before == after
        for index, (before, after) in enumerate(
            zip(old_candidate, new_candidate, strict=True)
        )
        if index != red_index
    )
    assert files["oracle.jsonl"] == b"".join(inputs["oldLines"]["oracle"])
    red = json.loads(new_candidate[red_index])
    assert (red["specId"], red["outcome"], red["attempts"]) == (
        verifier.RED_SPEC_ID,
        "expected",
        2,
    )
    assert "error" not in red
    assert red["resultFile"] == str(verifier.report_path("candidate"))
    assert receipt["changedRowsPerLane"] == {"candidate": 1, "oracle": 0}
    assert receipt["unchangedRawRowsPerLane"] == {"candidate": 48476, "oracle": 48477}
    assert receipt["browserExecutionEvidence"] is True
    assert receipt["publicApiParityEvidence"] is False
    assert receipt["candidateExpectedRows"] == receipt["oracleExpectedRows"] == 32


def test_candidate_non_target_raw_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, files = pair
    changed = dict(files)
    old = changed["candidate.jsonl"]
    changed["candidate.jsonl"] = old.replace(b'"attempts":0', b'"attempts":1', 1)
    assert changed["candidate.jsonl"] != old

    # Act / Assert
    with pytest.raises(ValueError, match="non-target raw row"):
        verifier.verify_payloads(changed, inputs)


def test_green_target_attempt_count_cannot_be_reset(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, files = pair
    changed = dict(files)
    lines = files["candidate.jsonl"].splitlines(keepends=True)
    red = json.loads(lines[inputs["redIndex"]])
    red["attempts"] = 1
    lines[inputs["redIndex"]] = verifier.canonical_line(red)
    changed["candidate.jsonl"] = b"".join(lines)

    # Act / Assert
    with pytest.raises(ValueError, match="historical RED target row"):
        verifier.verify_payloads(changed, inputs)


def test_oracle_carry_forward_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, files = pair
    changed = dict(files)
    old = changed["oracle.jsonl"]
    changed["oracle.jsonl"] = old.replace(b'"attempts":0', b'"attempts":1', 1)
    assert changed["oracle.jsonl"] != old

    # Act / Assert
    with pytest.raises(ValueError, match="non-target raw row"):
        verifier.verify_payloads(changed, inputs)


def test_receipt_cannot_claim_public_api_parity(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, files = pair
    changed = dict(files)
    receipt = json.loads(files["promotion-receipt.json"])
    receipt["publicApiParityEvidence"] = True
    changed["promotion-receipt.json"] = (json.dumps(receipt) + "\n").encode()

    # Act / Assert
    with pytest.raises(ValueError, match="receipt differs"):
        verifier.verify_payloads(changed, inputs)


def test_report_requires_an_executed_pass_for_every_invocation(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, _ = pair
    report = json.loads(verifier.report_path("candidate").read_bytes())
    report["suites"][0]["specs"][0]["tests"][0]["results"] = []
    target_ids = {row["specId"] for row in inputs["targets"].values()}

    # Act / Assert
    with pytest.raises(ValueError, match="unexecuted or failing invocation"):
        verifier.report_rows("candidate", json.dumps(report).encode(), target_ids)


def test_provenance_rejects_nonzero_exit_code() -> None:
    # Arrange
    provenance = json.loads(verifier.provenance_path("candidate").read_bytes())
    provenance["exitCode"] = 1

    # Act / Assert
    with pytest.raises(ValueError, match="provenance identity"):
        verifier.verify_provenance("candidate", json.dumps(provenance).encode())


def test_frozen_package_manifest_hash_drift_is_rejected(tmp_path: Path) -> None:
    # Arrange
    manifest = tmp_path / "candidate-manifest.json"
    manifest.write_bytes(b"changed")

    # Act / Assert
    with (
        patch.object(verifier, "PACKAGE_MANIFEST", manifest),
        pytest.raises(ValueError, match="SHA-256 changed"),
    ):
        verifier.verify_package()


def test_publish_refuses_existing_successor(tmp_path: Path) -> None:
    # Arrange
    destination = tmp_path / "already-published"
    destination.mkdir()
    marker = destination / "keep"
    marker.write_text("owned by an earlier publication")

    # Act / Assert
    with (
        patch.object(verifier, "V12", destination),
        pytest.raises(ValueError, match="already exists"),
    ):
        promoter.publish({})
    assert marker.read_text() == "owned by an earlier publication"


def test_publish_rejects_malformed_bundle_before_destination_creation(
    tmp_path: Path,
) -> None:
    # Arrange
    destination = tmp_path / "not-created"

    # Act / Assert
    with (
        patch.object(verifier, "V12", destination),
        pytest.raises(ValueError, match="bundle files"),
    ):
        promoter.publish({})
    assert not destination.exists()
