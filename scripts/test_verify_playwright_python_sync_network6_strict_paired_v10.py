"""Mutation controls for the Python sync network six-case strict verifier."""

from __future__ import annotations

import json
import xml.etree.ElementTree as ET
from pathlib import Path

import pytest
import verify_playwright_python_sync_network6_strict_paired_v10 as verify


def _line(row: dict[str, object]) -> bytes:
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _cases(mode: str) -> dict[str, dict[str, object]]:
    names = (
        "test_response_security_details",
        "test_response_security_details_none_without_https",
        "test_response_server_addr",
        "test_should_fulfill_with_global_fetch_result",
        "test_should_report_if_response_was_from_service_worker",
        "test_should_report_service_worker_request",
    )
    failed = set(verify.SHARED_RED)
    if mode == "candidate":
        failed |= set(verify.CANDIDATE_ONLY_RED)
    return {
        f"tests/sync/test_network.py::{name}[chromium]": {
            "outcome": "failed" if name in failed else "passed",
            "durationSeconds": 0.1,
            **({"error": f"exact-{name}"} if name in failed else {}),
        }
        for name in names
    }


def _ledger(
    monkeypatch: pytest.MonkeyPatch, mode: str
) -> tuple[list[bytes], list[bytes], list[dict[str, object]], dict[str, dict[str, object]]]:
    cases = _cases(mode)
    inventory: list[dict[str, object]] = [
        {"id": "tests/sync/test_other.py::before[chromium]", "sourceClassification": "public-api-source"},
        *({"id": test_id, "sourceClassification": "public-api-source"} for test_id in cases),
        {"id": "tests/sync/test_other.py::after[chromium]", "sourceClassification": "public-api-source"},
    ]
    monkeypatch.setattr(verify, "RECORDS", 8)
    monkeypatch.setattr(verify, "TARGET_INDEXES", tuple(range(1, 7)))
    monkeypatch.setattr(
        verify,
        "EXPECTED_SUCCESSOR_COUNTS",
        {"candidate": {"failed": 5, "passed": 1, "not-run": 2}, "oracle": {"failed": 3, "passed": 3, "not-run": 2}},
    )
    base = [_line({**source, "outcome": "not-run", "attempts": 0}) for source in inventory]
    successor = list(base)
    for index in verify.TARGET_INDEXES:
        source = inventory[index]
        case = cases[str(source["id"])]
        successor[index] = _line({**source, **case, "attempts": 1, "resultFile": str(verify.REPORTS[f"{mode}Qualified"])})
    return base, successor, inventory, cases


@pytest.mark.parametrize("mode", verify.MODES)
def test_only_six_target_rows_are_promoted(monkeypatch: pytest.MonkeyPatch, mode: str) -> None:
    # Arrange
    base, successor, inventory, cases = _ledger(monkeypatch, mode)

    # Act
    counts = verify.check_reconciliation(base, successor, inventory, cases, mode)

    # Assert
    assert counts == verify.EXPECTED_SUCCESSOR_COUNTS[mode]
    assert successor[0] == base[0] and successor[7] == base[7]
    assert all(json.loads(successor[index])["attempts"] == 1 for index in range(1, 7))


@pytest.mark.parametrize("mode", verify.MODES)
@pytest.mark.parametrize(
    "mutation",
    (
        "off-target-byte",
        "target-byte",
        "outcome",
        "error",
        "duration",
        "attempts",
        "result-path",
        "unreplaced",
        "base-attempts",
        "metadata",
        "missing-case",
        "extra-case",
    ),
)
def test_ledger_mutation_is_rejected(
    monkeypatch: pytest.MonkeyPatch, mode: str, mutation: str
) -> None:
    # Arrange
    base, successor, inventory, cases = _ledger(monkeypatch, mode)
    target = json.loads(successor[1])
    if mutation == "off-target-byte":
        successor[7] = b" " + successor[7]
    elif mutation == "target-byte":
        successor[1] = b" " + successor[1]
    elif mutation == "outcome":
        if target["outcome"] == "passed":
            target["outcome"] = "failed"
            target["error"] = "invented failure"
        else:
            target["outcome"] = "passed"
            target.pop("error", None)
        successor[1] = _line(target)
    elif mutation == "error":
        target["error"] = "edited diagnostic"
        successor[1] = _line(target)
    elif mutation == "duration":
        target["durationSeconds"] = 0.2
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
        old = json.loads(base[1])
        old["attempts"] = 1
        base[1] = _line(old)
    elif mutation == "metadata":
        target["sourceClassification"] = "private-api-source"
        successor[1] = _line(target)
    elif mutation == "missing-case":
        cases.pop(str(inventory[1]["id"]))
    elif mutation == "extra-case":
        cases["tests/sync/test_network.py::extra[chromium]"] = {"outcome": "passed", "durationSeconds": 0.1}

    # Act / Assert
    with pytest.raises(verify.SyncNetwork6ValidationError):
        verify.check_reconciliation(base, successor, inventory, cases, mode)


def _report_cases(key: str) -> dict[str, dict[str, object]]:
    raw = verify.REPORTS[key].read_bytes()
    selections = [
        f"tests/sync/test_network.py::{name}"
        for name in (
            "test_response_server_addr",
            "test_response_security_details",
            "test_response_security_details_none_without_https",
            "test_should_fulfill_with_global_fetch_result",
            "test_should_report_if_response_was_from_service_worker",
            "test_should_report_service_worker_request",
        )
    ]
    return verify._junit_cases(raw, key, selections)


@pytest.mark.parametrize("key", tuple(verify.REPORTS))
def test_sealed_junit_has_exact_six_cases(key: str) -> None:
    # Arrange / Act
    cases = _report_cases(key)

    # Assert
    assert len(cases) == 6
    assert {test_id.rsplit("[", 1)[-1] for test_id in cases} == {"chromium]"}
    assert dict(__import__("collections").Counter(case["outcome"] for case in cases.values())) == verify.EXPECTED_COUNTS[key]


@pytest.mark.parametrize(
    "mutation",
    ("duplicate", "missing", "wrong-browser", "wrong-class", "bad-duration", "missing-message", "wrong-terminal", "wrong-summary", "wrong-order"),
)
def test_junit_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    key = "candidateQualified"
    root = ET.fromstring(verify.REPORTS[key].read_bytes())
    suite = next(root.iter("testsuite"))
    cases = list(root.iter("testcase"))
    selections = [f"tests/sync/test_network.py::{node.get('name').removesuffix('[chromium]')}" for node in cases]
    if mutation == "duplicate":
        cases[1].set("name", cases[0].get("name", ""))
    elif mutation == "missing":
        suite.remove(cases[0])
    elif mutation == "wrong-browser":
        cases[0].set("name", "test_response_server_addr[firefox]")
    elif mutation == "wrong-class":
        cases[0].set("classname", "tests.sync.test_other")
    elif mutation == "bad-duration":
        cases[0].set("time", "nan")
    elif mutation == "missing-message":
        cases[0].find("failure").attrib.pop("message")  # type: ignore[union-attr]
    elif mutation == "wrong-terminal":
        cases[0].find("failure").tag = "error"  # type: ignore[union-attr]
    elif mutation == "wrong-summary":
        suite.set("failures", "4")
    elif mutation == "wrong-order":
        first = cases[0].get("name", "")
        cases[0].set("name", cases[1].get("name", ""))
        cases[1].set("name", first)

    # Act / Assert
    with pytest.raises(verify.SyncNetwork6ValidationError):
        verify._junit_cases(ET.tostring(root), key, selections)


def test_infrastructure_invalid_run_is_not_promoted() -> None:
    # Arrange
    invalid = _report_cases("candidateInfrastructureInvalid")
    qualified = _report_cases("candidateQualified")

    # Act / Assert
    assert all(case["outcome"] == "error" for case in invalid.values())
    assert all("Page-lane limit of 8" in str(case["error"]) for case in invalid.values())
    assert all(case["outcome"] != "error" for case in qualified.values())
    assert verify.report_receipt("candidateInfrastructureInvalid")["promoted"] is False


@pytest.mark.parametrize("mutation", ("candidate-only-green", "shared-oracle-green", "missing-id"))
def test_paired_classification_rejects_status_mutation(mutation: str) -> None:
    # Arrange
    results = {
        "candidateQualified": _report_cases("candidateQualified"),
        "oracleQualified": _report_cases("oracleQualified"),
    }
    if mutation == "candidate-only-green":
        results["candidateQualified"][f"tests/sync/test_network.py::{verify.CANDIDATE_ONLY_RED[0]}[chromium]"]["outcome"] = "passed"
    elif mutation == "shared-oracle-green":
        results["oracleQualified"][f"tests/sync/test_network.py::{verify.SHARED_RED[0]}[chromium]"]["outcome"] = "passed"
    else:
        results["candidateQualified"].pop(f"tests/sync/test_network.py::{verify.SHARED_GREEN[0]}[chromium]")

    # Act / Assert
    with pytest.raises(verify.SyncNetwork6ValidationError):
        verify.paired_classification(results)


def test_archive_fingerprint_detects_content_and_extra_directory(tmp_path: Path) -> None:
    # Arrange
    for relative in ("pwright", "shuozeli_pwright.dylibs", "harness/shim", "harness/runtime"):
        (tmp_path / relative).mkdir(parents=True)
        (tmp_path / relative / "one.txt").write_text(relative)
    expected = verify.archive_fingerprint(tmp_path)

    # Act / Assert
    (tmp_path / "pwright" / "one.txt").write_text("changed")
    assert verify.archive_fingerprint(tmp_path) != expected
    (tmp_path / "pwright" / "one.txt").write_text("pwright")
    (tmp_path / "pwright" / "extra").mkdir()
    assert verify.archive_fingerprint(tmp_path) != expected


def test_receipt_scope_never_claims_full_parity() -> None:
    # Arrange
    classification = verify.paired_classification({"candidateQualified": _report_cases("candidateQualified"), "oracleQualified": _report_cases("oracleQualified")})

    # Act
    receipt = verify.receipt_expected({}, classification, verify.SOURCE_PINS_SHA256)

    # Assert
    assert receipt["publicApiParityEvidence"] is False
    assert receipt["fullCorpusExecution"] is False
    assert receipt["candidateInfrastructureInvalidRuns"] == 1
    assert receipt["candidateQualifiedAttemptsPerTarget"] == 1
    assert receipt["pairedClassification"]["candidateOnlyRed"]
