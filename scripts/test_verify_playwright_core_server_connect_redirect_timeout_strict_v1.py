"""Negative controls for strict Core redirect-timeout evidence verification."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

import pytest
import run_playwright_core_server_connect_redirect_timeout_strict_v1 as capture
import verify_playwright_core_server_connect_redirect_timeout_strict_v1 as verify

HOST = "example-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"
SUITE = capture.LIBRARY_SUITE


def report(monkeypatch: pytest.MonkeyPatch) -> dict[str, Any]:
    specs = []
    identities = []
    for index, project in enumerate(sorted(capture.PROJECTS_BY_SUITE[SUITE])):
        spec_id = f"pinned-{project}"
        identities.append(f"{spec_id}\t{project}\t{capture.LINE}")
        specs.append(
            {
                "id": spec_id,
                "file": "library/browsertype-connect.spec.ts",
                "line": capture.LINE,
                "column": 1,
                "title": capture.TITLE,
                "ok": True,
                "tests": [
                    {
                        "projectId": project,
                        "expectedStatus": "passed",
                        "status": "expected",
                        "results": [
                            {
                                "retry": 0,
                                "status": "passed",
                                "workerIndex": index,
                                "errors": [],
                            }
                        ],
                    }
                ],
            }
        )
    digest = hashlib.sha256(("\n".join(sorted(identities)) + "\n").encode()).hexdigest()
    monkeypatch.setattr(capture, "IDENTITIES_SHA256_BY_SUITE", {SUITE: digest})
    return {
        "errors": [],
        "suites": [
            {"title": "browsertype-connect.spec.ts", "specs": specs, "suites": []}
        ],
        "stats": {"expected": 3, "unexpected": 0, "skipped": 0, "flaky": 0},
    }


def receipts(mode: str = "oracle") -> list[dict[str, Any]]:
    value = []
    for worker in range(3):
        common = {
            "pid": 1000 + worker,
            "workerIndex": str(worker),
            "mode": mode,
            "fixtureHost": HOST,
            "fixtureBind": BIND,
            "ownerBound": True if mode == "candidate" else None,
            "cdpEndpointPresent": False,
        }
        value.extend(
            [
                {
                    **common,
                    "event": "source-verified",
                    "sourceSha256": capture.SOURCE_SHA256,
                    "fixtureSha256": capture.TEST_SERVER_SHA256,
                },
                {
                    **common,
                    "event": "fixture-request",
                    "requestId": 1,
                    "requestPath": "/connect/json",
                    "requestHost": f"{HOST}:12345",
                    "requestMethod": "GET",
                },
                {
                    **common,
                    "event": "redirect-response",
                    "requestId": 1,
                    "status": 302,
                    "location": "/connect/slow",
                },
                {
                    **common,
                    "event": "fixture-request",
                    "requestId": 2,
                    "requestPath": "/connect/slow",
                    "requestHost": f"{HOST}:12345",
                    "requestMethod": "GET",
                },
                {**common, "event": "slow-socket-closed", "requestId": 2},
            ]
        )
    return value


def checked(
    tmp_path: Path,
    records: list[dict[str, Any]],
    selected: list[dict[str, Any]],
    mode: str = "oracle",
) -> dict[str, int]:
    receipt = tmp_path / "receipt.jsonl"
    receipt.write_text("".join(json.dumps(item) + "\n" for item in records))
    return verify.checked_receipts(receipt, mode, selected, HOST, BIND)


def test_exact_three_case_identity_is_accepted(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    raw = report(monkeypatch)

    # Act
    selected = capture.selected_tests(raw, SUITE)

    # Assert
    assert len(selected) == 3
    assert {item["projectId"] for item in selected} == capture.PROJECTS_BY_SUITE[SUITE]
    assert {item["line"] for item in selected} == {capture.LINE}


def test_candidate_red_is_preserved(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    raw = report(monkeypatch)
    failed = raw["suites"][0]["specs"][0]
    failed["ok"] = False
    failed["tests"][0]["status"] = "unexpected"
    failed["tests"][0]["results"][0]["status"] = "failed"
    failed["tests"][0]["results"][0]["errors"] = [{"message": "wrong timeout"}]
    raw["stats"]["expected"] = 2
    raw["stats"]["unexpected"] = 1
    records = [
        item
        for item in receipts("candidate")
        if item["workerIndex"] != "0" or item["event"] == "source-verified"
    ]

    # Act
    selected = capture.selected_tests(raw, SUITE)
    counts = checked(tmp_path, records, selected, "candidate")

    # Assert
    assert sum(item["status"] == "unexpected" for item in selected) == 1
    assert counts["/connect/slow"] == 2


@pytest.mark.parametrize(
    "change",
    [
        "wrong-id",
        "wrong-project",
        "wrong-line",
        "wrong-column",
        "wrong-title",
        "skip",
        "retry",
        "missing-row",
        "duplicate-row",
        "unreported-error",
    ],
)
def test_identity_or_outcome_drift_is_rejected(
    monkeypatch: pytest.MonkeyPatch, change: str
) -> None:
    # Arrange
    raw = report(monkeypatch)
    spec = raw["suites"][0]["specs"][0]
    case = spec["tests"][0]
    if change == "wrong-id":
        spec["id"] = "tampered-id"
    elif change == "wrong-project":
        case["projectId"] = "other-project"
    elif change == "wrong-line":
        spec["line"] = 1055
    elif change == "wrong-column":
        spec["column"] = 5
    elif change == "wrong-title":
        spec["title"] = "different assertion"
    elif change == "skip":
        case["status"] = "skipped"
    elif change == "retry":
        case["results"][0]["retry"] = 1
    elif change == "missing-row":
        raw["suites"][0]["specs"].pop()
    elif change == "duplicate-row":
        raw["suites"][0]["specs"].append(spec)
    elif change == "unreported-error":
        case["results"][0]["errors"] = [{"message": "hidden failure"}]

    # Act / Assert
    with pytest.raises(AssertionError):
        capture.selected_tests(raw, SUITE)


def test_valid_receipts_are_accepted(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    selected = capture.selected_tests(report(monkeypatch), SUITE)

    # Act
    counts = checked(tmp_path, receipts(), selected)

    # Assert
    assert counts == {
        "/connect/json": 3,
        "/connect/slow": 3,
        "redirect-response": 3,
        "slow-socket-closed": 3,
    }


@pytest.mark.parametrize(
    "change",
    [
        "wrong-host",
        "wrong-bind",
        "wrong-mode",
        "cdp",
        "wrong-method",
        "wrong-path",
        "wrong-location",
        "wrong-status",
        "missing-source",
        "missing-redirect",
        "missing-close",
        "duplicate-request",
        "wrong-request-id",
        "slow-finished",
    ],
)
def test_receipt_tampering_is_rejected(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path, change: str
) -> None:
    # Arrange
    selected = capture.selected_tests(report(monkeypatch), SUITE)
    records = receipts()
    if change == "wrong-host":
        records[1]["requestHost"] = "localhost:12345"
    elif change == "wrong-bind":
        records[1]["fixtureBind"] = "127.0.0.1"
    elif change == "wrong-mode":
        records[1]["mode"] = "candidate"
    elif change == "cdp":
        records[1]["cdpEndpointPresent"] = True
    elif change == "wrong-method":
        records[1]["requestMethod"] = "POST"
    elif change == "wrong-path":
        records[1]["requestPath"] = "/other"
    elif change == "wrong-location":
        records[2]["location"] = "/other"
    elif change == "wrong-status":
        records[2]["status"] = 200
    elif change == "missing-source":
        records.pop(0)
    elif change == "missing-redirect":
        records.pop(2)
    elif change == "missing-close":
        records.pop(4)
    elif change == "duplicate-request":
        records.insert(2, records[1].copy())
    elif change == "wrong-request-id":
        records[4]["requestId"] = 3
    elif change == "slow-finished":
        records.append({**records[4], "event": "slow-response-finished"})

    # Act / Assert
    with pytest.raises(AssertionError):
        checked(tmp_path, records, selected)
