"""Negative controls for the browser-free Core Server-connect Close verifier."""

from __future__ import annotations

import json

import pytest
import run_playwright_core_server_connect_close_error_strict_v3 as capture
import verify_playwright_core_server_connect_close_error_strict_v3 as verify


def report(statuses: tuple[str, str]) -> dict:
    specs = []
    for spec_id, kind, status in zip(
        capture.EXPECTED_SPEC_IDS,
        capture.EXPECTED_SPEC_IDS.values(),
        statuses,
        strict=True,
    ):
        passed = status == "passed"
        specs.append(
            {
                "id": spec_id,
                "file": "library/browsertype-connect.spec.ts",
                "line": 130,
                "column": 5,
                "title": "should print custom ws close error",
                "ok": passed,
                "tests": [
                    {
                        "projectId": "chromium-library",
                        "expectedStatus": "passed",
                        "status": "expected" if passed else "unexpected",
                        "results": [
                            {
                                "retry": 0,
                                "workerIndex": 0 if kind == "launchServer" else 1,
                                "status": status,
                                "errors": []
                                if passed
                                else [{"message": "browserType.connect unsupported"}],
                            }
                        ],
                    }
                ],
            }
        )
    return {
        "errors": [],
        "suites": [{"specs": specs, "suites": []}],
        "stats": {
            "expected": statuses.count("passed"),
            "unexpected": statuses.count("failed"),
            "skipped": 0,
            "flaky": 0,
        },
    }


def test_pinned_source_and_manifest_select_custom_close() -> None:
    # Arrange
    source = capture.UPSTREAM / capture.SOURCE

    # Act
    capture.check_source_and_manifest()

    # Assert
    assert source.is_file()
    assert capture.SUITE.endswith("close-error-strict-v3")


def test_exact_two_oracle_passes_are_selected() -> None:
    # Arrange
    raw = report(("passed", "passed"))

    # Act
    selected = capture.selected_tests(raw)

    # Assert
    assert [item["kind"] for item in selected] == ["launchServer", "run-server"]
    assert [item["status"] for item in selected] == ["expected", "expected"]


def test_candidate_failures_remain_unexpected() -> None:
    # Arrange
    raw = report(("failed", "failed"))

    # Act
    selected = capture.selected_tests(raw)

    # Assert
    assert [item["status"] for item in selected] == ["unexpected", "unexpected"]
    assert all(
        "browserType.connect unsupported" in item["errors"][0] for item in selected
    )


def test_unrelated_upstream_line_is_rejected() -> None:
    # Arrange
    raw = report(("passed", "passed"))
    raw["suites"][0]["specs"][0]["line"] = 122

    # Act / Assert
    with pytest.raises(AssertionError):
        capture.selected_tests(raw)


def test_duplicate_reported_case_is_rejected() -> None:
    # Arrange
    raw = report(("passed", "passed"))
    raw["suites"][0]["specs"][1] = raw["suites"][0]["specs"][0]

    # Act / Assert
    with pytest.raises(AssertionError):
        capture.selected_tests(raw)


def test_invented_skip_is_rejected() -> None:
    # Arrange
    raw = report(("passed", "passed"))
    raw["stats"]["skipped"] = 1

    # Act / Assert
    with pytest.raises(AssertionError):
        capture.selected_tests(raw)


def test_receipt_with_cdp_endpoint_is_rejected(tmp_path) -> None:
    # Arrange
    selected = capture.selected_tests(report(("passed", "passed")))
    receipt = tmp_path / "guard.jsonl"
    records = [
        {
            "pid": 123,
            "workerIndex": str(index),
            "mode": "oracle",
            "sourceSha256": capture.SOURCE_SHA256,
            "fixtureHost": "example-yuacx.tail8f3b66.ts.net",
            "fixtureBind": "100.78.97.63",
            "ownerBound": None,
            "cdpEndpointPresent": index == 1,
            "sourceAdapted": False,
        }
        for index in range(2)
    ]
    receipt.write_text("".join(json.dumps(record) + "\n" for record in records))

    # Act / Assert
    with pytest.raises(AssertionError):
        verify.checked_receipts(
            receipt,
            "oracle",
            selected,
            "example-yuacx.tail8f3b66.ts.net",
            "100.78.97.63",
        )


def test_receipt_with_source_adaptation_is_rejected(tmp_path) -> None:
    # Arrange
    selected = capture.selected_tests(report(("passed", "passed")))
    receipt = tmp_path / "guard.jsonl"
    records = [
        {
            "pid": 123,
            "workerIndex": str(index),
            "mode": "oracle",
            "sourceSha256": capture.SOURCE_SHA256,
            "fixtureHost": "example-yuacx.tail8f3b66.ts.net",
            "fixtureBind": "100.78.97.63",
            "ownerBound": None,
            "cdpEndpointPresent": False,
            "sourceAdapted": index == 1,
        }
        for index in range(2)
    ]
    receipt.write_text("".join(json.dumps(record) + "\n" for record in records))

    # Act / Assert
    with pytest.raises(AssertionError):
        verify.checked_receipts(
            receipt,
            "oracle",
            selected,
            "example-yuacx.tail8f3b66.ts.net",
            "100.78.97.63",
        )
