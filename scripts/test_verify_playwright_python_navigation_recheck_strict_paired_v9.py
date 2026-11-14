"""Mutation controls for the independent ABI131 navigation80 recheck verifier."""

from __future__ import annotations

import json
import xml.etree.ElementTree as ET
from pathlib import Path

import pytest
import verify_playwright_python_navigation_recheck_strict_paired_v9 as verify


def _line(row: dict[str, object]) -> bytes:
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _ledger(
    monkeypatch: pytest.MonkeyPatch,
) -> tuple[
    list[bytes], list[bytes], list[dict[str, object]], dict[str, dict[str, object]]
]:
    inventory: list[dict[str, object]] = [
        {
            "id": "tests/async/test_other.py::before[chromium]",
            "sourceClassification": "public-api-source",
        },
        {
            "id": verify.HISTORICAL_SYNTHETIC_ERROR_IDS[0],
            "sourceClassification": "public-api-source",
        },
        {
            "id": verify.HISTORICAL_SYNTHETIC_ERROR_IDS[1],
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
            "candidate": {"passed": 3, "not-run": 1},
            "oracle": {"passed": 3, "not-run": 1},
        },
    )
    cases = {
        str(inventory[1]["id"]): {"outcome": "passed", "durationSeconds": 0.25},
        str(inventory[2]["id"]): {"outcome": "passed", "durationSeconds": 0.5},
    }
    base = [
        _line({**inventory[0], "outcome": "passed", "attempts": 1}),
        _line(
            {
                **inventory[1],
                "outcome": "error",
                "attempts": 1,
                "durationSeconds": 0.0,
                "resultFile": "/evidence/old.xml",
                "error": "old duplicate JUnit",
            }
        ),
        _line(
            {
                **inventory[2],
                "outcome": "error",
                "attempts": 1,
                "durationSeconds": 0.0,
                "resultFile": "/evidence/old.xml",
                "error": "old duplicate JUnit",
            }
        ),
        _line({**inventory[3], "outcome": "not-run", "attempts": 0}),
    ]
    successor = list(base)
    for index in (1, 2):
        successor[index] = _line(
            {
                **inventory[index],
                **cases[str(inventory[index]["id"])],
                "attempts": 1,
                "resultFile": str(verify.RESULT),
            }
        )
    return base, successor, inventory, cases


def test_candidate_replaces_two_historical_error_rows(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    base, successor, inventory, cases = _ledger(monkeypatch)

    # Act
    counts = verify.check_reconciliation(base, successor, inventory, cases, "candidate")

    # Assert
    assert counts == {"passed": 3, "not-run": 1}
    assert successor[0] == base[0] and successor[3] == base[3]
    assert all(json.loads(successor[index])["outcome"] == "passed" for index in (1, 2))


@pytest.mark.parametrize(
    "mutation",
    (
        "off-target-byte",
        "target-byte",
        "reclassify",
        "extra-error",
        "duration",
        "attempts",
        "result-path",
        "unreplaced",
        "base-attempts",
        "source-classification",
        "missing-report-id",
    ),
)
def test_candidate_mutation_is_rejected(
    monkeypatch: pytest.MonkeyPatch, mutation: str
) -> None:
    # Arrange
    base, successor, inventory, cases = _ledger(monkeypatch)
    target = json.loads(successor[1])
    if mutation == "off-target-byte":
        successor[3] = b" " + successor[3]
    elif mutation == "target-byte":
        successor[1] = b" " + successor[1]
    elif mutation == "reclassify":
        target["outcome"] = "failed"
        target["error"] = "invented failure"
        successor[1] = _line(target)
    elif mutation == "extra-error":
        target["error"] = "stale error"
        successor[1] = _line(target)
    elif mutation == "duration":
        target["durationSeconds"] = 0.3
        successor[1] = _line(target)
    elif mutation == "attempts":
        target["attempts"] = 2
        successor[1] = _line(target)
    elif mutation == "result-path":
        target["resultFile"] = "/wrong.xml"
        successor[1] = _line(target)
    elif mutation == "unreplaced":
        successor[1] = base[1]
    elif mutation == "base-attempts":
        before = json.loads(base[1])
        before["attempts"] = 0
        base[1] = _line(before)
    elif mutation == "source-classification":
        target["sourceClassification"] = "private-api-source"
        successor[1] = _line(target)
    elif mutation == "missing-report-id":
        cases.pop(str(inventory[1]["id"]))

    # Act / Assert
    with pytest.raises(verify.NavigationRecheckValidationError):
        verify.check_reconciliation(base, successor, inventory, cases, "candidate")


@pytest.mark.parametrize("index", (0, 1, 2, 3))
def test_oracle_raw_row_mutation_is_rejected(
    monkeypatch: pytest.MonkeyPatch, index: int
) -> None:
    # Arrange
    _, _, inventory, cases = _ledger(monkeypatch)
    base = [
        _line({**source, "outcome": "passed", "attempts": 1})
        if row_index < 3
        else _line({**source, "outcome": "not-run", "attempts": 0})
        for row_index, source in enumerate(inventory)
    ]
    successor = list(base)
    successor[index] = b" " + successor[index]

    # Act / Assert
    with pytest.raises(verify.NavigationRecheckValidationError):
        verify.check_reconciliation(base, successor, inventory, cases, "oracle")


def _scope() -> dict[str, object]:
    return {
        "schema": 2,
        "kind": verify.KIND,
        "scope": verify.SCOPE,
        "publicApiParityEvidence": False,
        "fullCorpusExecution": False,
        "completeFileExecution": True,
        "fixtureAdapted": True,
        "sameWheelAbi131Recheck": True,
        "historicalSyntheticErrorsNotReproduced": True,
        "failureNonReproductionDoesNotProveAbsence": True,
        "rootCauseEstablished": False,
        "launchMode": "cdp-adapter",
    }


def test_bounded_recheck_scope_is_accepted() -> None:
    # Arrange
    receipt = _scope()

    # Act
    result = verify.check_receipt_scope(receipt)

    # Assert
    assert result is None


@pytest.mark.parametrize(
    ("field", "value"),
    (
        ("publicApiParityEvidence", True),
        ("fullCorpusExecution", True),
        ("completeFileExecution", False),
        ("sameWheelAbi131Recheck", False),
        ("historicalSyntheticErrorsNotReproduced", False),
        ("failureNonReproductionDoesNotProveAbsence", False),
        ("rootCauseEstablished", True),
    ),
)
def test_scope_false_claim_is_rejected(field: str, value: object) -> None:
    # Arrange
    receipt = {**_scope(), field: value}

    # Act / Assert
    with pytest.raises(verify.NavigationRecheckValidationError):
        verify.check_receipt_scope(receipt)


def _junit() -> tuple[bytes, list[str]]:
    ids = [
        verify.HISTORICAL_SYNTHETIC_ERROR_IDS[0]
        if index == 5
        else verify.HISTORICAL_SYNTHETIC_ERROR_IDS[1]
        if index == 70
        else f"tests/async/test_navigation.py::case_{index}[chromium]"
        for index in range(80)
    ]
    suite = ET.Element("testsuite")
    for index, test_id in enumerate(ids):
        case = ET.SubElement(
            suite,
            "testcase",
            classname="tests.async.test_navigation",
            name=test_id.split("::", 1)[1],
            time="0.25",
        )
        if index in {7, 35}:
            ET.SubElement(case, "failure", message="known shared failure")
    return ET.tostring(suite), ids


def test_junit_recheck_has_no_synthetic_errors() -> None:
    # Arrange
    raw, ids = _junit()

    # Act
    cases = verify._junit_cases(raw, ids)

    # Assert
    assert len(cases) == 80
    assert all(
        cases[test_id]["outcome"] == "passed"
        for test_id in verify.HISTORICAL_SYNTHETIC_ERROR_IDS
    )
    assert sum(case["outcome"] == "failed" for case in cases.values()) == 2


@pytest.mark.parametrize(
    "mutation",
    (
        "missing",
        "duplicate",
        "reorder",
        "outside",
        "skip",
        "error",
        "old-error",
        "nan",
        "empty-failure",
        "wrong-counts",
    ),
)
def test_junit_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    raw, ids = _junit()
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
    elif mutation == "old-error":
        ET.SubElement(cases[5], "error", message="old duplicate report")
    elif mutation == "nan":
        cases[0].set("time", "nan")
    elif mutation == "empty-failure":
        cases[7].find("failure").set("message", "")  # type: ignore[union-attr]
    elif mutation == "wrong-counts":
        ET.SubElement(cases[0], "failure", message="extra failure")

    # Act / Assert
    with pytest.raises(verify.NavigationRecheckValidationError):
        verify._junit_cases(ET.tostring(suite), ids)


def test_exact_recheck_metadata_is_accepted() -> None:
    # Arrange
    payload = json.loads(verify.PROVENANCE.read_bytes())

    # Act
    result = verify.check_report_metadata(payload)

    # Assert
    assert result is None


@pytest.mark.parametrize(
    ("field", "value"),
    (
        ("mode", "oracle"),
        ("manifestSha256", "0" * 64),
        ("fixtureHost", "wrong"),
        ("resultSha256", "0" * 64),
        ("caseReportsSha256", "0" * 64),
        ("isolatedCaseCount", 79),
        ("pwrightArtifactSha256", "0" * 64),
        ("exitCode", 0),
    ),
)
def test_recheck_metadata_mutation_is_rejected(field: str, value: object) -> None:
    # Arrange
    payload = json.loads(verify.PROVENANCE.read_bytes())
    payload[field] = value

    # Act / Assert
    with pytest.raises(verify.NavigationRecheckValidationError):
        verify.check_report_metadata(payload)


def test_unsealed_source_pin_is_rejected(tmp_path: Path) -> None:
    # Arrange
    path = tmp_path / "source-pins.json"
    path.write_bytes(b"{}\n")

    # Act / Assert
    with pytest.raises(verify.NavigationRecheckValidationError):
        verify.sealed(path, "0" * 64, "source pins")
