"""Negative controls for the isolated Core child-close diagnostic verifier."""

from __future__ import annotations

import json

import pytest

import verify_playwright_core_server_connect_child_close_synthetic_v1 as verify

HOST = "macbookpro-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"


def report() -> dict:
    return {
        "errors": [],
        "suites": [
            {
                "title": "run-server",
                "specs": [
                    {
                        "line": verify.TEST_LINE,
                        "file": "library/browsertype-connect.spec.ts",
                        "title": verify.TEST_TITLE,
                        "id": verify.TEST_ID,
                        "ok": True,
                        "tests": [
                            {
                                "projectId": "chromium-library",
                                "expectedStatus": "passed",
                                "status": "expected",
                                "results": [
                                    {"status": "passed", "retry": 0, "errors": []}
                                ],
                            }
                        ],
                    }
                ],
                "suites": [],
            }
        ],
        "stats": {"expected": 1, "unexpected": 0, "skipped": 0, "flaky": 0},
    }


def receipt() -> list[dict]:
    common = {"mode": "oracle", "fixtureHost": HOST, "fixtureBind": BIND}
    records = [
        {
            **common,
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
        {**common, "kind": "fixture-source-read", "workerIndex": "0"},
        {
            **common,
            "kind": "selected-child-close-source",
            "workerIndex": "0",
            "upstreamSourceSha256": verify.SOURCE_HASHES[
                "tests/library/browsertype-connect.spec.ts"
            ],
            "selectedBodySha256": verify.hashlib.sha256(
                verify._selected_body().encode()
            ).hexdigest(),
            "testSourceAdapted": False,
        },
    ]
    synthetic = {**common, "synthetic": True, "browserProcessLaunched": False}
    for kind in (
        "listening",
        "upgrade",
        "root-initialize",
        "bootstrap-sent",
        "context-created",
        "page-created",
        "stop-requested",
        "close",
        "stop",
    ):
        record = {**synthetic, "kind": kind}
        if kind == "root-initialize":
            record.update({"sdkLanguage": "javascript", "id": 1})
        if kind == "close":
            record["serverInitiated"] = True
        records.append(record)
    return records


def test_exact_upstream_case_identity_is_accepted() -> None:
    # Arrange
    value = report()

    # Act
    selected = verify.selected_test(value)

    # Assert
    assert selected["specId"] == verify.TEST_ID


@pytest.mark.parametrize("field", ["id", "line", "title"])
def test_tampered_case_identity_is_rejected(field: str) -> None:
    # Arrange
    value = report()
    value["suites"][0]["specs"][0][field] = "invented"

    # Act / Assert
    with pytest.raises(ValueError, match="selected close-event spec identity changed"):
        verify.selected_test(value)


@pytest.mark.parametrize("kind", ["context-created", "page-created", "close"])
def test_missing_context_page_or_server_close_is_rejected(tmp_path, kind: str) -> None:
    # Arrange
    path = tmp_path / "receipt.jsonl"
    records = [item for item in receipt() if item["kind"] != kind]
    path.write_text("".join(json.dumps(item) + "\n" for item in records))

    # Act / Assert
    with pytest.raises(
        ValueError, match="Context/Page graph|server-initiated WebSocket close"
    ):
        verify.verify_receipt(path, "oracle", HOST, BIND, True)


def test_browser_process_claim_is_rejected(tmp_path) -> None:
    # Arrange
    path = tmp_path / "receipt.jsonl"
    records = receipt()
    next(item for item in records if item["kind"] == "listening")[
        "browserProcessLaunched"
    ] = True
    path.write_text("".join(json.dumps(item) + "\n" for item in records))

    # Act / Assert
    with pytest.raises(ValueError, match="browser launch claim"):
        verify.verify_receipt(path, "oracle", HOST, BIND, True)


def test_selected_source_hash_tampering_is_rejected(tmp_path) -> None:
    # Arrange
    path = tmp_path / "receipt.jsonl"
    records = receipt()
    next(item for item in records if item["kind"] == "selected-child-close-source")[
        "selectedBodySha256"
    ] = "bad"
    path.write_text("".join(json.dumps(item) + "\n" for item in records))

    # Act / Assert
    with pytest.raises(ValueError, match="selected original assertion"):
        verify.verify_receipt(path, "oracle", HOST, BIND, True)
