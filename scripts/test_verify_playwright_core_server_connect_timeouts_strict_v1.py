"""Negative controls for the versioned Core connect-timeout evidence verifier."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

import pytest
import run_playwright_core_server_connect_timeouts_strict_v1 as capture
import verify_playwright_core_server_connect_timeouts_strict_v1 as verify

HOST = "example-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"


def report(monkeypatch: pytest.MonkeyPatch) -> dict[str, Any]:
    suites: list[dict[str, Any]] = []
    identities: list[str] = []
    for kind in ("launchServer", "run-server"):
        specs: list[dict[str, Any]] = []
        for project in sorted(capture.PROJECTS):
            for line, title in capture.CASE_TITLES.items():
                spec_id = f"pinned-{project}-{line}-{kind}"
                identities.append(f"{spec_id}\t{project}\t{line}\t{kind}")
                specs.append(
                    {
                        "id": spec_id,
                        "file": "library/browsertype-connect.spec.ts",
                        "line": line,
                        "column": 5,
                        "title": title,
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
                                        "workerIndex": 0,
                                        "errors": [],
                                    }
                                ],
                            }
                        ],
                    }
                )
        suites.append({"title": kind, "specs": specs, "suites": []})
    digest = hashlib.sha256(("\n".join(sorted(identities)) + "\n").encode()).hexdigest()
    monkeypatch.setattr(capture, "EXPECTED_IDENTITIES_SHA256", digest)
    return {
        "errors": [],
        "suites": suites,
        "stats": {"expected": 36, "unexpected": 0, "skipped": 0, "flaky": 0},
    }


def records(mode: str = "oracle") -> list[dict[str, Any]]:
    common = {
        "pid": 123,
        "workerIndex": "0",
        "mode": mode,
        "fixtureHost": HOST,
        "fixtureBind": BIND,
        "ownerBound": True if mode == "candidate" else None,
        "cdpEndpointPresent": False,
    }
    result = [
        {
            **common,
            "event": "source-adapted",
            "sourceSha256": capture.SOURCE_SHA256,
            "adaptedSha256": capture.ADAPTED_SOURCE_SHA256,
            "replacements": 2,
        }
    ]
    for path in ("/ws-slow", "/ws"):
        result.extend(
            {
                **common,
                "event": "fixture-upgrade-request",
                "requestPath": path,
                "requestHost": f"{HOST}:12345",
            }
            for _ in range(18)
        )
    return result


def check_records(
    tmp_path: Path,
    value: list[dict[str, Any]],
    selected: list[dict[str, Any]],
    mode: str = "oracle",
) -> dict[str, int]:
    destination = tmp_path / "receipt.jsonl"
    destination.write_text("".join(json.dumps(item) + "\n" for item in value))
    return verify.checked_receipts(destination, mode, selected, HOST, BIND)


def test_exact_36_case_identity_is_accepted(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    raw = report(monkeypatch)

    # Act
    selected = capture.selected_tests(raw)

    # Assert
    assert len(selected) == 36
    assert {item["projectId"] for item in selected} == capture.PROJECTS
    assert {item["line"] for item in selected} == {221, 228}


def test_candidate_red_is_preserved(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    raw = report(monkeypatch)
    spec = raw["suites"][0]["specs"][0]
    spec["ok"] = False
    case = spec["tests"][0]
    case["status"] = "unexpected"
    case["results"][0]["status"] = "failed"
    case["results"][0]["errors"] = [{"message": "browserType.connect: wrong timeout"}]
    raw["stats"]["expected"] = 35
    raw["stats"]["unexpected"] = 1

    # Act
    selected = capture.selected_tests(raw)

    # Assert
    assert sum(item["status"] == "unexpected" for item in selected) == 1


@pytest.mark.parametrize(
    "change",
    [
        "wrong-id",
        "wrong-project",
        "wrong-line",
        "wrong-title",
        "skip",
        "retry",
        "missing-error",
        "extra-stat",
        "missing-case",
    ],
)
def test_report_identity_or_outcome_drift_is_rejected(
    monkeypatch: pytest.MonkeyPatch, change: str
) -> None:
    # Arrange
    raw = report(monkeypatch)
    spec = raw["suites"][0]["specs"][0]
    case = spec["tests"][0]
    if change == "wrong-id":
        spec["id"] = "unrecognized-id"
    elif change == "wrong-project":
        case["projectId"] = "mobile-chrome-library"
    elif change == "wrong-line":
        spec["line"] = 130
    elif change == "wrong-title":
        spec["title"] = "should timeout eventually"
    elif change == "skip":
        raw["stats"]["skipped"] = 1
    elif change == "retry":
        case["results"][0]["retry"] = 1
    elif change == "missing-error":
        case["results"][0]["status"] = "failed"
    elif change == "extra-stat":
        raw["stats"]["expected"] = 37
    else:
        raw["suites"][0]["specs"].pop()

    # Act / Assert
    with pytest.raises(AssertionError):
        capture.selected_tests(raw)


def test_valid_candidate_receipts_are_accepted(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    selected = capture.selected_tests(report(monkeypatch))
    value = records("candidate")

    # Act
    counts = check_records(tmp_path, value, selected, "candidate")

    # Assert
    assert counts == {"/ws": 18, "/ws-slow": 18}


@pytest.mark.parametrize(
    "change",
    [
        "missing-adaptation",
        "missing-upgrade",
        "duplicate-upgrade",
        "wrong-source-digest",
        "wrong-adapted-digest",
        "wrong-host",
        "wrong-request-host",
        "wrong-path",
        "cdp-endpoint",
        "wrong-owner",
        "wrong-worker",
        "extra-replacement",
    ],
)
def test_receipt_tampering_is_rejected(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path, change: str
) -> None:
    # Arrange
    selected = capture.selected_tests(report(monkeypatch))
    value = records()
    if change == "missing-adaptation":
        value.pop(0)
    elif change == "missing-upgrade":
        value.pop(1)
    elif change == "duplicate-upgrade":
        value.append(value[1].copy())
    elif change == "wrong-source-digest":
        value[0]["sourceSha256"] = "0" * 64
    elif change == "wrong-adapted-digest":
        value[0]["adaptedSha256"] = "0" * 64
    elif change == "wrong-host":
        value[0]["fixtureHost"] = "localhost"
    elif change == "wrong-request-host":
        value[1]["requestHost"] = "localhost:12345"
    elif change == "wrong-path":
        value[1]["requestPath"] = "/other"
    elif change == "cdp-endpoint":
        value[0]["cdpEndpointPresent"] = True
    elif change == "wrong-owner":
        value[0]["ownerBound"] = True
    elif change == "wrong-worker":
        value[1]["workerIndex"] = None
    else:
        value[0]["replacements"] = 3

    # Act / Assert
    with pytest.raises((AssertionError, TypeError)):
        check_records(tmp_path, value, selected)
