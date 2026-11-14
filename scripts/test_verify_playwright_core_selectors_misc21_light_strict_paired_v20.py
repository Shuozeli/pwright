"""Read-only replay and mutation controls for selectors-misc21 v20."""

from __future__ import annotations

import json
from copy import deepcopy
from pathlib import Path

import pytest
import verify_playwright_core_selectors_misc21_light_strict_paired_v20 as verifier


@pytest.fixture(scope="module")
def pair() -> tuple[dict[str, object], dict[str, bytes]]:
    # Arrange
    checked = verifier.verify_inputs()
    lines = list(checked["oldLines"]["candidate"])
    old = json.loads(lines[verifier.RED_INDEX])
    lines[verifier.RED_INDEX] = verifier.canonical_line(verifier.expected_red_row(old))
    candidate = b"".join(lines)
    files = {
        "candidate.jsonl": candidate,
        "candidate.summary.json": (
            json.dumps(
                verifier.expected_candidate_summary(
                    checked["oldSummaries"]["candidate"], candidate
                ),
                indent=2,
            )
            + "\n"
        ).encode(),
        "oracle.jsonl": checked["oldFiles"]["oracle.jsonl"],
        "oracle.summary.json": checked["oldFiles"]["oracle.summary.json"],
    }
    receipt = verifier.expected_receipt(files, checked)
    files["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()

    # Act
    observed = verifier.verify_payloads(files, checked)

    # Assert
    assert observed == receipt
    return checked, files


def _mutate_row(
    files: dict[str, bytes], lane: str, index: int, key: str, value: object
) -> dict[str, bytes]:
    changed = dict(files)
    lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
    row = json.loads(lines[index])
    row[key] = value
    lines[index] = verifier.canonical_line(row)
    changed[f"{lane}.jsonl"] = b"".join(lines)
    return changed


def _mutate_receipt(
    files: dict[str, bytes], key: str, value: object
) -> dict[str, bytes]:
    changed = dict(files)
    receipt = json.loads(files["promotion-receipt.json"])
    receipt[key] = value
    changed["promotion-receipt.json"] = (json.dumps(receipt) + "\n").encode()
    return changed


def _report() -> dict[str, object]:
    return json.loads(verifier.REPORT.read_bytes())


def _target_ids() -> set[str]:
    inventory = verifier.misc_source.INVENTORY.read_text().splitlines()
    return {
        json.loads(raw)["specId"]
        for raw in inventory
        if (
            json.loads(raw).get("suite") == "library"
            and json.loads(raw).get("projectId") == verifier.misc_source.PROJECT
            and json.loads(raw).get("file") == verifier.misc_source.FILE
        )
    }


def test_one_candidate_row_and_zero_oracle_bytes_change(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    checked, files = pair

    # Act
    receipt = verifier.verify_payloads(files, checked)

    # Assert
    before = checked["oldLines"]["candidate"]
    after = files["candidate.jsonl"].splitlines(keepends=True)
    assert [
        i
        for i, (left, right) in enumerate(zip(before, after, strict=True))
        if left != right
    ] == [verifier.RED_INDEX]
    assert files["oracle.jsonl"] == checked["oldFiles"]["oracle.jsonl"]
    assert files["oracle.summary.json"] == checked["oldFiles"]["oracle.summary.json"]
    assert receipt["changedRowsPerLane"] == {"candidate": 1, "oracle": 0}
    assert receipt["unchangedRawRowsPerLane"] == {"candidate": 48_476, "oracle": 48_477}
    assert receipt["candidateExpectedRows"] == 11
    assert receipt["candidateUnexpectedRows"] == 10
    assert receipt["oracleExpectedRows"] == 21
    assert receipt["oracleUnexpectedRows"] == 0
    assert receipt["newlyExpectedSpecIds"] == [verifier.RED_ID]
    assert receipt["regressedSpecIds"] == []
    assert receipt["candidateOnlyRerun"] is True
    assert receipt["fullCorpusExecution"] is False
    assert receipt["publicApiParityEvidence"] is False


@pytest.mark.parametrize("lane", verifier.LANES)
def test_off_target_raw_row_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], lane: str
) -> None:
    # Arrange
    checked, files = pair
    changed = _mutate_row(files, lane, 0, "attempts", 99)

    # Act / Assert
    with pytest.raises(ValueError, match="off-target raw row"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("attempts", 1),
        ("outcome", "unexpected"),
        ("error", "fabricated"),
        ("resultFile", "different-report"),
        ("evidenceScope", "different-scope"),
        ("publicApiParityEvidence", True),
    ],
)
def test_candidate_red_upgrade_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], key: str, value: object
) -> None:
    # Arrange
    checked, files = pair
    changed = _mutate_row(files, "candidate", verifier.RED_INDEX, key, value)

    # Act / Assert
    with pytest.raises(ValueError, match="RED target row"):
        verifier.verify_payloads(changed, checked)


def test_oracle_summary_byte_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    checked, files = pair
    changed = dict(files)
    changed["oracle.summary.json"] += b"\n"

    # Act / Assert
    with pytest.raises(ValueError, match="Oracle summary bytes changed"):
        verifier.verify_payloads(changed, checked)


def test_candidate_summary_spoof_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    checked, files = pair
    changed = dict(files)
    summary = json.loads(files["candidate.summary.json"])
    summary["outcomes"]["expected"] += 1
    changed["candidate.summary.json"] = (json.dumps(summary) + "\n").encode()

    # Act / Assert
    with pytest.raises(ValueError, match="candidate summary differs"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("fullCorpusExecution", True),
        ("publicApiParityEvidence", True),
        ("candidateOnlyRerun", False),
        ("oracleJsonlAndSummaryByteIdenticalToV19", False),
        ("candidateExpectedRows", 10),
        ("oracleUnexpectedRows", 1),
        ("redComparison", {}),
        ("changedRowsPerLane", {"candidate": 21, "oracle": 21}),
        ("regressedSpecIds", [verifier.RED_ID]),
    ],
)
def test_receipt_scope_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], key: str, value: object
) -> None:
    # Arrange
    checked, files = pair
    changed = _mutate_receipt(files, key, value)

    # Act / Assert
    with pytest.raises(ValueError, match="receipt differs"):
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
    "mutation",
    [
        "missing",
        "duplicate",
        "wrong-project",
        "no-attempt",
        "fake-pass",
        "wrong-file",
        "retry",
    ],
)
def test_report_selection_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    report = _report()
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
        specs[0]["tests"][0]["status"] = "unexpected"
    elif mutation == "wrong-file":
        specs[0]["file"] = "page/other.spec.ts"
    else:
        specs[0]["tests"][0]["results"][0]["retry"] = 1
    altered = json.dumps(report).encode()

    # Act / Assert
    with pytest.raises(ValueError, match="report|invocation"):
        verifier.report_rows(altered, _target_ids())


@pytest.mark.parametrize("mutation", ["totals", "top-level-error", "red-without-error"])
def test_report_evidence_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    report = _report()
    if mutation == "totals":
        report["stats"]["expected"] = 12
    elif mutation == "top-level-error":
        report["errors"] = [{"message": "untracked runner failure"}]
    else:
        red = next(
            spec
            for spec in report["suites"][0]["specs"]
            if spec["tests"][0]["status"] == "unexpected"
        )
        red["tests"][0]["results"][0].pop("error")

    # Act / Assert
    with pytest.raises(ValueError, match="report|RED invocation"):
        verifier.report_rows(json.dumps(report).encode(), _target_ids())


@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("resultSha256", "0" * 64),
        ("candidateArtifactSha256", "0" * 64),
        ("exitCode", 0),
        ("fixtureHost", "wrong.example"),
        ("candidateProfile", "official-conformance-page-check"),
        ("filters", ["tests/page/page-dialog.spec.ts"]),
    ],
)
def test_provenance_pin_mutation_is_rejected(key: str, value: object) -> None:
    # Arrange
    provenance = json.loads(verifier.PROVENANCE.read_bytes())
    provenance[key] = value

    # Act / Assert
    with pytest.raises(ValueError, match="provenance identity"):
        verifier.verify_new_provenance(json.dumps(provenance).encode())


def test_sealed_report_digest_rejects_byte_mutation(tmp_path: Path) -> None:
    # Arrange
    altered = tmp_path / "report.json"
    altered.write_bytes(verifier.REPORT.read_bytes() + b"\n")

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 changed"):
        verifier.sealed(altered, verifier.REPORT_SHA256, "new candidate report")


def test_untouched_green_case_regression_is_rejected(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    original = verifier.report_rows
    target_ids = _target_ids()
    rows = deepcopy(original(verifier.REPORT.read_bytes(), target_ids))
    old_green = next(
        spec_id
        for spec_id, row in rows.items()
        if spec_id != verifier.RED_ID and row["outcome"] == "expected"
    )
    old_red = next(
        spec_id
        for spec_id, row in rows.items()
        if spec_id != verifier.RED_ID and row["outcome"] == "unexpected"
    )
    rows[old_green] = {"outcome": "unexpected", "attempts": 1, "error": "regression"}
    rows[old_red] = {"outcome": "expected", "attempts": 1}
    monkeypatch.setattr(verifier, "report_rows", lambda *_: rows)

    # Act / Assert
    with pytest.raises(ValueError, match="candidate regression"):
        verifier.verify_inputs()
