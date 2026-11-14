"""Pure mutation controls for six browserless Bidi HTTP-401 invocations."""

from __future__ import annotations

import json
from copy import deepcopy
from pathlib import Path

import pytest
import run_playwright_core_server_connect_http_401_bidi_chromium_strict_v6 as capture
import verify_playwright_core_server_connect_http_401_bidi_chromium_strict_v6 as verify


def report(project: str) -> dict:
    groups = []
    for kind in ("launchServer", "run-server"):
        spec_id = next(
            spec_id
            for spec_id, value in capture.PROJECTS[project]["specs"].items()
            if value == kind
        )
        groups.append(
            {
                "title": kind,
                "file": "library/browsertype-connect.spec.ts",
                "specs": [
                    {
                        "id": spec_id,
                        "file": "library/browsertype-connect.spec.ts",
                        "line": 116,
                        "column": 5,
                        "title": "should print HTTP error",
                        "ok": True,
                        "tests": [
                            {
                                "projectId": project,
                                "projectName": project,
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
                ],
            }
        )
    return {
        "errors": [],
        "suites": [
            {
                "title": "library/browsertype-connect.spec.ts",
                "file": "library/browsertype-connect.spec.ts",
                "specs": [],
                "suites": groups,
            }
        ],
        "stats": {"expected": 2, "unexpected": 0, "skipped": 0, "flaky": 0},
    }


def selected(project: str) -> list[dict]:
    return verify.selected_tests(report(project), project)


def test_pinned_upstream_source_manifest_and_frozen_snapshot() -> None:
    capture.check_browserless_source_and_manifest()
    snapshot = capture.checked_snapshot()
    assert snapshot["profile"] == capture.PROFILE
    assert snapshot["artifactSha256"] == capture.ARTIFACT_SHA256


@pytest.mark.parametrize("project", list(capture.PROJECTS))
def test_each_project_has_exact_expanded_ids(project: str) -> None:
    raw = report(project)
    expected = capture.selected_tests(raw, project)
    observed = verify.selected_tests(raw, project)
    assert expected == observed
    assert [item["kind"] for item in observed] == ["launchServer", "run-server"]
    assert {item["specId"] for item in observed} == set(capture.PROJECTS[project]["specs"])


@pytest.mark.parametrize(
    "mutation",
    [
        "root-title",
        "root-file",
        "root-spec",
        "missing-group",
        "extra-group",
        "swapped-group",
        "group-file",
        "group-nesting",
        "wrong-group-id",
        "cross-project-id",
        "duplicate-id",
        "missing-spec",
        "extra-spec",
    ],
)
def test_suite_and_id_mutations_rejected(mutation: str) -> None:
    project = "bidi-chromium-library"
    raw = report(project)
    root = raw["suites"][0]
    groups = root["suites"]
    if mutation == "root-title":
        root["title"] = "library/other.spec.ts"
    elif mutation == "root-file":
        root["file"] = "library/other.spec.ts"
    elif mutation == "root-spec":
        root["specs"] = [deepcopy(groups[0]["specs"][0])]
    elif mutation == "missing-group":
        groups.pop()
    elif mutation == "extra-group":
        groups.append(deepcopy(groups[1]))
    elif mutation == "swapped-group":
        groups.reverse()
    elif mutation == "group-file":
        groups[0]["file"] = "library/other.spec.ts"
    elif mutation == "group-nesting":
        groups[0]["suites"] = [deepcopy(groups[0])]
    elif mutation == "wrong-group-id":
        groups[0]["specs"][0]["id"] = groups[1]["specs"][0]["id"]
    elif mutation == "cross-project-id":
        groups[0]["specs"][0]["id"] = next(
            iter(capture.PROJECTS["bidi-chrome-stable-library"]["specs"])
        )
    elif mutation == "duplicate-id":
        groups[1]["specs"][0]["id"] = groups[0]["specs"][0]["id"]
    elif mutation == "missing-spec":
        groups[0]["specs"].clear()
    else:
        groups[0]["specs"].append(deepcopy(groups[0]["specs"][0]))
    with pytest.raises(AssertionError):
        capture.selected_tests(raw, project)
    with pytest.raises(AssertionError):
        verify.selected_tests(raw, project)


@pytest.mark.parametrize(
    "field,value",
    [
        ("projectId", "chromium-library"),
        ("projectName", "chromium-library"),
        ("expectedStatus", "skipped"),
        ("status", "flaky"),
    ],
)
def test_test_identity_mutations_rejected(field: str, value: str) -> None:
    project = "bidi-chromium-library"
    raw = report(project)
    raw["suites"][0]["suites"][0]["specs"][0]["tests"][0][field] = value
    with pytest.raises(AssertionError):
        capture.selected_tests(raw, project)
    with pytest.raises(AssertionError):
        verify.selected_tests(raw, project)


@pytest.mark.parametrize(
    "field,value",
    [
        ("retry", 1),
        ("status", "failed"),
        ("workerIndex", -1),
        ("workerIndex", 2),
        ("workerIndex", "0"),
        ("workerIndex", True),
    ],
)
def test_attempt_mutations_rejected(field: str, value: object) -> None:
    project = "bidi-chromium-library"
    raw = report(project)
    attempt = raw["suites"][0]["suites"][0]["specs"][0]["tests"][0]["results"][0]
    attempt[field] = value
    with pytest.raises(AssertionError):
        capture.selected_tests(raw, project)
    with pytest.raises(AssertionError):
        verify.selected_tests(raw, project)


@pytest.mark.parametrize("value", [{"message": ""}, {"message": 1}, "bad"])
def test_failed_error_message_shape_rejected(value: object) -> None:
    project = "bidi-chromium-library"
    raw = report(project)
    spec = raw["suites"][0]["suites"][0]["specs"][0]
    spec["ok"] = False
    test = spec["tests"][0]
    test["status"] = "unexpected"
    attempt = test["results"][0]
    attempt["status"] = "failed"
    attempt["errors"] = [value]
    raw["stats"]["expected"] = 1
    raw["stats"]["unexpected"] = 1
    with pytest.raises(AssertionError):
        capture.selected_tests(raw, project)
    with pytest.raises(AssertionError):
        verify.selected_tests(raw, project)


def red_report() -> dict:
    project = "bidi-chromium-library"
    raw = report(project)
    spec = raw["suites"][0]["suites"][0]["specs"][0]
    spec["ok"] = False
    test = spec["tests"][0]
    test["status"] = "unexpected"
    attempt = test["results"][0]
    attempt["status"] = "failed"
    attempt["errors"] = [{"message": "401 mismatch"}]
    raw["stats"]["expected"] = 1
    raw["stats"]["unexpected"] = 1
    return raw


def test_well_formed_red_candidate_is_preserved() -> None:
    project = "bidi-chromium-library"
    raw = red_report()
    assert capture.selected_tests(raw, project) == verify.selected_tests(raw, project)


def receipts() -> list[dict]:
    base = {
        "pid": 123,
        "workerIndex": "0",
        "mode": "candidate",
        "fixtureHost": "host-yuacx.tail8f3b66.ts.net",
        "fixtureBind": "100.101.102.103",
        "ownerBound": True,
        "cdpEndpointPresent": False,
    }
    return [
        {
            **base,
            "event": "source-adapted",
            "sourceSha256": capture.SOURCE_SHA256,
            "adaptedSha256": capture.ADAPTED_SOURCE_SHA256,
            "replacements": 1,
        },
        {**base, "event": "fixture-401-sent", "responseSha256": verify.RESPONSE_SHA256},
        {**base, "event": "fixture-401-sent", "responseSha256": verify.RESPONSE_SHA256},
    ]


def write_receipts(path: Path, records: list[dict]) -> None:
    path.write_text("".join(json.dumps(record) + "\n" for record in records))


def test_valid_receipt_counts_two_real_fixture_responses(tmp_path: Path) -> None:
    records = receipts()
    path = tmp_path / "receipt.jsonl"
    write_receipts(path, records)
    verify.checked_receipts(
        path,
        "candidate",
        selected("bidi-chromium-library"),
        records[0]["fixtureHost"],
        records[0]["fixtureBind"],
    )


def test_red_candidate_can_lack_one_fixture_response(tmp_path: Path) -> None:
    records = receipts()[:-1]
    path = tmp_path / "receipt.jsonl"
    write_receipts(path, records)
    verify.checked_receipts(
        path,
        "candidate",
        verify.selected_tests(red_report(), "bidi-chromium-library"),
        records[0]["fixtureHost"],
        records[0]["fixtureBind"],
    )


@pytest.mark.parametrize("mutation", ["zero-response", "extra-response", "unknown-worker"])
def test_red_candidate_response_bounds_rejected(tmp_path: Path, mutation: str) -> None:
    records = receipts()[:-1]
    if mutation == "zero-response":
        records.pop()
    elif mutation == "extra-response":
        records.extend(deepcopy(records[-1]) for _ in range(2))
    else:
        records[-1]["workerIndex"] = "1"
    path = tmp_path / "receipt.jsonl"
    write_receipts(path, records)
    with pytest.raises(AssertionError):
        verify.checked_receipts(
            path,
            "candidate",
            verify.selected_tests(red_report(), "bidi-chromium-library"),
            "host-yuacx.tail8f3b66.ts.net",
            "100.101.102.103",
        )


@pytest.mark.parametrize(
    "event,index,field,value",
    [
        ("source", 0, "mode", "oracle"),
        ("source", 0, "fixtureHost", "localhost"),
        ("source", 0, "fixtureBind", "127.0.0.1"),
        ("source", 0, "ownerBound", False),
        ("source", 0, "cdpEndpointPresent", True),
        ("source", 0, "sourceSha256", "0" * 64),
        ("source", 0, "adaptedSha256", "0" * 64),
        ("source", 0, "replacements", 2),
        ("source", 0, "workerIndex", "1"),
        ("response", 1, "responseSha256", "0" * 64),
        ("response", 1, "workerIndex", "1"),
    ],
)
def test_receipt_mutations_rejected(
    tmp_path: Path, event: str, index: int, field: str, value: object
) -> None:
    records = receipts()
    assert records[index]["event"].startswith(event if event == "source" else "fixture")
    records[index][field] = value
    path = tmp_path / "receipt.jsonl"
    write_receipts(path, records)
    with pytest.raises(AssertionError):
        verify.checked_receipts(
            path,
            "candidate",
            selected("bidi-chromium-library"),
            "host-yuacx.tail8f3b66.ts.net",
            "100.101.102.103",
        )


@pytest.mark.parametrize(
    "mutation",
    ["missing-source", "extra-source-worker", "missing-response", "extra-response"],
)
def test_receipt_count_mutations_rejected(tmp_path: Path, mutation: str) -> None:
    records = receipts()
    if mutation == "missing-source":
        records.pop(0)
    elif mutation == "extra-source-worker":
        extra = deepcopy(records[0])
        extra["workerIndex"] = "1"
        records.append(extra)
    elif mutation == "missing-response":
        records.pop()
    else:
        records.append(deepcopy(records[-1]))
    path = tmp_path / "receipt.jsonl"
    write_receipts(path, records)
    with pytest.raises(AssertionError):
        verify.checked_receipts(
            path,
            "candidate",
            selected("bidi-chromium-library"),
            "host-yuacx.tail8f3b66.ts.net",
            "100.101.102.103",
        )


@pytest.mark.parametrize(
    "field,value",
    [
        ("fixtureHost", "localhost"),
        ("fixtureBind", "127.0.0.1"),
        ("resolvedIPv4", ["100.1.2.3"]),
    ],
)
def test_dns_mutations_rejected(field: str, value: object) -> None:
    dns = {
        "fixtureHost": "host-yuacx.tail8f3b66.ts.net",
        "fixtureBind": "100.101.102.103",
        "resolvedIPv4": ["100.101.102.103"],
    }
    dns[field] = value
    with pytest.raises(AssertionError):
        verify.checked_dns(dns)


def pair() -> tuple[dict, dict]:
    dns = {
        "fixtureHost": "host-yuacx.tail8f3b66.ts.net",
        "fixtureBind": "100.101.102.103",
        "resolvedIPv4": ["100.101.102.103"],
    }
    runs = [
        {"project": project, "suite": selection["suite"], "selected": selected(project)}
        for project, selection in capture.PROJECTS.items()
    ]
    common = {
        "scope": capture.SCOPE,
        "sourceClassification": capture.CLASSIFICATION,
        "fixtureAdaptation": capture.FIXTURE_ADAPTATION,
        "fixtureAdapted": True,
        "receiptAttribution": "worker-count-only",
        "upstreamInputs": capture.UPSTREAM_INPUTS,
        "inputs": {"manifest": "hash"},
        "fixtureDnsPreflight": dns,
        "unadaptedOfficialPass": False,
        "publicApiParityEvidence": False,
        "browserExecutionEvidence": False,
        "bidiProjectNamesOnly": True,
        "bidiProtocolParityEvidence": False,
        "currentLivePackageEvidence": False,
        "promotedToCoreLedger": False,
        "runs": runs,
    }
    return (
        {**deepcopy(common), "mode": "oracle", "candidateSnapshot": None},
        {
            **deepcopy(common),
            "mode": "candidate",
            "candidateSnapshot": str(capture.SNAPSHOT_PACKAGE),
        },
    )


def test_valid_pair_has_six_internal_ids() -> None:
    oracle, candidate = pair()
    compared = verify.compare_verified_pair(oracle, candidate)
    assert len(compared["selectedSpecIds"]) == 6
    assert compared["publicApiParityEvidence"] is False


@pytest.mark.parametrize(
    "mutation",
    [
        "mode",
        "inputs",
        "dns",
        "flag",
        "spec-id",
        "project",
        "suite",
        "oracle-red",
        "snapshot",
        "classification",
        "adaptation",
        "attribution",
    ],
)
def test_pair_mutations_rejected(mutation: str) -> None:
    oracle, candidate = pair()
    if mutation == "mode":
        candidate["mode"] = "oracle"
    elif mutation == "inputs":
        candidate["inputs"] = {"manifest": "other"}
    elif mutation == "dns":
        candidate["fixtureDnsPreflight"]["fixtureHost"] = "other.ts.net"
    elif mutation == "flag":
        candidate["publicApiParityEvidence"] = True
    elif mutation == "snapshot":
        candidate["candidateSnapshot"] = None
    elif mutation == "classification":
        candidate["sourceClassification"] = "public-api"
    elif mutation == "adaptation":
        candidate["fixtureAdaptation"] = "none"
    elif mutation == "attribution":
        candidate["receiptAttribution"] = "per-spec"
    elif mutation == "spec-id":
        candidate["runs"][0]["selected"][0]["specId"] = "wrong"
    elif mutation == "project":
        candidate["runs"][0]["project"] = "wrong"
    elif mutation == "suite":
        candidate["runs"][0]["suite"] = "wrong"
    else:
        oracle["runs"][0]["selected"][0]["status"] = "unexpected"
    with pytest.raises(AssertionError):
        verify.compare_verified_pair(oracle, candidate)
