"""Read-only replay and mutation controls for the paired Core dialog v15 ledger."""

from __future__ import annotations

import json
from copy import deepcopy
from pathlib import Path

import pytest
import verify_playwright_core_page_dialog10_strict_paired_v15 as verifier


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
    assert len(target_indexes) == 10
    for lane in verifier.LANES:
        before = checked["oldLines"][lane]
        after = files[f"{lane}.jsonl"].splitlines(keepends=True)
        assert len(before) == len(after) == 48_477
        assert (
            sum(left != right for left, right in zip(before, after, strict=True)) == 10
        )
        assert all(
            left == right
            for index, (left, right) in enumerate(zip(before, after, strict=True))
            if index not in target_indexes
        )
    assert receipt["changedRowsPerLane"] == {"candidate": 10, "oracle": 10}
    assert receipt["unchangedRawRowsPerLane"] == {"candidate": 48_467, "oracle": 48_467}
    assert (receipt["candidateExpectedRows"], receipt["candidateUnexpectedRows"]) == (
        9,
        1,
    )
    assert (receipt["oracleExpectedRows"], receipt["oracleUnexpectedRows"]) == (10, 0)
    assert receipt["redComparison"]["candidateOnlySpecIds"] == [verifier.RED_ID]
    assert receipt["completeFileExecution"] is True
    assert receipt["fullCorpusExecution"] is False
    assert receipt["publicApiParityEvidence"] is False


@pytest.mark.parametrize("lane", verifier.LANES)
def test_off_target_raw_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], lane: str
) -> None:
    # Arrange
    checked, files = pair
    changed = _replace_row(files, lane, 0, "attempts", 99)

    # Act / Assert
    with pytest.raises(ValueError, match="off-target raw row"):
        verifier.verify_payloads(changed, checked)


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
def test_candidate_red_target_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], key: str, value: object
) -> None:
    # Arrange
    checked, files = pair
    red_index = next(
        index for index, spec_id, _ in verifier.TARGETS if spec_id == verifier.RED_ID
    )
    changed = _replace_row(files, "candidate", red_index, key, value)

    # Act / Assert
    with pytest.raises(ValueError, match="target row"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize("lane", verifier.LANES)
def test_green_target_fabricated_error_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], lane: str
) -> None:
    # Arrange
    checked, files = pair
    green_index = verifier.TARGETS[0][0]
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
        ("candidateExpectedRows", 10),
        ("oracleUnexpectedRows", 1),
        ("targetZeroBasedIndexes", []),
        ("unchangedRawRowsPerLane", {"candidate": 48_477, "oracle": 48_477}),
        ("manifestSha256", "0" * 64),
        ("redComparison", {}),
    ],
)
def test_receipt_scope_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], key: str, value: object
) -> None:
    # Arrange
    checked, files = pair
    changed = _replace_receipt(files, key, value)

    # Act / Assert
    with pytest.raises(ValueError, match="receipt differs"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize("lane", verifier.LANES)
def test_summary_outcome_spoof_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], lane: str
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
    with pytest.raises(ValueError, match="bundle files differ"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize(
    "mutation", ["missing", "duplicate", "wrong-project", "no-attempt", "fake-pass"]
)
def test_report_selection_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    report = _report("candidate")
    specs = report["suites"][0]["specs"]
    if mutation == "missing":
        specs.pop()
    elif mutation == "duplicate":
        specs[-1] = deepcopy(specs[0])
    elif mutation == "wrong-project":
        specs[0]["tests"][0]["projectId"] = "firefox-page"
    elif mutation == "no-attempt":
        specs[0]["tests"][0]["results"] = []
    else:
        red = next(spec for spec in specs if spec["id"] == verifier.RED_ID)
        red["tests"][0]["status"] = "expected"
    altered = json.dumps(report).encode()

    # Act / Assert
    with pytest.raises(ValueError, match="report"):
        verifier.report_rows("candidate", altered, _target_ids())


@pytest.mark.parametrize("lane", verifier.LANES)
@pytest.mark.parametrize(
    "field", ["manifestSha256", "resultSha256", "fixtureHost", "exitCode"]
)
def test_provenance_identity_mutation_is_rejected(lane: str, field: str) -> None:
    # Arrange
    provenance = json.loads(verifier.provenance_path(lane).read_bytes())
    provenance[field] = "wrong"

    # Act / Assert
    with pytest.raises(ValueError, match="provenance identity"):
        verifier.verify_provenance(lane, json.dumps(provenance).encode())


def test_sealed_report_digest_rejects_byte_mutation(tmp_path: Path) -> None:
    # Arrange
    altered = tmp_path / "report.json"
    altered.write_bytes(verifier.report_path("candidate").read_bytes() + b"\n")

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 changed"):
        verifier.sealed(
            altered, verifier.REPORT_SHA256["candidate"], "candidate report"
        )
