"""Browser-free negative controls for the fixture-adapted Core lifecycle verifier."""

from __future__ import annotations

import json

import pytest

import verify_playwright_core_server_connect_lifecycle_synthetic_v1 as verify


HOST = "macbookpro-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"


def _report() -> dict:
    specs = []
    for line, (title, spec_id) in verify.EXPECTED_IDS.items():
        specs.append(
            {
                "line": line,
                "file": "library/browsertype-connect.spec.ts",
                "title": title,
                "id": spec_id,
                "ok": True,
                "tests": [
                    {
                        "projectId": "chromium-library",
                        "expectedStatus": "passed",
                        "status": "expected",
                        "results": [{"status": "passed", "retry": 0, "errors": []}],
                    }
                ],
            }
        )
    return {
        "errors": [],
        "suites": [{"title": "run-server", "specs": specs, "suites": []}],
        "stats": {"expected": 2, "unexpected": 0, "skipped": 0, "flaky": 0},
    }


def _receipt_records() -> list[dict]:
    shared = {"mode": "oracle", "fixtureHost": HOST, "fixtureBind": BIND}
    preload = {
        **shared,
        "kind": "preload",
        "workerIndex": "0",
        "upstreamSourceSha256": verify.SOURCE_HASHES[
            "tests/library/browsertype-connect.spec.ts"
        ],
        "upstreamFixtureSha256": verify.SOURCE_HASHES["tests/config/remoteServer.ts"],
        "testSourceAdapted": False,
        "fixtureAdapted": True,
        "cdpEndpointPresent": False,
        "ownerBound": None,
    }
    records = [preload, {**shared, "kind": "fixture-source-read", "workerIndex": "0"}]
    for pid in (1001, 1002):
        base = {
            **shared,
            "synthetic": True,
            "browserProcessLaunched": False,
            "pid": pid,
        }
        records.extend(
            [
                {**base, "kind": "listening", "endpointPath": "/browser-synthetic"},
                {**base, "kind": "upgrade"},
                {
                    **base,
                    "kind": "root-initialize",
                    "sdkLanguage": "javascript",
                    "id": 1,
                },
                {**base, "kind": "bootstrap-sent", "frames": 8, "version": "155.0"},
                {**base, "kind": "stop-requested"},
                {**base, "kind": "close", "forcedByFixtureTeardown": False},
                {**base, "kind": "stop"},
            ]
        )
    return records


def test_exact_two_case_ids_are_accepted() -> None:
    # Arrange
    report = _report()

    # Act
    selected = verify._selected_tests(report)

    # Assert
    assert [item["line"] for item in selected] == [304, 321]


def test_tampered_case_id_is_rejected() -> None:
    # Arrange
    report = _report()
    report["suites"][0]["specs"][0]["id"] = "invented-case-id"

    # Act / Assert
    with pytest.raises(ValueError, match="selected spec identity changed"):
        verify._selected_tests(report)


@pytest.mark.parametrize("tamper", ["missing", "forced"])
def test_missing_or_forced_close_receipt_is_rejected(tmp_path, tamper: str) -> None:
    # Arrange
    receipt = tmp_path / "receipt.jsonl"
    records = _receipt_records()
    if tamper == "missing":
        records = [
            item
            for item in records
            if not (item["kind"] == "close" and item.get("pid") == 1002)
        ]
    else:
        next(
            item
            for item in records
            if item["kind"] == "close" and item.get("pid") == 1002
        )["forcedByFixtureTeardown"] = True
    receipt.write_text("".join(json.dumps(item) + "\n" for item in records))

    # Act / Assert
    with pytest.raises(ValueError, match="distinct clean WS close receipts"):
        verify._receipts(receipt, "oracle", HOST, BIND, 2)
