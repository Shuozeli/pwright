"""Read-only replay and mutation controls for the Core selectors-misc21 v18 pair."""

from __future__ import annotations

import json
from copy import deepcopy
from pathlib import Path
from typing import Any

import promote_playwright_core_selectors_misc21_strict_paired_v18 as publisher
import pytest
import verify_playwright_core_selectors_misc21_strict_paired_v18 as verifier


@pytest.fixture(scope="module")
def pair() -> tuple[dict[str, Any], dict[str, bytes]]:
    # Arrange
    checked = verifier.verify_inputs()

    # Act
    files = publisher.prepare_pair(checked)

    # Assert
    assert verifier.verify_payloads(files, checked)["scope"] == verifier.SCOPE
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


def _report(lane: str) -> dict[str, Any]:
    return json.loads(verifier.report_path(lane).read_bytes())


def test_exact_red_successor_preserves_every_off_target_raw_row(
    pair: tuple[dict[str, Any], dict[str, bytes]],
) -> None:
    # Arrange
    checked, files = pair
    targets = set(checked["selected"])

    # Act
    receipt = verifier.verify_payloads(files, checked)

    # Assert
    assert len(targets) == 21
    for lane in verifier.LANES:
        before = checked["oldLines"][lane]
        after = files[f"{lane}.jsonl"].splitlines(keepends=True)
        assert len(before) == len(after) == 48_477
        assert (
            sum(left != right for left, right in zip(before, after, strict=True)) == 21
        )
        assert all(
            left == right
            for index, (left, right) in enumerate(zip(before, after, strict=True))
            if index not in targets
        )
    assert (
        receipt["priorBundleReceiptSha256"]
        == verifier.V17_SHA256["promotion-receipt.json"]
    )
    assert receipt["changedRowsPerLane"] == {"candidate": 21, "oracle": 21}
    assert receipt["unchangedRawRowsPerLane"] == {
        "candidate": 48_456,
        "oracle": 48_456,
    }
    assert (receipt["candidateExpectedRows"], receipt["candidateUnexpectedRows"]) == (
        10,
        11,
    )
    assert (receipt["oracleExpectedRows"], receipt["oracleUnexpectedRows"]) == (21, 0)
    assert receipt["redComparison"]["candidateOnlyCount"] == 11
    assert receipt["redComparison"]["bothRedCount"] == 0
    assert receipt["redComparison"]["oracleOnlyCount"] == 0
    assert receipt["completeFileExecution"] is True
    assert receipt["fullCorpusExecution"] is False
    assert receipt["publicApiParityEvidence"] is False


@pytest.mark.parametrize("lane", verifier.LANES)
def test_off_target_raw_mutation_is_rejected(
    pair: tuple[dict[str, Any], dict[str, bytes]], lane: str
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
    pair: tuple[dict[str, Any], dict[str, bytes]], key: str, value: object
) -> None:
    # Arrange
    checked, files = pair
    red_index = next(
        index
        for index, row in checked["replacementRows"]["candidate"].items()
        if row["outcome"] == "unexpected"
    )
    changed = _replace_row(files, "candidate", red_index, key, value)

    # Act / Assert
    with pytest.raises(ValueError, match="target row"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize("lane", verifier.LANES)
def test_green_target_fabricated_error_is_rejected(
    pair: tuple[dict[str, Any], dict[str, bytes]], lane: str
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
        ("candidateExpectedRows", 21),
        ("oracleUnexpectedRows", 1),
        ("targetZeroBasedIndexes", []),
        ("unchangedRawRowsPerLane", {"candidate": 48_477, "oracle": 48_477}),
        ("manifestSha256", "0" * 64),
        ("redComparison", {}),
        ("priorBundleReceiptSha256", "0" * 64),
    ],
)
def test_receipt_scope_mutation_is_rejected(
    pair: tuple[dict[str, Any], dict[str, bytes]], key: str, value: object
) -> None:
    # Arrange
    checked, files = pair
    changed = _replace_receipt(files, key, value)

    # Act / Assert
    with pytest.raises(ValueError, match="receipt differs"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize("lane", verifier.LANES)
def test_summary_outcome_spoof_is_rejected(
    pair: tuple[dict[str, Any], dict[str, bytes]], lane: str
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
    pair: tuple[dict[str, Any], dict[str, bytes]],
) -> None:
    # Arrange
    checked, files = pair
    changed = dict(files)
    changed.pop("oracle.summary.json")

    # Act / Assert
    with pytest.raises(ValueError, match="bundle files differ"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize(
    "mutation",
    [
        "missing",
        "duplicate",
        "wrong-project",
        "no-attempt",
        "fake-pass",
        "top-level-error",
        "retry",
        "flaky",
    ],
)
def test_report_selection_mutation_is_rejected(
    pair: tuple[dict[str, Any], dict[str, bytes]], mutation: str
) -> None:
    # Arrange
    checked, _ = pair
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
    elif mutation == "fake-pass":
        red = next(spec for spec in specs if spec["tests"][0]["status"] == "unexpected")
        red["tests"][0]["status"] = "expected"
    elif mutation == "top-level-error":
        report["errors"] = [{"message": "synthetic setup error"}]
    elif mutation == "retry":
        specs[0]["tests"][0]["results"][0]["retry"] = 1
    else:
        report["stats"]["flaky"] = 1
    altered = json.dumps(report).encode()
    ids = {row["specId"] for row in checked["selected"].values()}

    # Act / Assert
    with pytest.raises(ValueError, match="report|invocation"):
        verifier.report_rows("candidate", altered, ids)


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


def test_publisher_refuses_existing_destination(
    pair: tuple[dict[str, Any], dict[str, bytes]],
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    _, files = pair
    occupied = tmp_path / "occupied-v18"
    occupied.mkdir()
    monkeypatch.setattr(verifier, "V18", occupied)

    # Act / Assert
    with pytest.raises(ValueError, match="already exists"):
        publisher.publish(files)
