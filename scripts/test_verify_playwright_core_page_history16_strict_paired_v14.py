"""Read-only replay and mutation controls for the paired Core history v14 ledger."""

from __future__ import annotations

import json
from pathlib import Path
from unittest.mock import patch

import pytest
import verify_playwright_core_page_history16_strict_paired_v14 as verifier


@pytest.fixture(scope="module")
def pair() -> tuple[dict[str, object], dict[str, bytes]]:
    # Arrange
    checked = verifier.verify_inputs()
    files: dict[str, bytes] = {}
    for lane in verifier.LANES:
        lines = list(checked["oldLines"][lane])
        for index, row in checked["replacementRows"][lane].items():
            lines[index] = verifier.canonical_line(row)
        ledger = b"".join(lines)
        files[f"{lane}.jsonl"] = ledger
        summary = verifier.expected_summary(lane, checked["oldSummaries"][lane], ledger)
        files[f"{lane}.summary.json"] = (json.dumps(summary, indent=2) + "\n").encode()
    receipt = verifier.expected_receipt(files, checked)
    files["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()

    # Act
    observed = verifier.verify_payloads(files, checked)

    # Assert
    assert observed == receipt
    return checked, files


def _replace_row(
    files: dict[str, bytes], lane: str, index: int, key: str, value: object
) -> dict[str, bytes]:
    changed = dict(files)
    lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
    row = json.loads(lines[index])
    row[key] = value
    lines[index] = verifier.canonical_line(row)
    changed[f"{lane}.jsonl"] = b"".join(lines)
    return changed


def _replace_receipt(
    files: dict[str, bytes], key: str, value: object
) -> dict[str, bytes]:
    changed = dict(files)
    receipt = json.loads(files["promotion-receipt.json"])
    receipt[key] = value
    changed["promotion-receipt.json"] = (json.dumps(receipt) + "\n").encode()
    return changed


def _report(lane: str) -> dict[str, object]:
    return json.loads(verifier.report_path(lane).read_bytes())


def _target_ids() -> set[str]:
    return {spec_id for _, spec_id, _ in verifier.TARGETS}


def test_exact_red_successor_preserves_every_off_target_raw_row(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    checked, files = pair
    target_indexes = {index for index, _, _ in verifier.TARGETS}

    # Act
    receipt = verifier.verify_payloads(files, checked)

    # Assert
    assert len(target_indexes) == 16
    for lane in verifier.LANES:
        before = checked["oldLines"][lane]
        after = files[f"{lane}.jsonl"].splitlines(keepends=True)
        assert len(before) == len(after) == 48_477
        assert (
            sum(left != right for left, right in zip(before, after, strict=True)) == 16
        )
        assert all(
            left == right
            for index, (left, right) in enumerate(zip(before, after, strict=True))
            if index not in target_indexes
        )
    assert receipt["changedRowsPerLane"] == {"candidate": 16, "oracle": 16}
    assert receipt["unchangedRawRowsPerLane"] == {"candidate": 48_461, "oracle": 48_461}
    assert (receipt["candidateExpectedRows"], receipt["candidateUnexpectedRows"]) == (
        11,
        5,
    )
    assert (receipt["oracleExpectedRows"], receipt["oracleUnexpectedRows"]) == (13, 3)
    assert receipt["completeFileExecution"] is True
    assert receipt["fullCorpusExecution"] is False
    assert receipt["publicApiParityEvidence"] is False


@pytest.mark.parametrize("lane", verifier.LANES)
def test_off_target_raw_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
    lane: str,
) -> None:
    # Arrange
    checked, files = pair
    changed = _replace_row(files, lane, 0, "attempts", 99)

    # Act / Assert
    with pytest.raises(ValueError, match="off-target raw row"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize("lane", verifier.LANES)
@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("attempts", 0),
        ("outcome", "expected"),
        ("error", "fabricated"),
        ("evidenceScope", "other-scope"),
        ("browserExecutionEvidence", False),
        ("publicApiParityEvidence", True),
    ],
)
def test_red_target_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
    lane: str,
    key: str,
    value: object,
) -> None:
    # Arrange
    checked, files = pair
    red_index = next(
        index
        for index, row in checked["replacementRows"][lane].items()
        if row["outcome"] == "unexpected"
    )
    changed = _replace_row(files, lane, red_index, key, value)

    # Act / Assert
    with pytest.raises(ValueError, match="target row"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize("lane", verifier.LANES)
def test_green_target_fabricated_error_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
    lane: str,
) -> None:
    # Arrange
    checked, files = pair
    green_index = next(
        index
        for index, row in checked["replacementRows"][lane].items()
        if row["outcome"] == "expected"
    )
    changed = _replace_row(files, lane, green_index, "error", "hidden failure")

    # Act / Assert
    with pytest.raises(ValueError, match="target row"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("fullCorpusExecution", True),
        ("completeFileExecution", False),
        ("publicApiParityEvidence", True),
        ("candidateExpectedRows", 16),
        ("oracleUnexpectedRows", 0),
        ("targetZeroBasedIndexes", []),
        ("unchangedRawRowsPerLane", {"candidate": 48_477, "oracle": 48_477}),
        ("manifestSha256", "0" * 64),
        ("redComparison", {}),
    ],
)
def test_receipt_scope_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
    key: str,
    value: object,
) -> None:
    # Arrange
    checked, files = pair
    changed = _replace_receipt(files, key, value)

    # Act / Assert
    with pytest.raises(ValueError, match="receipt differs"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize("lane", verifier.LANES)
def test_summary_outcome_spoof_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
    lane: str,
) -> None:
    # Arrange
    checked, files = pair
    changed = dict(files)
    summary = json.loads(files[f"{lane}.summary.json"])
    summary["outcomes"]["expected"] += 1
    changed[f"{lane}.summary.json"] = (json.dumps(summary) + "\n").encode()

    # Act / Assert
    with pytest.raises(ValueError, match="summary differs"):
        verifier.verify_payloads(changed, checked)


def test_missing_bundle_member_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    checked, files = pair
    changed = dict(files)
    changed.pop("oracle.summary.json")

    # Act / Assert
    with pytest.raises(ValueError, match="bundle files"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize("lane", verifier.LANES)
def test_report_rejects_missing_terminal_attempt(lane: str) -> None:
    # Arrange
    report = _report(lane)
    report["suites"][0]["specs"][0]["tests"][0]["results"] = []

    # Act / Assert
    with pytest.raises(ValueError, match="unexecuted or malformed invocation"):
        verifier.report_rows(lane, json.dumps(report).encode(), _target_ids())


@pytest.mark.parametrize("lane", verifier.LANES)
def test_report_rejects_duplicate_spec_id(lane: str) -> None:
    # Arrange
    report = _report(lane)
    specs = report["suites"][0]["specs"]
    specs[1]["id"] = specs[0]["id"]

    # Act / Assert
    with pytest.raises(ValueError, match="wrong, missing, or duplicate invocation"):
        verifier.report_rows(lane, json.dumps(report).encode(), _target_ids())


@pytest.mark.parametrize("lane", verifier.LANES)
def test_report_rejects_fabricated_green_for_red_attempt(lane: str) -> None:
    # Arrange
    report = _report(lane)
    red = next(
        spec
        for spec in report["suites"][0]["specs"]
        if spec["tests"][0]["status"] == "unexpected"
    )
    red["tests"][0]["status"] = "expected"

    # Act / Assert
    with pytest.raises(ValueError, match="GREEN invocation is not a clean pass"):
        verifier.report_rows(lane, json.dumps(report).encode(), _target_ids())


@pytest.mark.parametrize("lane", verifier.LANES)
def test_report_rejects_red_without_error_message(lane: str) -> None:
    # Arrange
    report = _report(lane)
    red = next(
        spec
        for spec in report["suites"][0]["specs"]
        if spec["tests"][0]["status"] == "unexpected"
    )
    red["tests"][0]["results"][0]["error"] = None

    # Act / Assert
    with pytest.raises(ValueError, match="RED invocation lacks terminal failure"):
        verifier.report_rows(lane, json.dumps(report).encode(), _target_ids())


@pytest.mark.parametrize("lane", verifier.LANES)
def test_report_rejects_skipped_status(lane: str) -> None:
    # Arrange
    report = _report(lane)
    report["suites"][0]["specs"][0]["tests"][0]["status"] = "skipped"

    # Act / Assert
    with pytest.raises(ValueError, match="unexecuted or malformed invocation"):
        verifier.report_rows(lane, json.dumps(report).encode(), _target_ids())


@pytest.mark.parametrize("lane", verifier.LANES)
def test_provenance_rejects_wrong_result_sha(lane: str) -> None:
    # Arrange
    provenance = json.loads(verifier.provenance_path(lane).read_bytes())
    provenance["resultSha256"] = "0" * 64

    # Act / Assert
    with pytest.raises(ValueError, match="provenance identity"):
        verifier.verify_provenance(lane, json.dumps(provenance).encode())


@pytest.mark.parametrize("lane", verifier.LANES)
def test_provenance_rejects_wrong_tooling_pin(lane: str) -> None:
    # Arrange
    provenance = json.loads(verifier.provenance_path(lane).read_bytes())
    provenance["toolingSources"][1]["sha256"] = "0" * 64

    # Act / Assert
    with pytest.raises(ValueError, match="tooling source pins differ"):
        verifier.verify_provenance(lane, json.dumps(provenance).encode())


def test_oracle_provenance_rejects_candidate_runtime() -> None:
    # Arrange
    provenance = json.loads(verifier.provenance_path("oracle").read_bytes())
    provenance["candidateArtifactSha256"] = verifier.CANDIDATE_ARTIFACT_SHA256

    # Act / Assert
    with pytest.raises(
        ValueError, match="oracle provenance contains candidate runtime"
    ):
        verifier.verify_provenance("oracle", json.dumps(provenance).encode())


def test_manifest_sha_drift_is_rejected() -> None:
    # Arrange
    wrong_sha = "0" * 64

    # Act / Assert
    with (
        patch.object(verifier, "MANIFEST_SHA256", wrong_sha),
        pytest.raises(ValueError, match="history manifest SHA-256 changed"),
    ):
        verifier.verify_inputs()


def test_prior_receipt_sha_drift_is_rejected() -> None:
    # Arrange
    changed_pin = {"promotion-receipt.json": "0" * 64}

    # Act / Assert
    with (
        patch.dict(verifier.V13_SHA256, changed_pin),
        pytest.raises(ValueError, match="v13 promotion-receipt.json SHA-256 changed"),
    ):
        verifier.verify_inputs()


def test_candidate_provenance_live_artifact_path_drift_is_rejected(
    tmp_path: Path,
) -> None:
    # Arrange
    artifact = tmp_path / "pwright.node"
    artifact.write_bytes(b"wrong addon")
    provenance = verifier.provenance_path("candidate").read_bytes()

    # Act / Assert
    with (
        patch.object(verifier, "LIVE_ARTIFACT", artifact),
        pytest.raises(ValueError, match="candidate provenance runtime pin differs"),
    ):
        verifier.verify_provenance("candidate", provenance)


def test_frozen_candidate_manifest_sha_drift_is_rejected(tmp_path: Path) -> None:
    # Arrange
    manifest = tmp_path / "candidate-manifest.json"
    manifest.write_bytes(b"changed")

    # Act / Assert
    with (
        patch.object(verifier, "PACKAGE_MANIFEST", manifest),
        pytest.raises(ValueError, match="frozen candidate manifest SHA-256 changed"),
    ):
        verifier.verify_package()
