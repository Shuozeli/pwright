"""Mutation controls for the browser-free Core remote-waiter verifier."""

from __future__ import annotations

import json

import pytest
import verify_playwright_core_server_connect_waiters_synthetic_v1 as verify

HOST = "macbookpro-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"


def _report(mode: str) -> dict:
    specs = []
    for line, (title, spec_id, absent_method) in verify.EXPECTED_IDS.items():
        passed = mode == "oracle"
        specs.append(
            {
                "line": line,
                "file": "library/browsertype-connect.spec.ts",
                "title": title,
                "id": spec_id,
                "ok": passed,
                "tests": [
                    {
                        "projectId": "chromium-library",
                        "expectedStatus": "passed",
                        "status": "expected" if passed else "unexpected",
                        "results": [
                            {
                                "status": "passed" if passed else "failed",
                                "retry": 0,
                                "errors": []
                                if passed
                                else [
                                    {
                                        "message": f"TypeError: newPage.{absent_method} is not a function",
                                        "location": {
                                            "file": str(
                                                verify.UPSTREAM
                                                / "tests/library/browsertype-connect.spec.ts"
                                            ),
                                            "line": verify.EXPECTED_FAILURE_LINES[line],
                                            "column": 17 if line == 396 else 31,
                                        },
                                    }
                                ],
                            }
                        ],
                    }
                ],
            }
        )
    return {
        "errors": [],
        "suites": [{"title": "run-server", "specs": specs, "suites": []}],
        "stats": {
            "expected": 3 if mode == "oracle" else 0,
            "unexpected": 0 if mode == "oracle" else 3,
            "skipped": 0,
            "flaky": 0,
        },
    }


def _receipt_records() -> list[dict]:
    shared = {"mode": "oracle", "fixtureHost": HOST, "fixtureBind": BIND}
    records = [
        {
            **shared,
            "kind": "preload",
            "workerIndex": "0",
            "upstreamSourceSha256": verify.SOURCE_HASHES[
                "tests/library/browsertype-connect.spec.ts"
            ],
            "upstreamFixtureSha256": verify.SOURCE_HASHES[
                "tests/config/remoteServer.ts"
            ],
            "testSourceAdapted": False,
            "fixtureAdapted": True,
            "cdpEndpointPresent": False,
            "ownerBound": None,
        },
        {**shared, "kind": "fixture-source-read", "workerIndex": "0"},
    ]
    for pid in (1001, 1002, 1003):
        base = {
            **shared,
            "synthetic": True,
            "browserProcessLaunched": False,
            "pid": pid,
        }
        events = ("request", "response") if pid == 1001 else ("download",)
        arm_records = []
        for event in events:
            arm_records.extend(
                [
                    {
                        **base,
                        "kind": "wait-event-info-replied",
                        "guid": "page@1",
                        "phase": "before",
                        "event": event,
                    },
                    {
                        **base,
                        "kind": "wait-event-info-replied",
                        "guid": "page@1",
                        "phase": "log",
                        "event": None,
                    },
                ]
            )
            if event != "download":
                arm_records.append(
                    {
                        **base,
                        "kind": "subscription-replied",
                        "guid": "page@1",
                        "event": event,
                        "enabled": True,
                    }
                )
        records.extend(
            [
                {
                    **base,
                    "kind": "listening",
                    "endpointPath": "/browser-synthetic-waiters-v1",
                },
                {**base, "kind": "upgrade"},
                {
                    **base,
                    "kind": "root-initialize",
                    "sdkLanguage": "javascript",
                    "id": 1,
                },
                {**base, "kind": "bootstrap-sent", "frames": 8, "version": "155.0"},
                {**base, "kind": "new-context-replied", "contextGuid": "context@1"},
                {**base, "kind": "new-page-replied", "pageGuid": "page@1"},
                *arm_records,
                {**base, "kind": "stop-requested"},
                {**base, "kind": "close", "phase": 3, "forcedByFixtureTeardown": False},
                {**base, "kind": "stop"},
            ]
        )
    return records


def test_exact_three_oracle_cases_are_accepted() -> None:
    # Arrange
    report = _report("oracle")

    # Act
    selected = verify._selected_tests(report, "oracle")

    # Assert
    assert [item["line"] for item in selected] == [396, 412, 423]


def test_tampered_spec_id_is_rejected() -> None:
    # Arrange
    report = _report("oracle")
    report["suites"][0]["specs"][0]["id"] = "invented-case-id"

    # Act / Assert
    with pytest.raises(ValueError, match="selected spec identity changed"):
        verify._selected_tests(report, "oracle")


def test_unrelated_candidate_failure_is_rejected() -> None:
    # Arrange
    report = _report("candidate")
    report["suites"][0]["specs"][0]["tests"][0]["results"][0]["errors"][0][
        "message"
    ] = "fixture server failed to start"

    # Act / Assert
    with pytest.raises(ValueError, match="outside the expected missing waiter"):
        verify._selected_tests(report, "candidate")


def test_later_waiter_failure_is_rejected() -> None:
    # Arrange
    report = _report("candidate")
    report["suites"][0]["specs"][0]["tests"][0]["results"][0]["errors"][0]["location"][
        "line"
    ] = 402

    # Act / Assert
    with pytest.raises(ValueError, match="first waiter call"):
        verify._selected_tests(report, "candidate")


@pytest.mark.parametrize(
    "tamper",
    [
        "missing-page",
        "missing-waiter",
        "launched-browser",
        "mixed-lane",
        "protocol-error",
    ],
)
def test_bad_wire_receipt_is_rejected(tmp_path, tamper: str) -> None:
    # Arrange
    records = _receipt_records()
    if tamper == "missing-page":
        records = [
            item
            for item in records
            if not (item["kind"] == "new-page-replied" and item.get("pid") == 1003)
        ]
    elif tamper == "missing-waiter":
        records = [
            item
            for item in records
            if not (
                item["kind"] == "wait-event-info-replied"
                and item.get("pid") == 1003
                and item.get("phase") == "before"
            )
        ]
    elif tamper == "launched-browser":
        next(item for item in records if item["kind"] == "upgrade")[
            "browserProcessLaunched"
        ] = True
    elif tamper == "mixed-lane":
        records[0]["mode"] = "candidate"
    else:
        records.append(
            {
                "mode": "oracle",
                "fixtureHost": HOST,
                "fixtureBind": BIND,
                "synthetic": True,
                "browserProcessLaunched": False,
                "pid": 1001,
                "kind": "protocol-error",
            }
        )
    receipt = tmp_path / "receipt.jsonl"
    receipt.write_text("".join(json.dumps(item) + "\n" for item in records))

    # Act / Assert
    with pytest.raises(ValueError):
        verify._receipts(receipt, "oracle", HOST, BIND)
