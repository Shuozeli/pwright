#!/usr/bin/env python3
"""Independently verify the four-case Core Firefox/WebKit WS-error evidence."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

import run_playwright_core_server_connect_ws_error_other_engines_strict_v3 as capture
from official_candidate_package_snapshot_v1 import sha256, verify_snapshot
from verify_playwright_official_provenance import verify_provenance

STRICT_FIELDS = {
    "schema",
    "scope",
    "sourceClassification",
    "fixtureAdapted",
    "publicApiParityEvidence",
    "promotedToCoreLedger",
    "mode",
    "upstreamCommit",
    "upstreamInputs",
    "inputs",
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
RECEIPT_FIELDS = {
    "pid",
    "workerIndex",
    "mode",
    "sourceSha256",
    "fixtureHost",
    "fixtureBind",
    "ownerBound",
    "cdpEndpointPresent",
    "sourceAdapted",
}


def _regular(path: Path) -> None:
    assert path.is_file() and not path.is_symlink()


def checked_receipts(
    path: Path, mode: str, selected: list[dict[str, Any]], host: str, bind: str
) -> None:
    _regular(path)
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    assert records and all(isinstance(record, dict) for record in records)
    expected_workers = {str(item["attempts"][0]["workerIndex"]) for item in selected}
    observed_workers = set()
    for record in records:
        assert set(record) == RECEIPT_FIELDS
        assert isinstance(record["pid"], int) and record["pid"] > 0
        assert record["mode"] == mode
        assert record["sourceSha256"] == capture.SOURCE_SHA256
        assert record["fixtureHost"] == host and record["fixtureBind"] == bind
        assert record["ownerBound"] == (True if mode == "candidate" else None)
        assert record["cdpEndpointPresent"] is False
        assert record["sourceAdapted"] is False
        if record["workerIndex"] is not None:
            observed_workers.add(str(record["workerIndex"]))
    assert expected_workers <= observed_workers


def verify_strict(strict_path: Path) -> dict[str, Any]:
    if sys.flags.optimize:
        raise RuntimeError(
            "strict Core verification requires Python assertions enabled"
        )
    _regular(strict_path)
    strict = json.loads(strict_path.read_text())
    assert isinstance(strict, dict) and set(strict) == STRICT_FIELDS
    assert strict["schema"] == 1 and strict["scope"] == capture.SCOPE
    assert strict["sourceClassification"] == "implementation-internal"
    assert strict["fixtureAdapted"] is True
    assert strict["publicApiParityEvidence"] is False
    assert strict["promotedToCoreLedger"] is False
    mode = strict["mode"]
    assert mode in {"oracle", "candidate"}
    assert strict_path == capture.RESULT_PARENT / f"{mode}-v3.strict.json"
    assert strict["upstreamCommit"] == capture.COMMIT
    assert strict["upstreamInputs"] == capture.UPSTREAM_INPUTS
    capture.check_browserless_source_and_manifest()
    inputs = strict["inputs"]
    assert isinstance(inputs, dict)
    assert set(inputs) == {
        str(path.relative_to(capture.ROOT)) for path in capture.INPUT_FILES
    }
    assert all(sha256(capture.ROOT / name) == digest for name, digest in inputs.items())
    if mode == "candidate":
        assert strict["candidateProfile"] == capture.PROFILE
        assert strict["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
        assert strict["candidateSnapshot"] == str(capture.SNAPSHOT_PACKAGE)
        _regular(capture.SNAPSHOT_MANIFEST)
        assert strict["candidateSnapshotManifestSha256"] == sha256(
            capture.SNAPSHOT_MANIFEST
        )
        snapshot = verify_snapshot(
            capture.SNAPSHOT_PACKAGE,
            capture.SNAPSHOT_MANIFEST,
            source_package=capture.SOURCE_PACKAGE,
            profile=capture.PROFILE,
            artifact_sha256=capture.ARTIFACT_SHA256,
        )
        assert strict["candidateSourceSha256"] == snapshot["candidateSourceSha256"]
    else:
        for key in (
            "candidateProfile",
            "candidateArtifactSha256",
            "candidateSnapshot",
            "candidateSnapshotManifestSha256",
            "candidateSourceSha256",
        ):
            assert strict[key] is None

    runs = strict["runs"]
    assert isinstance(runs, list) and len(runs) == len(capture.PROJECTS)
    assert [run["project"] for run in runs] == list(capture.PROJECTS)
    all_ids = set()
    for run in runs:
        assert isinstance(run, dict) and set(run) == RUN_FIELDS
        project = run["project"]
        selection = capture.PROJECTS[project]
        assert run["suite"] == selection["suite"]
        paths = capture._paths(mode, project)
        assert run["result"] == str(paths["result"])
        for key, path in (
            ("resultSha256", paths["result"]),
            ("provenanceSha256", paths["provenance"]),
            ("guardReceiptSha256", paths["receipt"]),
            ("logSha256", paths["log"]),
        ):
            _regular(path)
            assert run[key] == sha256(path)
        provenance = verify_provenance(
            json.loads(paths["provenance"].read_text()),
            provenance_path=paths["provenance"],
            manifest_override=capture.MANIFEST,
            result_override=paths["result"],
            upstream_override=capture.UPSTREAM,
            candidate_override=capture.SNAPSHOT_PACKAGE
            if mode == "candidate"
            else None,
            tooling_root=capture.ROOT,
        )
        assert provenance["mode"] == mode
        assert provenance["suite"] == selection["suite"]
        assert provenance["project"] == project
        assert provenance["workers"] == 1 and provenance["headed"] is False
        assert provenance["filters"] == [f"{capture.SOURCE}:122"]
        assert provenance["resultSha256"] == run["resultSha256"]
        assert provenance["exitCode"] == run["exitCode"]
        fixture_host = provenance["fixtureHost"]
        fixture_bind = provenance["fixtureBind"]
        assert isinstance(fixture_host, str) and isinstance(fixture_bind, str)
        assert fixture_host and re.fullmatch(
            r"[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net",
            fixture_host,
        )
        if mode == "candidate":
            assert provenance["candidate"] == str(capture.SNAPSHOT_PACKAGE)
            assert provenance["candidateProfile"] == capture.PROFILE
            assert provenance["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
            assert (
                provenance["candidateSourceSha256"] == strict["candidateSourceSha256"]
            )
        else:
            assert provenance["candidate"] is None
        selected = capture.selected_tests(
            json.loads(paths["result"].read_text()), project
        )
        assert selected == run["selected"]
        assert run["exitCode"] == (
            0 if all(item["status"] == "expected" for item in selected) else 1
        )
        if mode == "oracle":
            assert all(item["status"] == "expected" for item in selected)
        assert ("[pwright-candidate]" in paths["log"].read_text()) == (
            mode == "candidate"
        )
        checked_receipts(
            paths["receipt"],
            mode,
            selected,
            fixture_host,
            fixture_bind,
        )
        for item in selected:
            assert item["specId"] not in all_ids
            all_ids.add(item["specId"])
    assert all_ids == {
        spec_id
        for selection in capture.PROJECTS.values()
        for spec_id in selection["specs"]
    }
    return strict


def verify_pair(
    oracle_path: Path, candidate_path: Path
) -> tuple[dict[str, Any], dict[str, Any]]:
    oracle = verify_strict(oracle_path)
    candidate = verify_strict(candidate_path)
    assert oracle["mode"] == "oracle" and candidate["mode"] == "candidate"
    assert oracle["upstreamInputs"] == candidate["upstreamInputs"]
    assert oracle["inputs"] == candidate["inputs"]
    for oracle_run, candidate_run in zip(
        oracle["runs"], candidate["runs"], strict=True
    ):
        assert oracle_run["project"] == candidate_run["project"]
        assert oracle_run["suite"] == candidate_run["suite"]
        oracle_selected = oracle_run["selected"]
        candidate_selected = candidate_run["selected"]
        assert [item["specId"] for item in oracle_selected] == [
            item["specId"] for item in candidate_selected
        ]
    return oracle, candidate


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("strict_sidecar", type=Path)
    parser.add_argument("candidate_sidecar", type=Path, nargs="?")
    args = parser.parse_args()
    try:
        if args.candidate_sidecar is None:
            strict = verify_strict(args.strict_sidecar.absolute())
            print(
                f"verified Core Server-connect other engines {strict['mode']}: "
                f"{[item['status'] for run in strict['runs'] for item in run['selected']]}"
            )
        else:
            oracle, candidate = verify_pair(
                args.strict_sidecar.absolute(), args.candidate_sidecar.absolute()
            )
            print(
                "verified Core Server-connect other-engines strict pair: "
                f"oracle={[item['status'] for run in oracle['runs'] for item in run['selected']]}, "
                f"candidate={[item['status'] for run in candidate['runs'] for item in run['selected']]}"
            )
    except (
        AssertionError,
        KeyError,
        OSError,
        TypeError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        parser.error(
            f"strict Core Server-connect other-engine evidence invalid: {error}"
        )


if __name__ == "__main__":
    main()
