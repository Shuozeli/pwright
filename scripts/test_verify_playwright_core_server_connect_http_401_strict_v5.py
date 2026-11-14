"""Negative controls for the versioned HTTP-401 diagnostic verifier."""

from __future__ import annotations

import json

import pytest
import run_playwright_core_server_connect_http_401_strict_v5 as capture
import verify_playwright_core_server_connect_http_401_strict_v5 as verify


def report(statuses: tuple[str, str] = ("passed", "passed")) -> dict:
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
                "line": 116,
                "column": 5,
                "title": "should print HTTP error",
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


def records(mode: str = "oracle") -> list[dict]:
    result = []
    for worker in (0, 1):
        common = {
            "pid": 100 + worker,
            "workerIndex": str(worker),
            "mode": mode,
            "fixtureHost": "example-yuacx.tail8f3b66.ts.net",
            "fixtureBind": "100.78.97.63",
            "ownerBound": True if mode == "candidate" else None,
            "cdpEndpointPresent": False,
        }
        result.append(
            {
                **common,
                "event": "source-adapted",
                "sourceSha256": capture.SOURCE_SHA256,
                "adaptedSha256": capture.ADAPTED_SOURCE_SHA256,
                "replacements": 1,
            }
        )
        result.append(
            {
                **common,
                "event": "fixture-401-sent",
                "responseSha256": capture.FIXTURE_RESPONSE_SHA256,
            }
        )
    return result


def check_records(tmp_path, value: list[dict], mode: str = "oracle") -> None:
    destination = tmp_path / "guard.jsonl"
    destination.write_text("".join(json.dumps(item) + "\n" for item in value))
    verify.checked_receipts(
        destination,
        mode,
        capture.selected_tests(report()),
        "example-yuacx.tail8f3b66.ts.net",
        "100.78.97.63",
    )


def test_pinned_source_assertions_and_fixture_are_unchanged() -> None:
    # Arrange
    source = capture.UPSTREAM / capture.SOURCE

    # Act
    capture.check_source_and_manifest()

    # Assert
    assert source.is_file()
    assert capture.ADAPTED_SOURCE_SHA256 != capture.SOURCE_SHA256


def test_v2_frozen_snapshot_remains_readable_after_live_rebuild() -> None:
    # Arrange
    snapshot = capture.V2_SNAPSHOT_PACKAGE

    # Act
    capture.verify_v2_snapshot()

    # Assert
    assert snapshot.is_dir()
    assert capture.ARTIFACT_SHA256 != capture.V2_ARTIFACT_SHA256


def test_exact_two_expected_cases_are_selected() -> None:
    # Arrange
    raw = report()

    # Act
    selected = capture.selected_tests(raw)

    # Assert
    assert [item["kind"] for item in selected] == ["launchServer", "run-server"]
    assert [item["status"] for item in selected] == ["expected", "expected"]


def test_candidate_failure_remains_unexpected() -> None:
    # Arrange
    raw = report(("failed", "passed"))

    # Act
    selected = capture.selected_tests(raw)

    # Assert
    assert [item["status"] for item in selected] == ["unexpected", "expected"]


@pytest.mark.parametrize(
    "change", ["duplicate", "wrong-line", "skip", "retry", "wrong-project"]
)
def test_report_identity_or_outcome_drift_is_rejected(change: str) -> None:
    # Arrange
    raw = report()
    first = raw["suites"][0]["specs"][0]
    if change == "duplicate":
        raw["suites"][0]["specs"][1] = first
    elif change == "wrong-line":
        first["line"] = 122
    elif change == "skip":
        raw["stats"]["skipped"] = 1
    elif change == "retry":
        first["tests"][0]["results"][0]["retry"] = 1
    else:
        first["tests"][0]["projectId"] = "firefox-library"

    # Act / Assert
    with pytest.raises(AssertionError):
        capture.selected_tests(raw)


def test_valid_worker_and_fixture_receipts_are_accepted(tmp_path) -> None:
    # Arrange
    value = records("candidate")

    # Act
    check_records(tmp_path, value, "candidate")

    # Assert
    assert len(value) == 4


@pytest.mark.parametrize(
    "change",
    [
        "missing-adaptation",
        "missing-response",
        "duplicate-response",
        "wrong-adapted-digest",
        "wrong-response-digest",
        "wrong-host",
        "cdp-endpoint",
        "candidate-owner",
        "wrong-worker",
        "extra-replacement",
    ],
)
def test_receipt_tampering_is_rejected(tmp_path, change: str) -> None:
    # Arrange
    value = records()
    if change == "missing-adaptation":
        value.pop(0)
    elif change == "missing-response":
        value.pop(1)
    elif change == "duplicate-response":
        value.append(value[1].copy())
    elif change == "wrong-adapted-digest":
        value[0]["adaptedSha256"] = "0" * 64
    elif change == "wrong-response-digest":
        value[1]["responseSha256"] = "0" * 64
    elif change == "wrong-host":
        value[0]["fixtureHost"] = "localhost"
    elif change == "cdp-endpoint":
        value[0]["cdpEndpointPresent"] = True
    elif change == "candidate-owner":
        value[0]["ownerBound"] = True
    elif change == "wrong-worker":
        value[1]["workerIndex"] = "4"
    else:
        value[0]["replacements"] = 2

    # Act / Assert
    with pytest.raises(AssertionError):
        check_records(tmp_path, value)
