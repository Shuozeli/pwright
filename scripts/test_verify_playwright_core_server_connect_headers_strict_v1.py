"""Negative controls for the versioned Core connect-header diagnostic."""

from __future__ import annotations

import copy
import json

import pytest
import run_playwright_core_server_connect_headers_strict_v1 as capture
import verify_playwright_core_server_connect_headers_strict_v1 as verify

HOST = "example-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"
DEFAULT_UA = "Playwright/1.58.0 (arm64; macOS 26.6) node/26.7"
ADAPTED_DIGEST = "4cbc9359b8fd5cf2855ab1fa763da7af510c5075421a6c17cf31c50e6a8321b6"


def report(
    statuses: tuple[str, str, str, str] = (
        "passed",
        "passed",
        "passed",
        "passed",
    ),
) -> dict:
    suites: dict[str, list[dict]] = {"launchServer": [], "run-server": []}
    for worker, (spec_id, identity) in enumerate(capture.EXPECTED_SPEC_IDS.items()):
        kind, header_case, line, title = identity
        status = statuses[worker]
        passed = status == "passed"
        suites[kind].append(
            {
                "id": spec_id,
                "file": "library/browsertype-connect.spec.ts",
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
                                "workerIndex": worker,
                                "status": status,
                                "errors": []
                                if passed
                                else [{"message": f"{header_case} header mismatch"}],
                            }
                        ],
                    }
                ],
            }
        )
    return {
        "errors": [],
        "suites": [{"specs": specs, "suites": []} for specs in suites.values()],
        "stats": {
            "expected": statuses.count("passed"),
            "unexpected": statuses.count("failed"),
            "skipped": 0,
            "flaky": 0,
        },
    }


def records(mode: str = "oracle") -> list[dict]:
    result = []
    for worker, (_, (_, header_case, _, _)) in enumerate(
        capture.EXPECTED_SPEC_IDS.items()
    ):
        common = {
            "pid": 100 + worker,
            "workerIndex": str(worker),
            "mode": mode,
            "fixtureHost": HOST,
            "fixtureBind": BIND,
            "ownerBound": True if mode == "candidate" else None,
            "cdpEndpointPresent": False,
        }
        result.append(
            {
                **common,
                "event": "source-adapted",
                "sourceSha256": capture.SOURCE_SHA256,
                "adaptedSha256": ADAPTED_DIGEST,
                "replacements": 2,
            }
        )
        headers = {
            "host": f"{HOST}:{8907 + worker * 4}",
            "user-agent": "Playwright" if header_case == "extra" else DEFAULT_UA,
            "foo": "bar",
            "upgrade": "websocket",
            "sec-websocket-key": "AAAAAAAAAAAAAAAAAAAAAA==",
            "sec-websocket-version": "13",
        }
        if header_case == "default":
            headers["x-playwright-browser"] = "chromium"
        asserted_names = ("host", "user-agent", "foo", "x-playwright-browser")
        asserted = {name: headers.get(name) for name in asserted_names}
        raw_asserted = {
            name: [] if headers.get(name) is None else [headers[name]]
            for name in asserted_names
        }
        result.append(
            {
                **common,
                "event": "fixture-upgrade-received",
                "upgradeOrdinal": 1,
                "method": "GET",
                "url": "/ws",
                "localAddress": BIND,
                "localPort": 8907 + worker * 4,
                "assertedHeaders": asserted,
                "rawAssertedHeaders": raw_asserted,
                "protocolHeaderCounts": {
                    "connection": 1,
                    "upgrade": 1,
                    "sec-websocket-version": 1,
                    "sec-websocket-key": 1,
                },
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
        HOST,
        BIND,
        DEFAULT_UA,
        ADAPTED_DIGEST,
    )


def test_local_pinned_source_assertions_and_manifest_are_unchanged() -> None:
    # Arrange
    source = capture.UPSTREAM / capture.SOURCE

    # Act
    capture.check_source_and_manifest()

    # Assert
    assert source.is_file()
    assert capture.adapted_source_sha256() == ADAPTED_DIGEST


def test_exact_four_official_cases_are_selected() -> None:
    # Arrange
    raw = report()

    # Act
    selected = capture.selected_tests(raw)

    # Assert
    assert len(selected) == 4
    assert {item["specId"] for item in selected} == set(capture.EXPECTED_SPEC_IDS)
    assert {item["headerCase"] for item in selected} == {"extra", "default"}


def test_candidate_header_failure_stays_unexpected() -> None:
    # Arrange
    raw = report(("passed", "failed", "passed", "passed"))

    # Act
    selected = capture.selected_tests(raw)

    # Assert
    assert len([item for item in selected if item["status"] == "unexpected"]) == 1


@pytest.mark.parametrize(
    "change",
    ["duplicate", "wrong-line", "skip", "retry", "wrong-project", "wrong-title"],
)
def test_report_identity_or_outcome_drift_is_rejected(change: str) -> None:
    # Arrange
    raw = report()
    first = raw["suites"][0]["specs"][0]
    if change == "duplicate":
        raw["suites"][0]["specs"][1] = copy.deepcopy(first)
    elif change == "wrong-line":
        first["line"] = 236
    elif change == "skip":
        raw["stats"]["skipped"] = 1
    elif change == "retry":
        first["tests"][0]["results"][0]["retry"] = 1
    elif change == "wrong-project":
        first["tests"][0]["projectId"] = "webkit-library"
    else:
        first["title"] = "different assertion"

    # Act / Assert
    with pytest.raises(AssertionError):
        capture.selected_tests(raw)


def test_real_upgrade_receipts_are_accepted(tmp_path) -> None:
    # Arrange
    value = records("candidate")

    # Act
    check_records(tmp_path, value, "candidate")

    # Assert
    assert len(value) == 8


@pytest.mark.parametrize(
    "change",
    [
        "missing-source",
        "missing-upgrade",
        "duplicate-upgrade",
        "wrong-adapted-digest",
        "wrong-worker",
        "wrong-local-address",
        "wrong-local-port",
        "wrong-host",
        "wrong-default-user-agent",
        "reordered-upgrades",
        "missing-browser-header",
        "wrong-foo",
        "raw-header-mismatch",
        "missing-protocol-header",
        "leaked-unrelated-header",
        "cdp-present",
        "wrong-candidate-owner",
    ],
)
def test_guard_rejects_forged_or_incomplete_receipt(change: str, tmp_path) -> None:
    # Arrange
    value = records("candidate")
    upgrade = value[3]
    if change == "missing-source":
        del value[2]
    elif change == "missing-upgrade":
        del value[3]
    elif change == "duplicate-upgrade":
        value.append(copy.deepcopy(upgrade))
    elif change == "wrong-adapted-digest":
        value[2]["adaptedSha256"] = "0" * 64
    elif change == "wrong-worker":
        upgrade["workerIndex"] = "99"
    elif change == "wrong-local-address":
        upgrade["localAddress"] = "127.0.0.1"
    elif change == "wrong-local-port":
        upgrade["localPort"] += 1
    elif change == "wrong-host":
        upgrade["assertedHeaders"]["host"] = "localhost:8911"
    elif change == "wrong-default-user-agent":
        upgrade["assertedHeaders"]["user-agent"] = "other"
        upgrade["rawAssertedHeaders"]["user-agent"] = ["other"]
    elif change == "reordered-upgrades":
        value[1], value[3] = value[3], value[1]
    elif change == "missing-browser-header":
        upgrade["assertedHeaders"]["x-playwright-browser"] = None
        upgrade["rawAssertedHeaders"]["x-playwright-browser"] = []
    elif change == "wrong-foo":
        upgrade["assertedHeaders"]["foo"] = "wrong"
    elif change == "raw-header-mismatch":
        upgrade["rawAssertedHeaders"]["user-agent"] = ["wrong"]
    elif change == "missing-protocol-header":
        upgrade["protocolHeaderCounts"]["sec-websocket-key"] = 0
    elif change == "leaked-unrelated-header":
        upgrade["assertedHeaders"]["sec-websocket-key"] = "secret"
    elif change == "cdp-present":
        upgrade["cdpEndpointPresent"] = True
    else:
        upgrade["ownerBound"] = False

    # Act / Assert
    with pytest.raises(AssertionError):
        check_records(tmp_path, value, "candidate")
