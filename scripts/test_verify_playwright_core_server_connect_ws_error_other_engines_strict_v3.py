"""Mutation controls for the browser-free four-case Core WS-error capture."""

from __future__ import annotations

import json
from copy import deepcopy

import pytest

import run_playwright_core_server_connect_ws_error_other_engines_strict_v3 as capture
from verify_playwright_core_server_connect_ws_error_other_engines_strict_v3 import (
    checked_receipts,
)


def _report(project: str) -> dict:
    specs = []
    for spec_id in capture.PROJECTS[project]["specs"]:
        specs.append(
            {
                "id": spec_id,
                "file": "library/browsertype-connect.spec.ts",
                "line": 122,
                "column": 5,
                "title": "should print ws error",
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
    return {
        "errors": [],
        "suites": [{"specs": specs}],
        "stats": {"expected": 2, "unexpected": 0, "skipped": 0, "flaky": 0},
    }


def test_upstream_and_existing_harness_are_pinned() -> None:
    capture.check_browserless_source_and_manifest()


@pytest.mark.parametrize("project", list(capture.PROJECTS))
def test_exact_two_specs_per_project(project: str) -> None:
    selected = capture.selected_tests(_report(project), project)
    assert len(selected) == 2
    assert {item["specId"] for item in selected} == set(
        capture.PROJECTS[project]["specs"]
    )


def test_cross_project_id_is_rejected() -> None:
    report = _report("firefox-library")
    report["suites"][0]["specs"][0]["id"] = next(
        iter(capture.PROJECTS["webkit-library"]["specs"])
    )
    with pytest.raises(AssertionError):
        capture.selected_tests(report, "firefox-library")


@pytest.mark.parametrize("mutation", ["duplicate", "missing", "extra"])
def test_non_exact_selection_is_rejected(mutation: str) -> None:
    report = _report("firefox-library")
    specs = report["suites"][0]["specs"]
    if mutation == "duplicate":
        specs[1] = deepcopy(specs[0])
    elif mutation == "missing":
        specs.pop()
    else:
        specs.append(deepcopy(specs[0]))
    with pytest.raises(AssertionError):
        capture.selected_tests(report, "firefox-library")


@pytest.mark.parametrize(
    "field,value",
    [
        ("projectId", "webkit-library"),
        ("expectedStatus", "skipped"),
        ("status", "flaky"),
    ],
)
def test_report_identity_or_status_drift_is_rejected(field: str, value: str) -> None:
    report = _report("firefox-library")
    report["suites"][0]["specs"][0]["tests"][0][field] = value
    with pytest.raises(AssertionError):
        capture.selected_tests(report, "firefox-library")


def test_pass_wrapper_around_failed_attempt_is_rejected() -> None:
    report = _report("firefox-library")
    attempt = report["suites"][0]["specs"][0]["tests"][0]["results"][0]
    attempt["status"] = "failed"
    attempt["errors"] = [{"message": "failure"}]
    with pytest.raises(AssertionError):
        capture.selected_tests(report, "firefox-library")


def test_retry_is_rejected() -> None:
    report = _report("firefox-library")
    report["suites"][0]["specs"][0]["tests"][0]["results"][0]["retry"] = 1
    with pytest.raises(AssertionError):
        capture.selected_tests(report, "firefox-library")


def test_secret_in_public_error_is_rejected() -> None:
    report = _report("firefox-library")
    spec = report["suites"][0]["specs"][0]
    spec["ok"] = False
    test = spec["tests"][0]
    test["status"] = "unexpected"
    test["results"][0]["status"] = "failed"
    test["results"][0]["errors"] = [{"message": "secret=MYSECRET"}]
    report["stats"] = {"expected": 1, "unexpected": 1, "skipped": 0, "flaky": 0}
    with pytest.raises(AssertionError):
        capture.selected_tests(report, "firefox-library")


@pytest.mark.parametrize(
    "field,value",
    [
        ("mode", "oracle"),
        ("fixtureHost", "localhost"),
        ("fixtureBind", "127.0.0.1"),
        ("sourceSha256", "0" * 64),
        ("ownerBound", False),
        ("cdpEndpointPresent", True),
        ("sourceAdapted", True),
        ("workerIndex", None),
    ],
)
def test_guard_receipt_mutations_are_rejected(
    tmp_path, field: str, value: object
) -> None:
    selected = capture.selected_tests(_report("firefox-library"), "firefox-library")
    host = "host-yuacx.tail8f3b66.ts.net"
    bind = "100.101.102.103"
    record = {
        "pid": 123,
        "workerIndex": "0",
        "mode": "candidate",
        "sourceSha256": capture.SOURCE_SHA256,
        "fixtureHost": host,
        "fixtureBind": bind,
        "ownerBound": True,
        "cdpEndpointPresent": False,
        "sourceAdapted": False,
    }
    record[field] = value
    receipt = tmp_path / "receipt.jsonl"
    receipt.write_text(json.dumps(record) + "\n")
    with pytest.raises(AssertionError):
        checked_receipts(receipt, "candidate", selected, host, bind)


def test_valid_guard_receipt_has_matching_worker(tmp_path) -> None:
    selected = capture.selected_tests(_report("webkit-library"), "webkit-library")
    record = {
        "pid": 123,
        "workerIndex": "0",
        "mode": "candidate",
        "sourceSha256": capture.SOURCE_SHA256,
        "fixtureHost": "host-yuacx.tail8f3b66.ts.net",
        "fixtureBind": "100.101.102.103",
        "ownerBound": True,
        "cdpEndpointPresent": False,
        "sourceAdapted": False,
    }
    receipt = tmp_path / "receipt.jsonl"
    receipt.write_text(json.dumps(record) + "\n")
    checked_receipts(
        receipt, "candidate", selected, record["fixtureHost"], record["fixtureBind"]
    )
