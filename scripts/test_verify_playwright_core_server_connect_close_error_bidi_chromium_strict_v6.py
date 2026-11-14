"""Pure controls for six official Bidi-config custom Close capture guards."""

from __future__ import annotations

import json

import pytest
import run_playwright_core_server_connect_close_error_bidi_chromium_strict_v6 as capture
import verify_playwright_core_server_connect_close_error_bidi_chromium_strict_v6 as verify

HOST = "example-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"
PROJECT = "bidi-chromium-library"


def report(
    statuses: tuple[str, str],
    workers: tuple[int, int] = (0, 0),
) -> dict:
    specs = []
    for index, (spec_id, status) in enumerate(
        zip(capture.PROJECTS[PROJECT]["specs"], statuses, strict=True)
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
                        "projectId": PROJECT,
                        "expectedStatus": "passed",
                        "status": "expected" if passed else "unexpected",
                        "results": [
                            {
                                "retry": 0,
                                "workerIndex": workers[index],
                                "startTime": f"2026-10-20T09:00:00.{index * 100:03d}Z",
                                "duration": 10,
                                "status": status,
                                "errors": (
                                    [] if passed else [{"message": "browserType.connect failed"}]
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


def selected(
    statuses: tuple[str, str] = ("passed", "passed"),
    workers: tuple[int, int] = (0, 0),
) -> list[dict]:
    return capture.selected_tests(report(statuses, workers), PROJECT)


def receipt_records(
    workers: tuple[int, int] = (0, 0),
    observed: tuple[int, ...] = (0, 1),
    mode: str = "oracle",
) -> list[dict]:
    parent = {
        "mode": mode,
        "sourceSha256": verify.SOURCE_SHA256,
        "adaptedSourceSha256": verify.ADAPTED_SOURCE_SHA256,
        "fixtureHost": HOST,
        "fixtureBind": BIND,
        "ownerBound": True if mode == "candidate" else None,
        "cdpEndpointPresent": False,
        "sourceAdapted": True,
        "fixtureAdapted": True,
    }
    records = [
        {
            **parent,
            "kind": "preload",
            "pid": 100,
            "workerIndex": None,
            "replacements": 1,
        },
        {
            **parent,
            "kind": "preload",
            "pid": 100,
            "workerIndex": None,
            "replacements": 1,
        },
        {
            **parent,
            "kind": "source-read",
            "pid": 100,
            "workerIndex": None,
            "replacements": 1,
        },
    ]
    started = set()
    for index, worker in enumerate(workers):
        pid = 200 + worker
        common = {**parent, "pid": pid, "workerIndex": str(worker)}
        if worker not in started:
            records.extend(
                [
                    {**common, "kind": "preload", "replacements": 1},
                    {**common, "kind": "source-read", "replacements": 1},
                ]
            )
            started.add(worker)
        if index not in observed:
            continue
        port = verify.SERVER_PORT_BASE + worker * verify.SERVER_PORT_STRIDE
        endpoint = {
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
                {**common, **endpoint, "kind": "ws-upgrade"},
                {**common, **endpoint, "kind": "ws-message", "messageBytes": 73},
                {
                    **common,
                    **endpoint,
                    "kind": "ws-close",
                    "code": 4123,
                    "reason": "Oh my!",
                },
            ]
        )
    return records


def check_records(
    tmp_path,
    records: list[dict],
    *,
    statuses: tuple[str, str] = ("passed", "passed"),
    workers: tuple[int, int] = (0, 0),
    mode: str = "oracle",
) -> int:
    receipt = tmp_path / "guard.jsonl"
    receipt.write_text("".join(json.dumps(record) + "\n" for record in records))
    return verify.checked_receipts(receipt, mode, selected(statuses, workers), HOST, BIND)


def test_pinned_source_manifest_and_frozen_snapshot_are_valid() -> None:
    # Arrange
    project_names = tuple(capture.PROJECTS)

    # Act
    capture.check_source_and_manifest()
    snapshot = capture.check_frozen_candidate()
    verify.check_inputs()

    # Assert
    assert len(project_names) == 3
    assert snapshot["candidateSourceSha256"] == capture.SNAPSHOT_SOURCE_SHA256
    assert capture.PROFILE == verify.PROFILE


def test_fixture_dns_preflight_accepts_matching_magicdns(monkeypatch) -> None:
    # Arrange
    def fake_check_output(command, *, text):
        assert text is True
        return (
            BIND + "\n"
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


def test_fixture_dns_preflight_rejects_wrong_address(monkeypatch) -> None:
    # Arrange
    monkeypatch.setattr(
        capture.subprocess,
        "check_output",
        lambda command, *, text: (
            BIND + "\n"
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


def test_all_six_ids_are_unique_and_cover_three_projects() -> None:
    # Arrange
    ids = [spec_id for selection in capture.PROJECTS.values() for spec_id in selection["specs"]]

    # Act
    kinds = [list(selection["specs"].values()) for selection in capture.PROJECTS.values()]

    # Assert
    assert len(ids) == len(set(ids)) == 6
    assert kinds == [["launchServer", "run-server"]] * 3


def test_both_selectors_accept_one_clean_pass_per_invocation() -> None:
    # Arrange
    raw = report(("passed", "passed"))

    # Act
    first = capture.selected_tests(raw, PROJECT)
    second = verify.selected_tests(raw, PROJECT)

    # Assert
    assert first == second
    assert [item["status"] for item in first] == ["expected", "expected"]


def test_candidate_red_and_worker_restart_are_preserved() -> None:
    # Arrange
    raw = report(("failed", "passed"), (0, 1))

    # Act
    first = capture.selected_tests(raw, PROJECT)
    second = verify.selected_tests(raw, PROJECT)

    # Assert
    assert first == second
    assert [item["status"] for item in first] == ["unexpected", "expected"]
    assert [item["attempts"][0]["workerIndex"] for item in first] == [0, 1]


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("line", 122),
        ("title", "wrong test"),
        ("column", 1),
    ],
)
def test_wrong_official_spec_is_rejected(field, value) -> None:
    # Arrange
    raw = report(("passed", "passed"))
    raw["suites"][0]["specs"][0][field] = value

    # Act / Assert
    with pytest.raises(AssertionError):
        verify.selected_tests(raw, PROJECT)


def test_duplicate_case_is_rejected() -> None:
    # Arrange
    raw = report(("passed", "passed"))
    raw["suites"][0]["specs"][1] = raw["suites"][0]["specs"][0]

    # Act / Assert
    with pytest.raises(AssertionError):
        capture.selected_tests(raw, PROJECT)


def test_retry_is_rejected() -> None:
    # Arrange
    raw = report(("passed", "passed"))
    raw["suites"][0]["specs"][0]["tests"][0]["results"][0]["retry"] = 1

    # Act / Assert
    with pytest.raises(AssertionError):
        verify.selected_tests(raw, PROJECT)


def test_skip_is_rejected() -> None:
    # Arrange
    raw = report(("passed", "passed"))
    raw["stats"]["skipped"] = 1

    # Act / Assert
    with pytest.raises(AssertionError):
        capture.selected_tests(raw, PROJECT)


def test_out_of_order_attempts_are_rejected() -> None:
    # Arrange
    raw = report(("passed", "passed"))
    raw["suites"][0]["specs"][0]["tests"][0]["results"][0]["startTime"] = "2026-10-20T09:00:02.000Z"

    # Act / Assert
    with pytest.raises(AssertionError):
        verify.selected_tests(raw, PROJECT)


def test_oracle_requires_two_real_ordered_close_triplets(tmp_path) -> None:
    # Arrange
    records = receipt_records()

    # Act
    count = check_records(tmp_path, records)

    # Assert
    assert count == 2


def test_worker_restart_and_duplicate_parent_preload_are_allowed(tmp_path) -> None:
    # Arrange
    records = receipt_records((0, 1))

    # Act
    count = check_records(tmp_path, records, workers=(0, 1))

    # Assert
    assert count == 2


def test_candidate_red_before_first_upgrade_is_truthfully_allowed(tmp_path) -> None:
    # Arrange
    records = receipt_records((0, 1), observed=(1,), mode="candidate")

    # Act
    count = check_records(
        tmp_path,
        records,
        statuses=("failed", "passed"),
        workers=(0, 1),
        mode="candidate",
    )

    # Assert
    assert count == 1


def test_candidate_two_red_cases_may_have_no_upgrade(tmp_path) -> None:
    # Arrange
    records = receipt_records(observed=(), mode="candidate")

    # Act
    count = check_records(
        tmp_path,
        records,
        statuses=("failed", "failed"),
        mode="candidate",
    )

    # Assert
    assert count == 0


def test_candidate_pass_without_any_upgrade_is_rejected(tmp_path) -> None:
    # Arrange
    records = receipt_records(observed=(), mode="candidate")

    # Act / Assert
    with pytest.raises(AssertionError):
        check_records(tmp_path, records, mode="candidate")


def test_candidate_pass_cannot_borrow_failed_workers_upgrade(tmp_path) -> None:
    # Arrange
    records = receipt_records((0, 1), observed=(0,), mode="candidate")

    # Act / Assert
    with pytest.raises(AssertionError):
        check_records(
            tmp_path,
            records,
            statuses=("failed", "passed"),
            workers=(0, 1),
            mode="candidate",
        )


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("cdpEndpointPresent", True),
        ("sourceAdapted", False),
        ("adaptedSourceSha256", "0" * 64),
        ("originalUrl", "ws://127.0.0.1:8907/ws"),
        ("mappedUrl", "ws://localhost:8907/ws"),
        ("port", 8911),
        ("listenerAddress", "127.0.0.1"),
        ("listenerFamily", "IPv6"),
        ("requestHost", "localhost:8907"),
        ("workerIndex", "2"),
    ],
)
def test_tampered_upgrade_is_rejected(tmp_path, field, value) -> None:
    # Arrange
    records = receipt_records()
    target = next(record for record in records if record["kind"] == "ws-upgrade")
    target[field] = value

    # Act / Assert
    with pytest.raises(AssertionError):
        check_records(tmp_path, records)


def test_missing_source_read_is_rejected(tmp_path) -> None:
    # Arrange
    records = [
        record
        for record in receipt_records()
        if not (record["kind"] == "source-read" and record["workerIndex"] == "0")
    ]

    # Act / Assert
    with pytest.raises(AssertionError):
        check_records(tmp_path, records)


def test_missing_close_is_rejected(tmp_path) -> None:
    # Arrange
    records = receipt_records()
    records.remove(next(record for record in records if record["kind"] == "ws-close"))

    # Act / Assert
    with pytest.raises(AssertionError):
        check_records(tmp_path, records)


def test_wrong_close_code_or_reason_is_rejected(tmp_path) -> None:
    # Arrange
    records = receipt_records()
    target = next(record for record in records if record["kind"] == "ws-close")
    target["code"] = 1006
    target["reason"] = ""

    # Act / Assert
    with pytest.raises(AssertionError):
        check_records(tmp_path, records)


def test_zero_byte_message_is_rejected(tmp_path) -> None:
    # Arrange
    records = receipt_records()
    target = next(record for record in records if record["kind"] == "ws-message")
    target["messageBytes"] = 0

    # Act / Assert
    with pytest.raises(AssertionError):
        check_records(tmp_path, records)


def test_extra_triplet_is_rejected(tmp_path) -> None:
    # Arrange
    records = receipt_records()
    records.extend(dict(record) for record in records[-3:])

    # Act / Assert
    with pytest.raises(AssertionError):
        check_records(tmp_path, records)


def test_out_of_order_ws_events_are_rejected(tmp_path) -> None:
    # Arrange
    records = receipt_records()
    first_close = next(
        index for index, record in enumerate(records) if record["kind"] == "ws-close"
    )
    records[first_close], records[first_close + 1] = (
        records[first_close + 1],
        records[first_close],
    )

    # Act / Assert
    with pytest.raises(AssertionError):
        check_records(tmp_path, records)


def test_project_receipt_worker_order_must_follow_report_order(tmp_path) -> None:
    # Arrange
    records = receipt_records((1, 0))

    # Act / Assert
    with pytest.raises(AssertionError):
        check_records(tmp_path, records, workers=(0, 1))
