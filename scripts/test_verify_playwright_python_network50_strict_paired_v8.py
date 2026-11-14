"""Mutation controls for the independent Python network50 v8 paired verifier."""

from __future__ import annotations

import json
import xml.etree.ElementTree as ET
from pathlib import Path

import pytest
import verify_playwright_python_network50_strict_paired_v8 as verify


def _line(row: dict[str, object]) -> bytes:
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _ledger(
    monkeypatch: pytest.MonkeyPatch, mode: str
) -> tuple[
    list[bytes], list[bytes], list[dict[str, object]], dict[str, dict[str, object]]
]:
    inventory: list[dict[str, object]] = [
        {
            "id": "tests/async/test_other.py::before[chromium]",
            "sourceClassification": "public-api-source",
        },
        {
            "id": "tests/async/test_network.py::first[chromium]",
            "sourceClassification": "public-api-source",
        },
        {
            "id": "tests/async/test_network.py::second[chromium]",
            "sourceClassification": "public-api-source",
        },
        {
            "id": "tests/async/test_other.py::after[chromium]",
            "sourceClassification": "public-api-source",
        },
    ]
    monkeypatch.setattr(verify, "RECORDS", 4)
    monkeypatch.setattr(verify, "TARGET_INDEXES", (1, 2))
    monkeypatch.setattr(
        verify,
        "EXPECTED_SUCCESSOR_COUNTS",
        {
            "candidate": {"passed": 1, "failed": 2, "not-run": 1},
            "oracle": {"passed": 2, "failed": 1, "not-run": 1},
        },
    )
    cases: dict[str, dict[str, object]] = {
        str(inventory[1]["id"]): {
            "outcome": "failed",
            "durationSeconds": 0.5,
            "error": "shared fixture failure",
        },
        str(inventory[2]["id"]): (
            {
                "outcome": "failed",
                "durationSeconds": 0.75,
                "error": "candidate-only failure",
            }
            if mode == "candidate"
            else {"outcome": "passed", "durationSeconds": 0.75}
        ),
    }
    base = [
        _line({**inventory[0], "outcome": "passed", "attempts": 1}),
        _line({**inventory[1], "outcome": "not-run", "attempts": 0}),
        _line({**inventory[2], "outcome": "not-run", "attempts": 0}),
        _line({**inventory[3], "outcome": "not-run", "attempts": 0}),
    ]
    successor = list(base)
    for index in (1, 2):
        case = cases[str(inventory[index]["id"])]
        successor[index] = _line(
            {
                **inventory[index],
                **case,
                "attempts": 1,
                "resultFile": str(verify.REPORTS[mode]),
            }
        )
    return base, successor, inventory, cases


@pytest.mark.parametrize("mode", ("candidate", "oracle"))
def test_exactly_two_target_rows_are_replaced(
    monkeypatch: pytest.MonkeyPatch, mode: str
) -> None:
    # Arrange
    base, successor, inventory, cases = _ledger(monkeypatch, mode)

    # Act
    counts = verify.check_reconciliation(base, successor, inventory, cases, mode)

    # Assert
    assert counts == verify.EXPECTED_SUCCESSOR_COUNTS[mode]
    assert successor[0] == base[0] and successor[3] == base[3]
    assert successor[1] != base[1] and successor[2] != base[2]


@pytest.mark.parametrize("mode", ("candidate", "oracle"))
@pytest.mark.parametrize(
    "mutation",
    (
        "off-target-byte",
        "target-byte",
        "status",
        "diagnostic",
        "duration",
        "attempts",
        "result-path",
        "missing-target",
        "base-id",
        "source-classification",
        "missing-report-id",
    ),
)
def test_ledger_mutation_is_rejected(
    monkeypatch: pytest.MonkeyPatch, mode: str, mutation: str
) -> None:
    # Arrange
    base, successor, inventory, cases = _ledger(monkeypatch, mode)
    target = json.loads(successor[1])
    if mutation == "off-target-byte":
        successor[3] = b" " + successor[3]
    elif mutation == "target-byte":
        successor[1] = b" " + successor[1]
    elif mutation == "status":
        target["outcome"] = "passed"
        target.pop("error")
        successor[1] = _line(target)
    elif mutation == "diagnostic":
        target["error"] = "invented diagnostic"
        successor[1] = _line(target)
    elif mutation == "duration":
        target["durationSeconds"] = 0.6
        successor[1] = _line(target)
    elif mutation == "attempts":
        target["attempts"] = 2
        successor[1] = _line(target)
    elif mutation == "result-path":
        target["resultFile"] = "/wrong.xml"
        successor[1] = _line(target)
    elif mutation == "missing-target":
        successor[1] = base[1]
    elif mutation == "base-id":
        before = json.loads(base[1])
        before["id"] = "moved"
        base[1] = _line(before)
    elif mutation == "source-classification":
        target["sourceClassification"] = "private-api-source"
        successor[1] = _line(target)
    elif mutation == "missing-report-id":
        cases.pop(str(inventory[1]["id"]))

    # Act / Assert
    with pytest.raises(verify.Network50ValidationError):
        verify.check_reconciliation(base, successor, inventory, cases, mode)


def _paired_cases() -> tuple[dict[str, dict[str, dict[str, object]]], list[str]]:
    ids = [
        f"tests/async/test_network.py::case_{index}[chromium]" for index in range(50)
    ]
    cases = {mode: {} for mode in verify.MODES}
    for index, test_id in enumerate(ids):
        cases["candidate"][test_id] = {"outcome": "failed" if index < 15 else "passed"}
        cases["oracle"][test_id] = {"outcome": "failed" if index < 3 else "passed"}
    return cases, ids


def test_comparison_separates_shared_and_candidate_only_red() -> None:
    # Arrange
    cases, ids = _paired_cases()

    # Act
    comparison = verify.compare_cases(cases, ids)

    # Assert
    assert len(comparison["shared"]) == 3
    assert len(comparison["candidateOnly"]) == 12
    assert comparison["oracleOnly"] == []


@pytest.mark.parametrize(
    "mutation", ("missing-id", "extra-shared", "oracle-only", "candidate-giant-green")
)
def test_comparison_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    cases, ids = _paired_cases()
    if mutation == "missing-id":
        cases["oracle"].pop(ids[0])
    elif mutation == "extra-shared":
        cases["oracle"][ids[3]]["outcome"] = "failed"
    elif mutation == "oracle-only":
        cases["candidate"][ids[0]]["outcome"] = "passed"
    elif mutation == "candidate-giant-green":
        cases["candidate"][ids[3]]["outcome"] = "passed"

    # Act / Assert
    with pytest.raises(verify.Network50ValidationError):
        verify.compare_cases(cases, ids)


def _junit() -> tuple[bytes, list[str]]:
    ids = [
        f"tests/async/test_network.py::case_{index}[chromium]" for index in range(50)
    ]
    suite = ET.Element("testsuite")
    for index, test_id in enumerate(ids):
        case = ET.SubElement(
            suite,
            "testcase",
            classname="tests.async.test_network",
            name=test_id.split("::", 1)[1],
            time="0.25",
        )
        if index == 3:
            ET.SubElement(case, "failure", message="real assertion failed")
    return ET.tostring(suite), ids


def test_junit_keeps_official_failure_red(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    raw, ids = _junit()
    monkeypatch.setattr(
        verify,
        "EXPECTED_REPORT_COUNTS",
        {
            "candidate": {"passed": 49, "failed": 1},
            "oracle": {"passed": 49, "failed": 1},
        },
    )

    # Act
    cases = verify._junit_cases(raw, ids, "candidate")

    # Assert
    assert len(cases) == 50
    assert cases[ids[3]] == {
        "outcome": "failed",
        "durationSeconds": 0.25,
        "error": "real assertion failed",
    }


@pytest.mark.parametrize(
    "mutation",
    (
        "missing",
        "duplicate",
        "reorder",
        "outside",
        "skip",
        "error",
        "nan",
        "empty-failure",
        "wrong-counts",
    ),
)
def test_junit_mutation_is_rejected(
    monkeypatch: pytest.MonkeyPatch, mutation: str
) -> None:
    # Arrange
    raw, ids = _junit()
    monkeypatch.setattr(
        verify,
        "EXPECTED_REPORT_COUNTS",
        {
            "candidate": {"passed": 49, "failed": 1},
            "oracle": {"passed": 49, "failed": 1},
        },
    )
    suite = ET.fromstring(raw)
    cases = suite.findall("testcase")
    if mutation == "missing":
        suite.remove(cases[0])
    elif mutation == "duplicate":
        cases[0].set("name", cases[1].get("name", ""))
    elif mutation == "reorder":
        suite.remove(cases[0])
        suite.append(cases[0])
    elif mutation == "outside":
        cases[0].set("classname", "tests.async.test_other")
    elif mutation == "skip":
        ET.SubElement(cases[0], "skipped")
    elif mutation == "error":
        ET.SubElement(cases[0], "error", message="synthetic error")
    elif mutation == "nan":
        cases[0].set("time", "nan")
    elif mutation == "empty-failure":
        cases[3].find("failure").set("message", "")  # type: ignore[union-attr]
    elif mutation == "wrong-counts":
        ET.SubElement(cases[0], "failure", message="extra failure")

    # Act / Assert
    with pytest.raises(verify.Network50ValidationError):
        verify._junit_cases(ET.tostring(suite), ids, "candidate")


def _receipt_scope() -> dict[str, object]:
    return {
        "schema": 2,
        "kind": verify.KIND,
        "scope": verify.SCOPE,
        "publicApiParityEvidence": False,
        "fullCorpusExecution": False,
        "completeFileExecution": False,
        "completeManifestExecution": True,
        "fixtureAdapted": True,
        "sharedFailedCases": 3,
        "candidateOnlyFailedCases": 12,
        "oracleOnlyFailedCases": 0,
        "launchMode": "cdp-adapter",
    }


def test_bounded_manifest_scope_is_accepted() -> None:
    # Arrange
    receipt = _receipt_scope()

    # Act
    result = verify.check_receipt_scope(receipt)

    # Assert
    assert result is None


@pytest.mark.parametrize(
    ("field", "value"),
    (
        ("publicApiParityEvidence", True),
        ("fullCorpusExecution", True),
        ("completeFileExecution", True),
        ("completeManifestExecution", False),
        ("sharedFailedCases", 2),
        ("candidateOnlyFailedCases", 11),
        ("oracleOnlyFailedCases", 1),
    ),
)
def test_scope_false_claim_is_rejected(field: str, value: object) -> None:
    # Arrange
    receipt = {**_receipt_scope(), field: value}

    # Act / Assert
    with pytest.raises(verify.Network50ValidationError):
        verify.check_receipt_scope(receipt)


@pytest.mark.parametrize("mode", ("candidate", "oracle"))
def test_exact_report_metadata_is_accepted(mode: str) -> None:
    # Arrange
    payload = json.loads(
        verify.REPORTS[mode].with_suffix(".provenance.json").read_bytes()
    )
    inventory = [
        json.loads(line) for line in verify.INVENTORY.read_bytes().splitlines()
    ]
    targets = [inventory[index]["id"] for index in verify.TARGET_INDEXES]

    # Act
    result = verify.check_report_metadata(payload, mode, targets)

    # Assert
    assert result is None


@pytest.mark.parametrize("mode", ("candidate", "oracle"))
@pytest.mark.parametrize(
    ("field", "value"),
    (
        ("scope", "selected"),
        ("manifestCases", False),
        ("browsers", ["firefox"]),
        ("resultSha256", "0" * 64),
        ("caseReportsSha256", "0" * 64),
        ("isolatedCaseCount", 49),
        ("fixtureHost", "wrong"),
        ("exitCode", 0),
    ),
)
def test_report_metadata_mutation_is_rejected(
    mode: str, field: str, value: object
) -> None:
    # Arrange
    payload = json.loads(
        verify.REPORTS[mode].with_suffix(".provenance.json").read_bytes()
    )
    inventory = [
        json.loads(line) for line in verify.INVENTORY.read_bytes().splitlines()
    ]
    targets = [inventory[index]["id"] for index in verify.TARGET_INDEXES]
    payload[field] = value

    # Act / Assert
    with pytest.raises(verify.Network50ValidationError):
        verify.check_report_metadata(payload, mode, targets)


def test_unsealed_pin_is_rejected(tmp_path: Path) -> None:
    # Arrange
    report = tmp_path / "report.xml"
    report.write_bytes(b"<testsuite/>")

    # Act / Assert
    with pytest.raises(verify.Network50ValidationError):
        verify.sealed(report, "0" * 64, "report")
