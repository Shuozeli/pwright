"""Pure mutation controls for the five-case Bidi timeout/redirect verifier."""

from __future__ import annotations

from copy import deepcopy

import pytest
import verify_playwright_core_server_connect_timeout_redirect_bidi_chromium_strict_v1 as verifier

HOST = "macbookpro-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"


def report(*, failed: str | None = None) -> dict:
    by_kind: dict[str, list[dict]] = {
        "launchServer": [],
        "run-server": [],
        "standalone": [],
    }
    for spec_id, (line, kind, title) in verifier.TARGETS.items():
        passed = spec_id != failed
        by_kind[kind].append(
            {
                "id": spec_id,
                "file": "library/browsertype-connect.spec.ts",
                "line": line,
                "column": 1 if line == 1056 else 5,
                "title": title,
                "ok": passed,
                "tests": [
                    {
                        "projectId": verifier.PROJECT,
                        "projectName": verifier.PROJECT,
                        "expectedStatus": "passed",
                        "status": "expected" if passed else "unexpected",
                        "results": [
                            {
                                "retry": 0,
                                "status": "passed" if passed else "failed",
                                "workerIndex": 0,
                                "errors": []
                                if passed
                                else [{"message": "Timeout assertion failed"}],
                            }
                        ],
                    }
                ],
            }
        )
    return {
        "errors": [],
        "suites": [
            {"title": "launchServer", "specs": by_kind["launchServer"]},
            {"title": "run-server", "specs": by_kind["run-server"]},
            {"title": "top-level", "specs": by_kind["standalone"]},
        ],
        "stats": {
            "expected": 5 if failed is None else 4,
            "unexpected": 0 if failed is None else 1,
            "skipped": 0,
            "flaky": 0,
        },
    }


def selected(*, failed: str | None = None) -> list[dict]:
    return verifier.selected_from_report(report(failed=failed), mode="candidate")


def receipt_event(event: str, **extra: object) -> dict:
    return {
        "event": event,
        "pid": 1234,
        "workerIndex": "0",
        "mode": "candidate",
        "fixtureHost": HOST,
        "fixtureBind": BIND,
        "ownerBound": True,
        "cdpEndpointPresent": False,
        **extra,
    }


def receipts() -> list[dict]:
    return [
        receipt_event(
            "source-verified",
            sourceSha256=verifier.SOURCE_SHA256,
            fixtureSha256=verifier.UPSTREAM_INPUTS["tests/config/testserver/index.ts"],
        ),
        receipt_event(
            "source-adapted",
            sourceSha256=verifier.SOURCE_SHA256,
            adaptedSha256=verifier.ADAPTED_SOURCE_SHA256,
            replacements=2,
        ),
        receipt_event(
            "fixture-upgrade-request",
            requestPath="/ws-slow",
            requestHost=f"{HOST}:8907",
        ),
        receipt_event(
            "fixture-upgrade-request",
            requestPath="/ws-slow",
            requestHost=f"{HOST}:8907",
        ),
        receipt_event("fixture-upgrade-request", requestPath="/ws", requestHost=f"{HOST}:8907"),
        receipt_event("fixture-upgrade-request", requestPath="/ws", requestHost=f"{HOST}:8907"),
        receipt_event(
            "fixture-request",
            requestId=1,
            requestPath="/connect/json",
            requestHost=f"{HOST}:8907",
            requestMethod="GET",
        ),
        receipt_event("redirect-response", requestId=1, status=302, location="/connect/slow"),
        receipt_event(
            "fixture-request",
            requestId=2,
            requestPath="/connect/slow",
            requestHost=f"{HOST}:8907",
            requestMethod="GET",
        ),
        receipt_event("slow-socket-closed", requestId=2),
    ]


def strict_payload(mode: str, cases: list[dict]) -> dict:
    output = verifier.paths(mode)
    candidate = mode == "candidate"
    return {
        "schema": 1,
        "scope": verifier.SCOPE,
        "sourceClassification": "implementation-internal",
        "fixtureAdapted": True,
        "fixtureAdaptation": verifier.FIXTURE_ADAPTATION,
        "publicApiParityEvidence": False,
        "promotedToCoreLedger": False,
        "realBrowserServer": False,
        "mode": mode,
        "upstreamCommit": verifier.COMMIT,
        "upstreamInputs": verifier.UPSTREAM_INPUTS,
        "inputs": {},
        "candidateProfile": verifier.PROFILE if candidate else None,
        "candidateArtifactSha256": verifier.ARTIFACT_SHA256 if candidate else None,
        "candidateSnapshot": str(verifier.SNAPSHOT) if candidate else None,
        "candidateSnapshotManifestSha256": verifier.SNAPSHOT_MANIFEST_SHA256 if candidate else None,
        "candidateSourceSha256": verifier.CANDIDATE_SOURCE_SHA256 if candidate else None,
        "run": {
            "suite": verifier.SUITE,
            "project": verifier.PROJECT,
            "result": str(output["result"]),
            "resultSha256": "0" * 64,
            "provenanceSha256": "0" * 64,
            "guardReceiptSha256": "0" * 64,
            "logSha256": "0" * 64,
            "exitCode": 0 if all(item["status"] == "expected" for item in cases) else 1,
            "selected": cases,
        },
    }


def test_exact_five_official_invocations_accepted() -> None:
    # Arrange
    oracle_report = report()

    # Act
    selected_cases = verifier.selected_from_report(oracle_report, mode="oracle")

    # Assert
    assert len(selected_cases) == 5
    assert {case["specId"] for case in selected_cases} == set(verifier.TARGETS)
    assert {case["line"] for case in selected_cases} == {221, 228, 1056}


def test_truthful_candidate_red_accepted_but_oracle_red_rejected() -> None:
    # Arrange
    failed = next(iter(verifier.TARGETS))
    candidate_report = report(failed=failed)

    # Act
    red = verifier.selected_from_report(candidate_report, mode="candidate")

    # Assert
    assert sum(case["status"] == "unexpected" for case in red) == 1
    with pytest.raises(ValueError, match="oracle has a failed"):
        verifier.selected_from_report(candidate_report, mode="oracle")


@pytest.mark.parametrize(
    ("field", "replacement"),
    [
        ("id", "wrong-id"),
        ("line", 222),
        ("column", 1),
        ("title", "different assertion"),
    ],
)
def test_timeout_spec_identity_mutations_rejected(field: str, replacement: object) -> None:
    # Arrange
    changed = report()
    changed["suites"][0]["specs"][0][field] = replacement

    # Act / Assert
    with pytest.raises(ValueError, match="selected spec"):
        verifier.selected_from_report(changed, mode="candidate")


def test_redirect_column_one_is_required() -> None:
    # Arrange
    changed = report()
    changed["suites"][2]["specs"][0]["column"] = 5

    # Act / Assert
    with pytest.raises(ValueError, match="selected spec source"):
        verifier.selected_from_report(changed, mode="candidate")


def test_kind_substitution_rejected() -> None:
    # Arrange
    changed = report()
    changed["suites"][0]["title"] = "standalone"

    # Act / Assert
    with pytest.raises(ValueError, match="selected spec source"):
        verifier.selected_from_report(changed, mode="candidate")


@pytest.mark.parametrize("mutation", ["duplicate", "missing", "extra"])
def test_target_count_mutations_rejected(mutation: str) -> None:
    # Arrange
    changed = report()
    specs = changed["suites"][0]["specs"]
    if mutation == "duplicate":
        specs[1] = deepcopy(specs[0])
    elif mutation == "missing":
        specs.pop()
    else:
        specs.append(deepcopy(specs[0]))

    # Act / Assert
    with pytest.raises(ValueError, match="selected spec|exactly five"):
        verifier.selected_from_report(changed, mode="candidate")


@pytest.mark.parametrize(
    ("field", "replacement"),
    [
        ("projectId", "chromium-library"),
        ("projectName", "chromium-library"),
        ("expectedStatus", "skipped"),
    ],
)
def test_project_or_expected_status_mutation_rejected(field: str, replacement: str) -> None:
    # Arrange
    changed = report()
    changed["suites"][0]["specs"][0]["tests"][0][field] = replacement

    # Act / Assert
    with pytest.raises(ValueError, match="selected test project"):
        verifier.selected_from_report(changed, mode="candidate")


def test_retry_and_false_pass_rejected() -> None:
    # Arrange
    retry = report()
    retry["suites"][0]["specs"][0]["tests"][0]["results"][0]["retry"] = 1
    false_pass = report()
    attempt = false_pass["suites"][0]["specs"][0]["tests"][0]["results"][0]
    attempt["status"] = "failed"
    attempt["errors"] = [{"message": "failed"}]

    # Act / Assert
    with pytest.raises(ValueError, match="attempt is malformed"):
        verifier.selected_from_report(retry, mode="candidate")
    with pytest.raises(ValueError, match="attempt/status/errors disagree"):
        verifier.selected_from_report(false_pass, mode="candidate")


def test_worker_index_outside_five_case_bound_rejected() -> None:
    # Arrange
    changed = report()
    changed["suites"][1]["specs"][0]["tests"][0]["results"][0]["workerIndex"] = 5

    # Act / Assert
    with pytest.raises(ValueError, match="attempt is malformed"):
        verifier.selected_from_report(changed, mode="candidate")


@pytest.mark.parametrize("worker_index", [1, 4])
def test_bounded_restarted_worker_and_derived_port_accepted(worker_index: int) -> None:
    # Arrange
    changed = report()
    changed["suites"][0]["specs"][0]["tests"][0]["results"][0]["workerIndex"] = worker_index
    cases = verifier.selected_from_report(changed, mode="candidate")
    events = receipts()
    events.extend(
        [
            {**events[0], "workerIndex": str(worker_index)},
            {**events[1], "workerIndex": str(worker_index)},
        ]
    )
    events[2]["workerIndex"] = str(worker_index)
    events[2]["requestHost"] = f"{HOST}:{8907 + 4 * worker_index}"

    # Act
    counts = verifier.checked_receipts(
        events, mode="candidate", selected=cases, host=HOST, bind=BIND
    )

    # Assert
    assert counts["/ws-slow"] == 2


def test_restarted_worker_wrong_derived_port_rejected() -> None:
    # Arrange
    changed = report()
    changed["suites"][0]["specs"][0]["tests"][0]["results"][0]["workerIndex"] = 1
    cases = verifier.selected_from_report(changed, mode="candidate")
    events = receipts()
    events.extend([{**events[0], "workerIndex": "1"}, {**events[1], "workerIndex": "1"}])
    events[2]["workerIndex"] = "1"

    # Act / Assert
    with pytest.raises(ValueError, match="upgrade request host"):
        verifier.checked_receipts(events, mode="candidate", selected=cases, host=HOST, bind=BIND)


def test_redirect_receipts_use_restarted_worker_port() -> None:
    # Arrange
    changed = report()
    changed["suites"][2]["specs"][0]["tests"][0]["results"][0]["workerIndex"] = 1
    cases = verifier.selected_from_report(changed, mode="candidate")
    events = receipts()
    events.extend([{**events[0], "workerIndex": "1"}, {**events[1], "workerIndex": "1"}])
    for event in events:
        if event["event"] in {"fixture-request", "redirect-response", "slow-socket-closed"}:
            event["workerIndex"] = "1"
            if event["event"] == "fixture-request":
                event["requestHost"] = f"{HOST}:8911"

    # Act
    counts = verifier.checked_receipts(
        events, mode="candidate", selected=cases, host=HOST, bind=BIND
    )

    # Assert
    assert counts["slow-socket-closed"] == 1


def test_truthful_red_with_worker_restart_is_accepted() -> None:
    # Arrange
    failed = next(iter(verifier.TARGETS))
    changed = report(failed=failed)
    for suite in changed["suites"]:
        for spec in suite["specs"]:
            if spec["id"] != failed:
                spec["tests"][0]["results"][0]["workerIndex"] = 1
    cases = verifier.selected_from_report(changed, mode="candidate")
    events = receipts()
    events.extend([{**events[0], "workerIndex": "1"}, {**events[1], "workerIndex": "1"}])
    for event in events[3:10]:
        event["workerIndex"] = "1"
        if "requestHost" in event:
            event["requestHost"] = f"{HOST}:8911"

    # Act
    counts = verifier.checked_receipts(
        events, mode="candidate", selected=cases, host=HOST, bind=BIND
    )

    # Assert
    assert sum(case["status"] == "unexpected" for case in cases) == 1
    assert counts["/ws-slow"] == 2


def test_report_stats_mutation_rejected() -> None:
    # Arrange
    changed = report()
    changed["stats"]["expected"] = 6

    # Act / Assert
    with pytest.raises(ValueError, match="report stats"):
        verifier.selected_from_report(changed, mode="candidate")


def test_exact_composed_receipt_accepted() -> None:
    # Arrange
    events = receipts()

    # Act
    counts = verifier.checked_receipts(
        events, mode="candidate", selected=selected(), host=HOST, bind=BIND
    )

    # Assert
    assert counts["/ws"] == counts["/ws-slow"] == 2
    assert counts["redirect-response"] == counts["slow-socket-closed"] == 1


@pytest.mark.parametrize(
    ("index", "field", "replacement"),
    [
        (0, "fixtureSha256", "0" * 64),
        (1, "adaptedSha256", "0" * 64),
        (1, "replacements", 1),
        (2, "requestHost", "localhost:3000"),
        (2, "requestHost", f"{HOST}:8908"),
        (6, "requestHost", f"{HOST}:8911"),
        (7, "location", "/wrong"),
        (9, "requestId", 999),
        (2, "ownerBound", False),
        (2, "cdpEndpointPresent", True),
        (2, "workerIndex", "1"),
    ],
)
def test_receipt_mutations_rejected(index: int, field: str, replacement: object) -> None:
    # Arrange
    changed = receipts()
    changed[index][field] = replacement

    # Act / Assert
    with pytest.raises(ValueError):
        verifier.checked_receipts(
            changed, mode="candidate", selected=selected(), host=HOST, bind=BIND
        )


def test_missing_redirect_socket_close_rejected_for_pass() -> None:
    # Arrange
    changed = receipts()[:-1]

    # Act / Assert
    with pytest.raises(ValueError, match="slow-socket-closed"):
        verifier.checked_receipts(
            changed, mode="candidate", selected=selected(), host=HOST, bind=BIND
        )


def test_truthful_red_receipt_may_have_no_failed_case_upgrade() -> None:
    # Arrange
    failed = next(spec_id for spec_id, target in verifier.TARGETS.items() if target[0] == 221)
    changed = receipts()
    changed.pop(2)

    # Act
    counts = verifier.checked_receipts(
        changed,
        mode="candidate",
        selected=selected(failed=failed),
        host=HOST,
        bind=BIND,
    )

    # Assert
    assert counts["/ws-slow"] == 1


@pytest.mark.parametrize(
    ("field", "replacement"),
    [
        ("scope", "wrong-scope"),
        ("fixtureAdapted", False),
        ("publicApiParityEvidence", True),
        ("promotedToCoreLedger", True),
        ("realBrowserServer", True),
        ("upstreamCommit", "0" * 40),
        ("candidateSourceSha256", "0" * 64),
    ],
)
def test_strict_header_mutations_rejected(field: str, replacement: object) -> None:
    # Arrange
    cases = selected()
    changed = strict_payload("candidate", cases)
    changed[field] = replacement

    # Act / Assert
    with pytest.raises(ValueError, match="strict"):
        verifier.verify_strict_payload(changed, mode="candidate", selected=cases)


def test_truthful_red_strict_envelope_accepted() -> None:
    # Arrange
    failed = next(iter(verifier.TARGETS))
    cases = selected(failed=failed)
    envelope = strict_payload("candidate", cases)

    # Act
    verifier.verify_strict_payload(envelope, mode="candidate", selected=cases)

    # Assert
    assert envelope["run"]["exitCode"] == 1


def test_false_green_exit_code_rejected() -> None:
    # Arrange
    failed = next(iter(verifier.TARGETS))
    cases = selected(failed=failed)
    changed = strict_payload("candidate", cases)
    changed["run"]["exitCode"] = 0

    # Act / Assert
    with pytest.raises(ValueError, match="strict run"):
        verifier.verify_strict_payload(changed, mode="candidate", selected=cases)
