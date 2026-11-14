"""Negative controls for the fixture-host-adapted Core Close verifier."""

from __future__ import annotations

import json

import pytest
import run_playwright_core_server_connect_close_error_strict_v4 as capture
import verify_playwright_core_server_connect_close_error_strict_v4 as verify


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
                                "errors": (
                                    []
                                    if passed
                                    else [
                                        {"message": "browserType.connect unsupported"}
                                    ]
                                ),
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
    assert capture.SUITE.endswith("close-error-strict-v4")
    assert capture.FIXTURE_ADAPTATION.startswith("exact-line130-")


def test_fixture_dns_preflight_accepts_matching_magicdns_and_bind(monkeypatch) -> None:
    # Arrange
    def fake_check_output(command, *, text):
        assert text is True
        return (
            "100.78.97.63\n"
            if command == ["tailscale", "ip", "-4"]
            else json.dumps({"Self": {"DNSName": HOST + "."}})
        )

    monkeypatch.setattr(capture.subprocess, "check_output", fake_check_output)
    monkeypatch.setattr(
        capture.socket,
        "getaddrinfo",
        lambda *_args, **_kwargs: [(None, None, None, None, (BIND, 0))],
    )

    # Act
    proof = capture.fixture_dns_preflight()

    # Assert
    assert proof == {"fixtureHost": HOST, "fixtureBind": BIND, "resolvedIPv4": [BIND]}


def test_fixture_dns_preflight_rejects_mismatched_resolution(monkeypatch) -> None:
    # Arrange
    monkeypatch.setattr(
        capture.subprocess,
        "check_output",
        lambda command, *, text: (
            "100.78.97.63\n"
            if command == ["tailscale", "ip", "-4"]
            else json.dumps({"Self": {"DNSName": HOST + "."}})
        ),
    )
    monkeypatch.setattr(
        capture.socket,
        "getaddrinfo",
        lambda *_args, **_kwargs: [(None, None, None, None, ("100.78.97.64", 0))],
    )

    # Act / Assert
    with pytest.raises(ValueError, match="does not match"):
        capture.fixture_dns_preflight()


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


HOST = "example-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"


def receipt_records() -> list[dict]:
    records = []
    for index in range(2):
        port = capture.SERVER_PORT_BASE + index * capture.SERVER_PORT_STRIDE
        common = {
            "pid": 123 + index,
            "workerIndex": str(index),
            "mode": "oracle",
            "sourceSha256": capture.SOURCE_SHA256,
            "adaptedSourceSha256": capture.ADAPTED_SOURCE_SHA256,
            "fixtureHost": HOST,
            "fixtureBind": BIND,
            "ownerBound": None,
            "cdpEndpointPresent": False,
            "sourceAdapted": True,
            "fixtureAdapted": True,
        }
        listener = {
            "originalUrl": f"ws://localhost:{port}/ws",
            "mappedUrl": f"ws://{HOST}:{port}/ws",
            "port": port,
            "listenerAddress": BIND,
            "listenerPort": port,
            "listenerFamily": "IPv4",
            "requestHost": f"{HOST}:{port}",
        }
        records.extend(
            [
                {**common, "kind": "preload", "workerIndex": None, "replacements": 1},
                {**common, "kind": "source-read", "replacements": 1},
                {**common, **listener, "kind": "ws-upgrade"},
                {**common, **listener, "kind": "ws-message", "messageBytes": 73},
                {
                    **common,
                    **listener,
                    "kind": "ws-close",
                    "code": 4123,
                    "reason": "Oh my!",
                },
            ]
        )
    return records


def verify_records(tmp_path, records: list[dict]) -> None:
    receipt = tmp_path / "guard.jsonl"
    receipt.write_text("".join(json.dumps(record) + "\n" for record in records))
    selected = capture.selected_tests(report(("passed", "passed")))
    verify.checked_receipts(receipt, "oracle", selected, HOST, BIND)


def test_exact_two_fixture_mappings_are_verified(tmp_path) -> None:
    # Arrange
    records = receipt_records()

    # Act
    verify_records(tmp_path, records)

    # Assert
    assert sum(record["kind"] == "ws-upgrade" for record in records) == 2


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("cdpEndpointPresent", True),
        ("sourceAdapted", False),
        ("adaptedSourceSha256", "0" * 64),
        ("mappedUrl", "ws://localhost:8907/ws"),
        ("port", 8911),
        ("listenerAddress", "127.0.0.1"),
        ("listenerFamily", "IPv6"),
        ("requestHost", "localhost:8907"),
        ("workerIndex", "2"),
    ],
)
def test_tampered_mapping_receipt_is_rejected(tmp_path, field, value) -> None:
    # Arrange
    records = receipt_records()
    target = next(record for record in records if record["kind"] == "ws-upgrade")
    target[field] = value

    # Act / Assert
    with pytest.raises(AssertionError):
        verify_records(tmp_path, records)


def test_missing_source_read_is_rejected(tmp_path) -> None:
    # Arrange
    records = [
        record
        for record in receipt_records()
        if not (record["kind"] == "source-read" and record["workerIndex"] == "0")
    ]

    # Act / Assert
    with pytest.raises(AssertionError):
        verify_records(tmp_path, records)


def test_missing_custom_close_is_rejected(tmp_path) -> None:
    # Arrange
    records = [
        record
        for record in receipt_records()
        if not (record["kind"] == "ws-close" and record["workerIndex"] == "0")
    ]

    # Act / Assert
    with pytest.raises(AssertionError):
        verify_records(tmp_path, records)


def test_wrong_custom_close_reason_is_rejected(tmp_path) -> None:
    # Arrange
    records = receipt_records()
    target = next(record for record in records if record["kind"] == "ws-close")
    target["reason"] = "not the pinned Close reason"

    # Act / Assert
    with pytest.raises(AssertionError):
        verify_records(tmp_path, records)
