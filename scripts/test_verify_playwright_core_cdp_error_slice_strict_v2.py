"""AAA controls for strict Core CDP error evidence across restarted workers."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
import run_playwright_core_cdp_error_slice_strict_v1 as base
import run_playwright_core_cdp_error_slice_strict_v2 as capture

HOST = "fixture.tail8f3b66.ts.net"


def _report(statuses: tuple[str, str, str]) -> dict:
    specs = []
    for worker_index, (spec_id, (line, title)) in enumerate(base.CASES.items()):
        passed = statuses[worker_index] == "passed"
        specs.append(
            {
                "id": spec_id,
                "file": base.REPORT_FILE,
                "line": line,
                "column": 5,
                "title": title,
                "ok": passed,
                "tests": [
                    {
                        "projectId": "chromium-library",
                        "expectedStatus": "passed",
                        "status": "expected" if passed else "unexpected",
                        "results": [
                            {
                                "retry": 0,
                                "workerIndex": worker_index,
                                "status": statuses[worker_index],
                                "errors": []
                                if passed
                                else [{"message": f"official failure {line}"}],
                            }
                        ],
                    }
                ],
            }
        )
    expected = sum(status == "passed" for status in statuses)
    return {
        "errors": [],
        "suites": [{"specs": specs, "suites": []}],
        "stats": {"expected": expected, "unexpected": 3 - expected, "skipped": 0, "flaky": 0},
    }


def _receipt(path: Path, workers: tuple[str | None, ...]) -> None:
    records = [
        {
            "pid": 1000 + index,
            "workerIndex": worker,
            "sourceSha256": base.SOURCE_SHA256,
            "adaptedSha256": base.adapted_source_sha256(HOST),
            "fixtureHost": HOST,
            "replacements": {"websocket": 1, "serverPrefixCases": 0},
        }
        for index, worker in enumerate(workers)
    ]
    path.write_text("".join(json.dumps(record) + "\n" for record in records))


def test_accepts_genuine_failed_test_worker_restarts(tmp_path: Path) -> None:
    # Arrange
    report = _report(("failed", "failed", "failed"))
    receipt = tmp_path / "receipt.jsonl"
    _receipt(receipt, (None, "0", "1", "2"))

    # Act
    selected = capture.selected_tests(report)
    capture.checked_receipts(receipt, selected, HOST)

    # Assert
    assert [item["attempts"][0]["workerIndex"] for item in selected] == [0, 1, 2]
    assert [item["status"] for item in selected] == ["unexpected"] * 3


def test_rejects_missing_restarted_worker_receipt(tmp_path: Path) -> None:
    # Arrange
    selected = capture.selected_tests(_report(("failed", "failed", "failed")))
    receipt = tmp_path / "receipt.jsonl"
    _receipt(receipt, (None, "0", "2"))

    # Act
    with pytest.raises(AssertionError):
        capture.checked_receipts(receipt, selected, HOST)

    # Assert
    assert {item["attempts"][0]["workerIndex"] for item in selected} == {0, 1, 2}


def test_rejects_tampered_restarted_worker_host(tmp_path: Path) -> None:
    # Arrange
    selected = capture.selected_tests(_report(("failed", "failed", "failed")))
    receipt = tmp_path / "receipt.jsonl"
    _receipt(receipt, (None, "0", "1", "2"))
    records = [json.loads(line) for line in receipt.read_text().splitlines()]
    records[-1]["fixtureHost"] = "other.tail8f3b66.ts.net"
    receipt.write_text("".join(json.dumps(record) + "\n" for record in records))

    # Act
    with pytest.raises(AssertionError):
        capture.checked_receipts(receipt, selected, HOST)

    # Assert
    assert records[-1]["workerIndex"] == "2"


def test_rejects_skip_masked_as_official_failure() -> None:
    # Arrange
    report = _report(("failed", "failed", "failed"))
    report["suites"][0]["specs"][1]["tests"][0]["results"][0]["status"] = "skipped"

    # Act
    with pytest.raises(AssertionError):
        capture.selected_tests(report)

    # Assert
    assert report["stats"]["unexpected"] == 3


def test_rejects_extra_case_even_if_stats_match() -> None:
    # Arrange
    report = _report(("passed", "passed", "passed"))
    report["suites"][0]["specs"].append(report["suites"][0]["specs"][0])

    # Act
    with pytest.raises(AssertionError):
        capture.selected_tests(report)

    # Assert
    assert len(report["suites"][0]["specs"]) == 4
