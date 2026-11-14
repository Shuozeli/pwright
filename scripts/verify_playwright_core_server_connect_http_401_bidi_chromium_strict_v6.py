#!/usr/bin/env python3
"""Independently verify six Bidi-config HTTP-401 diagnostic invocations."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any

import run_playwright_core_server_connect_http_401_bidi_chromium_strict_v6 as capture
from official_candidate_package_snapshot_v1 import sha256, verify_snapshot
from verify_playwright_official_provenance import verify_provenance

STRICT_FIELDS = {
    "schema",
    "scope",
    "sourceClassification",
    "fixtureAdaptation",
    "fixtureAdapted",
    "receiptAttribution",
    "unadaptedOfficialPass",
    "publicApiParityEvidence",
    "browserExecutionEvidence",
    "bidiProjectNamesOnly",
    "bidiProtocolParityEvidence",
    "currentLivePackageEvidence",
    "promotedToCoreLedger",
    "mode",
    "upstreamCommit",
    "upstreamInputs",
    "inputs",
    "fixtureDnsPreflight",
    "candidateProfile",
    "candidateArtifactSha256",
    "candidateSnapshot",
    "candidateSnapshotManifestSha256",
    "candidateSourceSha256",
    "runs",
}
RUN_FIELDS = {
    "project",
    "suite",
    "result",
    "resultSha256",
    "provenanceSha256",
    "guardReceiptSha256",
    "logSha256",
    "exitCode",
    "selected",
}
COMMON_RECEIPT_FIELDS = {
    "pid",
    "workerIndex",
    "mode",
    "fixtureHost",
    "fixtureBind",
    "ownerBound",
    "cdpEndpointPresent",
    "event",
}
SOURCE_RECEIPT_FIELDS = COMMON_RECEIPT_FIELDS | {
    "sourceSha256",
    "adaptedSha256",
    "replacements",
}
RESPONSE_RECEIPT_FIELDS = COMMON_RECEIPT_FIELDS | {"responseSha256"}
RESPONSE_SHA256 = hashlib.sha256(b"HTTP/1.1 401 Unauthorized\r\n\r\nUnauthorized body").hexdigest()


def regular(path: Path) -> None:
    assert path.is_file() and not path.is_symlink()


def selected_tests(report: dict[str, Any], project: str) -> list[dict[str, Any]]:
    expected_ids = capture.PROJECTS[project]["specs"]
    assert report["errors"] == []
    selected: list[dict[str, Any]] = []
    assert len(report["suites"]) == 1
    file_suite = report["suites"][0]
    assert file_suite["title"] == "library/browsertype-connect.spec.ts"
    assert file_suite["file"] == "library/browsertype-connect.spec.ts"
    assert file_suite["specs"] == []
    groups = file_suite["suites"]
    assert [group["title"] for group in groups] == ["launchServer", "run-server"]
    for group in groups:
        assert group["file"] == "library/browsertype-connect.spec.ts"
        assert not group.get("suites")
        assert len(group["specs"]) == 1
        for spec in group["specs"]:
            spec_id = spec["id"]
            assert spec_id in expected_ids
            assert expected_ids[spec_id] == group["title"]
            assert (spec["file"], spec["line"], spec["column"], spec["title"]) == (
                "library/browsertype-connect.spec.ts",
                116,
                5,
                "should print HTTP error",
            )
            assert len(spec["tests"]) == 1
            test = spec["tests"][0]
            assert test["projectId"] == test["projectName"] == project
            assert test["expectedStatus"] == "passed"
            assert test["status"] in {"expected", "unexpected"}
            assert len(test["results"]) == 1
            attempt = test["results"][0]
            assert attempt["retry"] == 0 and attempt["status"] in {"passed", "failed"}
            assert type(attempt["workerIndex"]) is int
            assert 0 <= attempt["workerIndex"] <= 1
            passed = attempt["status"] == "passed"
            assert spec["ok"] == passed
            assert test["status"] == ("expected" if passed else "unexpected")
            errors = attempt["errors"]
            assert isinstance(errors, list) and (not errors if passed else bool(errors))
            assert all(
                isinstance(error, dict)
                and isinstance(error.get("message"), str)
                and error["message"]
                for error in errors
            )
            selected.append(
                {
                    "project": project,
                    "kind": expected_ids[spec_id],
                    "specId": spec_id,
                    "line": 116,
                    "title": spec["title"],
                    "status": test["status"],
                    "attempts": [
                        {
                            "retry": attempt["retry"],
                            "status": attempt["status"],
                            "workerIndex": attempt["workerIndex"],
                        }
                    ],
                    "errors": [str(error.get("message", "")) for error in errors],
                }
            )
    assert len(selected) == 2
    assert {item["specId"] for item in selected} == set(expected_ids)
    stats = report["stats"]
    assert stats["expected"] == sum(item["status"] == "expected" for item in selected)
    assert stats["unexpected"] == sum(item["status"] == "unexpected" for item in selected)
    assert stats["skipped"] == stats["flaky"] == 0
    return sorted(selected, key=lambda item: item["kind"])


def checked_receipts(
    path: Path,
    mode: str,
    selected: list[dict[str, Any]],
    host: str,
    bind: str,
) -> None:
    regular(path)
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    assert records and all(isinstance(record, dict) for record in records)
    selected_workers = Counter(str(item["attempts"][0]["workerIndex"]) for item in selected)
    passed_workers = Counter(
        str(item["attempts"][0]["workerIndex"]) for item in selected if item["status"] == "expected"
    )
    source_workers: set[str] = set()
    response_workers: Counter[str] = Counter()
    for record in records:
        assert isinstance(record["pid"], int) and record["pid"] > 0
        assert record["mode"] == mode
        assert record["fixtureHost"] == host and record["fixtureBind"] == bind
        assert record["ownerBound"] == (True if mode == "candidate" else None)
        assert record["cdpEndpointPresent"] is False
        if record["event"] == "source-adapted":
            assert set(record) == SOURCE_RECEIPT_FIELDS
            assert record["sourceSha256"] == capture.SOURCE_SHA256
            assert record["adaptedSha256"] == capture.ADAPTED_SOURCE_SHA256
            assert record["replacements"] == 1
            if record["workerIndex"] is not None:
                source_workers.add(str(record["workerIndex"]))
        else:
            assert record["event"] == "fixture-401-sent"
            assert set(record) == RESPONSE_RECEIPT_FIELDS
            assert record["responseSha256"] == RESPONSE_SHA256
            assert record["workerIndex"] is not None
            response_workers[str(record["workerIndex"])] += 1
    assert source_workers == set(selected_workers)
    # The guard records worker IDs but not spec IDs. This bounds responses per
    # worker without falsely attributing a response to a failed invocation.
    assert set(response_workers) <= set(selected_workers)
    for worker, total in selected_workers.items():
        assert passed_workers[worker] <= response_workers[worker] <= total


def checked_dns(value: object) -> dict[str, Any]:
    assert isinstance(value, dict) and set(value) == {
        "fixtureHost",
        "fixtureBind",
        "resolvedIPv4",
    }
    host = value["fixtureHost"]
    bind = value["fixtureBind"]
    assert isinstance(host, str) and re.fullmatch(
        r"[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net", host
    )
    assert isinstance(bind, str) and re.fullmatch(
        r"100\.(?:6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\."
        r"(?:[0-9]{1,3})\.(?:[0-9]{1,3})",
        bind,
    )
    assert value["resolvedIPv4"] == [bind]
    return value


def verify_strict(strict_path: Path) -> dict[str, Any]:
    if sys.flags.optimize:
        raise RuntimeError("strict Core verification requires Python assertions enabled")
    regular(strict_path)
    strict = json.loads(strict_path.read_text())
    assert isinstance(strict, dict) and set(strict) == STRICT_FIELDS
    assert strict["schema"] == 1 and strict["scope"] == capture.SCOPE
    assert strict["sourceClassification"] == capture.CLASSIFICATION
    assert strict["fixtureAdaptation"] == capture.FIXTURE_ADAPTATION
    assert strict["fixtureAdapted"] is True
    assert strict["receiptAttribution"] == "worker-count-only"
    for flag in (
        "unadaptedOfficialPass",
        "publicApiParityEvidence",
        "browserExecutionEvidence",
        "bidiProtocolParityEvidence",
        "currentLivePackageEvidence",
        "promotedToCoreLedger",
    ):
        assert strict[flag] is False
    assert strict["bidiProjectNamesOnly"] is True
    mode = strict["mode"]
    assert mode in {"oracle", "candidate"}
    assert strict_path == capture.RESULT_PARENT / f"{mode}-v6.strict.json"
    assert strict["upstreamCommit"] == capture.COMMIT
    assert strict["upstreamInputs"] == capture.UPSTREAM_INPUTS
    capture.check_browserless_source_and_manifest()
    inputs = strict["inputs"]
    assert isinstance(inputs, dict)
    assert set(inputs) == {str(path.relative_to(capture.ROOT)) for path in capture.INPUT_FILES}
    assert all(sha256(capture.ROOT / name) == digest for name, digest in inputs.items())
    dns = checked_dns(strict["fixtureDnsPreflight"])
    if mode == "candidate":
        assert strict["candidateProfile"] == capture.PROFILE
        assert strict["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
        assert strict["candidateSnapshot"] == str(capture.SNAPSHOT_PACKAGE)
        assert strict["candidateSnapshotManifestSha256"] == capture.SNAPSHOT_MANIFEST_SHA256
        assert strict["candidateSourceSha256"] == capture.CANDIDATE_SOURCE_SHA256
    else:
        for key in (
            "candidateProfile",
            "candidateArtifactSha256",
            "candidateSnapshot",
            "candidateSnapshotManifestSha256",
            "candidateSourceSha256",
        ):
            assert strict[key] is None
    regular(capture.SNAPSHOT_MANIFEST)
    assert sha256(capture.SNAPSHOT_MANIFEST) == capture.SNAPSHOT_MANIFEST_SHA256
    snapshot = verify_snapshot(
        capture.SNAPSHOT_PACKAGE,
        capture.SNAPSHOT_MANIFEST,
        source_package=capture.SOURCE_PACKAGE,
        profile=capture.PROFILE,
        artifact_sha256=capture.ARTIFACT_SHA256,
    )
    assert snapshot["candidateSourceSha256"] == capture.CANDIDATE_SOURCE_SHA256
    runs = strict["runs"]
    assert isinstance(runs, list) and len(runs) == len(capture.PROJECTS)
    assert [run["project"] for run in runs] == list(capture.PROJECTS)
    observed_ids: set[str] = set()
    for run in runs:
        assert isinstance(run, dict) and set(run) == RUN_FIELDS
        project = run["project"]
        suite = capture.PROJECTS[project]["suite"]
        assert run["suite"] == suite
        output = capture.paths(mode, project)
        assert run["result"] == str(output["result"])
        for key, path in (
            ("resultSha256", output["result"]),
            ("provenanceSha256", output["provenance"]),
            ("guardReceiptSha256", output["receipt"]),
            ("logSha256", output["log"]),
        ):
            regular(path)
            assert run[key] == sha256(path)
        provenance = verify_provenance(
            json.loads(output["provenance"].read_text()),
            provenance_path=output["provenance"],
            manifest_override=capture.MANIFEST,
            result_override=output["result"],
            upstream_override=capture.UPSTREAM,
            candidate_override=capture.SNAPSHOT_PACKAGE if mode == "candidate" else None,
            tooling_root=capture.ROOT,
        )
        assert provenance["mode"] == mode and provenance["suite"] == suite
        assert provenance["project"] == project
        assert provenance["adapter"] == (
            "test-mode-playwright-core" if mode == "candidate" else None
        )
        assert provenance["workers"] == 1 and provenance["headed"] is False
        assert provenance["filters"] == [f"{capture.SOURCE}:116"]
        assert provenance["suiteRole"] == "browser-independent-fixture-adapted-diagnostic"
        assert provenance["resultSha256"] == run["resultSha256"]
        assert provenance["exitCode"] == run["exitCode"]
        assert (provenance["fixtureHost"], provenance["fixtureBind"]) == (
            dns["fixtureHost"],
            dns["fixtureBind"],
        )
        command = provenance["command"]
        assert command.count("--config=tests/bidi/playwright.config.ts") == 1
        assert command.count(f"--project={project}") == 1
        assert command.count(f"{capture.SOURCE}:116") == 1
        if mode == "candidate":
            assert provenance["candidate"] == str(capture.SNAPSHOT_PACKAGE)
            assert provenance["candidateProfile"] == capture.PROFILE
            assert provenance["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
            assert provenance["candidateSourceSha256"] == capture.CANDIDATE_SOURCE_SHA256
        else:
            assert provenance["candidate"] is None
        selected = selected_tests(json.loads(output["result"].read_text()), project)
        assert selected == run["selected"]
        assert run["exitCode"] == (
            0 if all(item["status"] == "expected" for item in selected) else 1
        )
        if mode == "oracle":
            assert all(item["status"] == "expected" for item in selected)
        assert ("[pwright-candidate]" in output["log"].read_text()) == (mode == "candidate")
        checked_receipts(output["receipt"], mode, selected, dns["fixtureHost"], dns["fixtureBind"])
        for item in selected:
            assert item["specId"] not in observed_ids
            observed_ids.add(item["specId"])
    assert observed_ids == {
        spec_id for selection in capture.PROJECTS.values() for spec_id in selection["specs"]
    }
    assert len(observed_ids) == 6
    return strict


def compare_verified_pair(oracle: dict[str, Any], candidate: dict[str, Any]) -> dict[str, Any]:
    assert oracle["mode"] == "oracle" and candidate["mode"] == "candidate"
    assert oracle["scope"] == candidate["scope"] == capture.SCOPE
    assert (
        oracle["sourceClassification"]
        == candidate["sourceClassification"]
        == capture.CLASSIFICATION
    )
    assert (
        oracle["fixtureAdaptation"] == candidate["fixtureAdaptation"] == capture.FIXTURE_ADAPTATION
    )
    assert oracle["fixtureAdapted"] is candidate["fixtureAdapted"] is True
    assert oracle["receiptAttribution"] == candidate["receiptAttribution"] == "worker-count-only"
    assert oracle["upstreamInputs"] == candidate["upstreamInputs"]
    assert oracle["inputs"] == candidate["inputs"]
    assert oracle["fixtureDnsPreflight"] == candidate["fixtureDnsPreflight"]
    assert oracle["candidateSnapshot"] is None
    assert candidate["candidateSnapshot"] == str(capture.SNAPSHOT_PACKAGE)
    for flag in (
        "unadaptedOfficialPass",
        "publicApiParityEvidence",
        "browserExecutionEvidence",
        "bidiProtocolParityEvidence",
        "currentLivePackageEvidence",
        "promotedToCoreLedger",
    ):
        assert oracle[flag] is candidate[flag] is False
    assert oracle["bidiProjectNamesOnly"] is candidate["bidiProjectNamesOnly"] is True
    assert [run["project"] for run in oracle["runs"]] == list(capture.PROJECTS)
    assert [run["project"] for run in candidate["runs"]] == list(capture.PROJECTS)
    for oracle_run, candidate_run in zip(oracle["runs"], candidate["runs"], strict=True):
        project = oracle_run["project"]
        assert oracle_run["suite"] == candidate_run["suite"] == capture.PROJECTS[project]["suite"]
        expected_ids = set(capture.PROJECTS[project]["specs"])
        assert {item["specId"] for item in oracle_run["selected"]} == expected_ids
        assert {item["specId"] for item in candidate_run["selected"]} == expected_ids
        assert len(oracle_run["selected"]) == len(candidate_run["selected"]) == 2
        assert [item["specId"] for item in oracle_run["selected"]] == [
            item["specId"] for item in candidate_run["selected"]
        ]
        assert all(item["status"] == "expected" for item in oracle_run["selected"])
    return {
        "scope": capture.SCOPE,
        "classification": capture.CLASSIFICATION,
        "fixtureDnsPreflight": oracle["fixtureDnsPreflight"],
        "candidateSnapshot": candidate["candidateSnapshot"],
        "selectedSpecIds": sorted(
            item["specId"] for run in oracle["runs"] for item in run["selected"]
        ),
        "candidateStatuses": [
            item["status"] for run in candidate["runs"] for item in run["selected"]
        ],
        "publicApiParityEvidence": False,
        "browserExecutionEvidence": False,
        "bidiProtocolParityEvidence": False,
        "currentLivePackageEvidence": False,
    }


def verify_pair(oracle_path: Path, candidate_path: Path) -> dict[str, Any]:
    return compare_verified_pair(verify_strict(oracle_path), verify_strict(candidate_path))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("oracle_strict", type=Path)
    parser.add_argument("candidate_strict", type=Path, nargs="?")
    args = parser.parse_args()
    try:
        if args.candidate_strict is None:
            strict = verify_strict(args.oracle_strict.absolute())
            print(
                f"verified Bidi HTTP-401 {strict['mode']}: "
                f"{[item['status'] for run in strict['runs'] for item in run['selected']]}"
            )
        else:
            pair = verify_pair(args.oracle_strict.absolute(), args.candidate_strict.absolute())
            print(f"verified Bidi HTTP-401 pair: {pair['candidateStatuses']}")
    except (
        AssertionError,
        KeyError,
        OSError,
        TypeError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"strict Bidi HTTP-401 evidence invalid: {error}")


if __name__ == "__main__":
    main()
