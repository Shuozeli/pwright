"""AAA mutation controls for the three browserless Bidi-config CDP Close cases."""

from __future__ import annotations

import json
import os
from copy import deepcopy
from pathlib import Path
from typing import Any

import pytest
import run_playwright_core_cdp_close_bidi_chromium_strict_v4 as capture
import run_playwright_core_cdp_error_slice_strict_v1 as cdp_base
import verify_playwright_core_cdp_close_bidi_chromium_strict_v4 as verifier
from official_candidate_package_snapshot_v1 import sha256

HOST = "host-yuacx.tail8f3b66.ts.net"
BIND = "100.101.102.103"


def report(project: str, *, passed: bool = True) -> dict[str, Any]:
    spec = {
        "id": capture.PROJECTS[project]["specId"],
        "file": capture.REPORT_FILE,
        "line": capture.LINE,
        "column": 5,
        "title": capture.TITLE,
        "ok": passed,
        "tests": [
            {
                "projectId": project,
                "expectedStatus": "passed",
                "status": "expected" if passed else "unexpected",
                "results": [
                    {
                        "retry": 0,
                        "status": "passed" if passed else "failed",
                        "workerIndex": 0,
                        "errors": []
                        if passed
                        else [{"message": "CDP close assertion failed"}],
                    }
                ],
            }
        ],
    }
    return {
        "errors": [],
        "suites": [{"specs": [spec], "suites": []}],
        "stats": {
            "expected": int(passed),
            "unexpected": int(not passed),
            "skipped": 0,
            "flaky": 0,
        },
    }


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.write_text(json.dumps(value) + "\n")


def receipt(selected: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [
        {
            "pid": 1000 + index,
            "workerIndex": worker,
            "sourceSha256": capture.SOURCE_SHA256,
            "adaptedSha256": cdp_base.adapted_source_sha256(HOST),
            "fixtureHost": HOST,
            "replacements": {"websocket": 1, "serverPrefixCases": 0},
        }
        for index, worker in enumerate(
            (None, str(selected[0]["attempts"][0]["workerIndex"]))
        )
    ]


def make_oracle(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    monkeypatch.setattr(capture, "RESULT_PARENT", tmp_path)
    monkeypatch.setattr(verifier, "verify_provenance", lambda raw, **_kwargs: raw)
    runs = []
    for project, selection in capture.PROJECTS.items():
        output = capture.paths("oracle", project)
        raw = report(project)
        write_json(output["result"], raw)
        selected = capture.selected_tests(raw, project)
        output["receipt"].write_text(
            "".join(json.dumps(record) + "\n" for record in receipt(selected))
        )
        output["log"].write_text("oracle selected\n")
        provenance = {
            "mode": "oracle",
            "suite": selection["suite"],
            "project": project,
            "adapter": None,
            "workers": 1,
            "headed": False,
            "filters": [capture.FILTER],
            "resultSha256": sha256(output["result"]),
            "exitCode": 0,
            "command": [
                "node",
                "test",
                "--config=tests/bidi/playwright.config.ts",
                f"--project={project}",
                capture.FILTER,
            ],
            "fixtureHost": HOST,
            "fixtureBind": BIND,
            "candidate": None,
            "candidateArtifactSha256": None,
        }
        write_json(output["provenance"], provenance)
        runs.append(
            {
                "project": project,
                "suite": selection["suite"],
                "result": str(output["result"]),
                "resultSha256": sha256(output["result"]),
                "provenanceSha256": sha256(output["provenance"]),
                "guardReceiptSha256": sha256(output["receipt"]),
                "logSha256": sha256(output["log"]),
                "exitCode": 0,
                "selected": selected,
            }
        )
    strict = {
        "schema": 1,
        "scope": capture.SCOPE,
        "protocolUnderTest": "CDP-over-WebSocket",
        "bidiProjectNamesOnly": True,
        "fixtureAdapted": True,
        "currentLivePackageParityAttested": False,
        "publicApiParityEvidence": False,
        "promotedToCoreLedger": False,
        "mode": "oracle",
        "upstreamCommit": capture.COMMIT,
        "upstreamInputs": capture.UPSTREAM_INPUTS,
        "pinnedHarness": {
            str(path.relative_to(capture.ROOT)): digest
            for path, digest in capture.PINNED_HARNESS.items()
        },
        "inputs": {
            str(path.relative_to(capture.ROOT)): sha256(path)
            for path in capture.INPUT_FILES
        },
        "candidateProfile": None,
        "candidateArtifactSha256": None,
        "candidateSnapshot": None,
        "candidateSnapshotManifestSha256": None,
        "candidateSourceSha256": None,
        "pairedOracleStrictSha256": None,
        "runs": runs,
    }
    sidecar = tmp_path / "oracle-v4.strict.json"
    write_json(sidecar, strict)
    return sidecar


def test_pinned_bidi_config_assertion_and_frozen_candidate() -> None:
    # Arrange
    expected = capture.FROZEN_SOURCE_SHA256

    # Act
    capture.check_source_and_manifest()
    snapshot = capture.check_frozen_snapshot()

    # Assert
    assert snapshot["candidateSourceSha256"] == expected


@pytest.mark.parametrize("project", list(capture.PROJECTS))
def test_exact_one_case_per_bidi_project(project: str) -> None:
    # Arrange
    raw = report(project)

    # Act
    selected = capture.selected_tests(raw, project)

    # Assert
    assert [item["specId"] for item in selected] == [
        capture.PROJECTS[project]["specId"]
    ]


@pytest.mark.parametrize("mutation", ["cross-project", "duplicate", "missing", "extra"])
def test_inexact_selection_rejected(mutation: str) -> None:
    # Arrange
    raw = report("bidi-chromium-library")
    specs = raw["suites"][0]["specs"]
    if mutation == "cross-project":
        specs[0]["id"] = capture.PROJECTS["bidi-chrome-stable-library"]["specId"]
    elif mutation == "duplicate" or mutation == "extra":
        specs.append(deepcopy(specs[0]))
    else:
        specs.clear()

    # Act
    with pytest.raises(AssertionError):
        capture.selected_tests(raw, "bidi-chromium-library")

    # Assert
    assert (
        len(specs) != 1
        or specs[0]["id"] != capture.PROJECTS["bidi-chromium-library"]["specId"]
    )


@pytest.mark.parametrize(
    "mutation", ["skip", "retry", "extra-attempt", "wrong-project", "wrong-title"]
)
def test_status_or_attempt_drift_rejected(mutation: str) -> None:
    # Arrange
    raw = report("bidi-chromium-library")
    spec = raw["suites"][0]["specs"][0]
    test = spec["tests"][0]
    attempt = test["results"][0]
    if mutation == "skip":
        test["status"] = "skipped"
        raw["stats"] = {"expected": 0, "unexpected": 0, "skipped": 1, "flaky": 0}
    elif mutation == "retry":
        attempt["retry"] = 1
    elif mutation == "extra-attempt":
        test["results"].append(deepcopy(attempt))
    elif mutation == "wrong-project":
        test["projectId"] = "chromium-library"
    else:
        spec["title"] = "different assertion"

    # Act
    with pytest.raises(AssertionError):
        capture.selected_tests(raw, "bidi-chromium-library")

    # Assert
    assert raw["suites"][0]["specs"]


def test_failed_candidate_is_recorded_as_unexpected() -> None:
    # Arrange
    raw = report("bidi-chromium-library", passed=False)

    # Act
    selected = capture.selected_tests(raw, "bidi-chromium-library")

    # Assert
    assert selected[0]["status"] == "unexpected"
    assert selected[0]["errors"] == ["CDP close assertion failed"]


@pytest.mark.parametrize(
    "field,value",
    [
        ("workerIndex", "1"),
        ("fixtureHost", "localhost"),
        ("sourceSha256", "0" * 64),
        ("adaptedSha256", "0" * 64),
        ("replacements", {"websocket": 2, "serverPrefixCases": 0}),
    ],
)
def test_receipt_tamper_rejected(tmp_path: Path, field: str, value: object) -> None:
    # Arrange
    selected = capture.selected_tests(
        report("bidi-chromium-library"), "bidi-chromium-library"
    )
    records = receipt(selected)
    records[-1][field] = value
    path = tmp_path / "receipt.jsonl"
    path.write_text("".join(json.dumps(record) + "\n" for record in records))

    # Act
    with pytest.raises(AssertionError):
        verifier.checked_receipts(path, selected, HOST)

    # Assert
    assert path.is_file()


def test_ambient_bidi_expectation_override_rejected(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setenv("PWTEST_USE_BIDI_EXPECTATIONS", "1")

    # Act
    with pytest.raises(ValueError, match="rejects inherited runtime overrides"):
        capture._check_environment()

    # Assert
    assert os.environ["PWTEST_USE_BIDI_EXPECTATIONS"] == "1"


def test_existing_result_destination_rejected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    monkeypatch.setattr(capture, "RESULT_PARENT", tmp_path)
    (tmp_path / "oracle-v4.strict.json").write_text("reserved\n")

    # Act
    with pytest.raises(ValueError, match="destination already exists"):
        capture.run("oracle")

    # Assert
    assert (tmp_path / "oracle-v4.strict.json").read_text() == "reserved\n"


def test_oracle_verifier_accepts_exact_three_project_evidence(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    sidecar = make_oracle(tmp_path, monkeypatch)

    # Act
    strict = verifier.verify_strict(sidecar)

    # Assert
    assert [run["selected"][0]["status"] for run in strict["runs"]] == [
        "expected",
        "expected",
        "expected",
    ]


@pytest.mark.parametrize(
    "field,value",
    [
        ("protocolUnderTest", "WebDriver-BiDi"),
        ("currentLivePackageParityAttested", True),
        ("publicApiParityEvidence", True),
        ("promotedToCoreLedger", True),
    ],
)
def test_diagnostic_scope_promotion_rejected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, field: str, value: object
) -> None:
    # Arrange
    sidecar = make_oracle(tmp_path, monkeypatch)
    strict = json.loads(sidecar.read_text())
    strict[field] = value
    write_json(sidecar, strict)

    # Act
    with pytest.raises(AssertionError):
        verifier.verify_strict(sidecar)

    # Assert
    assert strict[field] == value


def test_receipt_tamper_rejected_after_rehash(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    sidecar = make_oracle(tmp_path, monkeypatch)
    strict = json.loads(sidecar.read_text())
    path = capture.paths("oracle", "bidi-chromium-library")["receipt"]
    records = [json.loads(line) for line in path.read_text().splitlines()]
    records[-1]["fixtureHost"] = "other.tail8f3b66.ts.net"
    path.write_text("".join(json.dumps(record) + "\n" for record in records))
    strict["runs"][0]["guardReceiptSha256"] = sha256(path)
    write_json(sidecar, strict)

    # Act
    with pytest.raises(AssertionError):
        verifier.verify_strict(sidecar)

    # Assert
    assert records[-1]["fixtureHost"] != HOST


def test_report_skip_rejected_after_rehash(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    sidecar = make_oracle(tmp_path, monkeypatch)
    strict = json.loads(sidecar.read_text())
    result = capture.paths("oracle", "bidi-chromium-library")["result"]
    raw = json.loads(result.read_text())
    raw["suites"][0]["specs"][0]["tests"][0]["status"] = "skipped"
    write_json(result, raw)
    strict["runs"][0]["resultSha256"] = sha256(result)
    provenance = capture.paths("oracle", "bidi-chromium-library")["provenance"]
    raw_provenance = json.loads(provenance.read_text())
    raw_provenance["resultSha256"] = sha256(result)
    write_json(provenance, raw_provenance)
    strict["runs"][0]["provenanceSha256"] = sha256(provenance)
    write_json(sidecar, strict)

    # Act
    with pytest.raises(AssertionError):
        verifier.verify_strict(sidecar)

    # Assert
    assert raw["suites"][0]["specs"][0]["tests"][0]["status"] == "skipped"


def test_stale_harness_digest_rejected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    sidecar = make_oracle(tmp_path, monkeypatch)
    strict = json.loads(sidecar.read_text())
    strict["inputs"][str(capture.URL_ONLY_PRELOAD.relative_to(capture.ROOT))] = "0" * 64
    write_json(sidecar, strict)

    # Act
    with pytest.raises(AssertionError):
        verifier.verify_strict(sidecar)

    # Assert
    assert capture.URL_ONLY_PRELOAD.is_file()
