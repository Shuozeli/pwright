"""Verify the paired browserless Bidi-config Core CDP Close evidence."""

from __future__ import annotations

import argparse
import ipaddress
import json
import re
import sys
from pathlib import Path
from typing import Any

import run_playwright_core_cdp_close_bidi_chromium_strict_v4 as capture
import run_playwright_core_cdp_error_slice_strict_v1 as cdp_base
from official_candidate_package_snapshot_v1 import sha256
from verify_playwright_official_provenance import verify_provenance

STRICT_FIELDS = {
    "schema",
    "scope",
    "protocolUnderTest",
    "bidiProjectNamesOnly",
    "fixtureAdapted",
    "currentLivePackageParityAttested",
    "publicApiParityEvidence",
    "promotedToCoreLedger",
    "mode",
    "upstreamCommit",
    "upstreamInputs",
    "pinnedHarness",
    "inputs",
    "candidateProfile",
    "candidateArtifactSha256",
    "candidateSnapshot",
    "candidateSnapshotManifestSha256",
    "candidateSourceSha256",
    "pairedOracleStrictSha256",
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
    "sourceSha256",
    "adaptedSha256",
    "fixtureHost",
    "replacements",
}


def regular(path: Path) -> None:
    assert path.is_file() and not path.is_symlink()


def checked_receipts(path: Path, selected: list[dict[str, Any]], host: str) -> None:
    regular(path)
    lines = path.read_text().splitlines()
    assert len(lines) == 2 and all(lines)
    records = [json.loads(line) for line in lines]
    assert all(
        isinstance(record, dict) and set(record) == RECEIPT_FIELDS for record in records
    )
    worker = str(selected[0]["attempts"][0]["workerIndex"])
    assert {record["workerIndex"] for record in records} == {None, worker}
    assert len({record["pid"] for record in records}) == 2
    adapted = cdp_base.adapted_source_sha256(host)
    for record in records:
        assert isinstance(record["pid"], int) and record["pid"] > 0
        assert record["sourceSha256"] == capture.SOURCE_SHA256
        assert record["adaptedSha256"] == adapted
        assert record["fixtureHost"] == host
        assert record["replacements"] == {"websocket": 1, "serverPrefixCases": 0}


def verify_strict(strict_path: Path) -> dict[str, Any]:
    if sys.flags.optimize:
        raise RuntimeError(
            "strict Core verification requires Python assertions enabled"
        )
    regular(strict_path)
    strict = json.loads(strict_path.read_text())
    assert isinstance(strict, dict) and set(strict) == STRICT_FIELDS
    assert strict["schema"] == 1 and strict["scope"] == capture.SCOPE
    assert strict["protocolUnderTest"] == "CDP-over-WebSocket"
    assert strict["bidiProjectNamesOnly"] is True
    assert strict["fixtureAdapted"] is True
    assert strict["currentLivePackageParityAttested"] is False
    assert strict["publicApiParityEvidence"] is False
    assert strict["promotedToCoreLedger"] is False
    mode = strict["mode"]
    assert mode in ("oracle", "candidate")
    assert strict_path == capture.RESULT_PARENT / f"{mode}-v4.strict.json"
    assert strict["upstreamCommit"] == capture.COMMIT
    assert strict["upstreamInputs"] == capture.UPSTREAM_INPUTS
    assert strict["pinnedHarness"] == {
        str(path.relative_to(capture.ROOT)): digest
        for path, digest in capture.PINNED_HARNESS.items()
    }
    capture.check_source_and_manifest()
    inputs = strict["inputs"]
    assert isinstance(inputs, dict)
    assert set(inputs) == {
        str(path.relative_to(capture.ROOT)) for path in capture.INPUT_FILES
    }
    for name, digest in inputs.items():
        path = capture.ROOT / name
        regular(path)
        assert sha256(path) == digest
    if mode == "candidate":
        assert strict["candidateProfile"] == capture.PROFILE
        assert strict["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
        assert strict["candidateSnapshot"] == str(capture.FROZEN_SNAPSHOT)
        assert (
            strict["candidateSnapshotManifestSha256"] == capture.FROZEN_MANIFEST_SHA256
        )
        assert strict["candidateSourceSha256"] == capture.FROZEN_SOURCE_SHA256
        snapshot = capture.check_frozen_snapshot()
        assert snapshot["candidateSourceSha256"] == strict["candidateSourceSha256"]
        oracle_path = capture.RESULT_PARENT / "oracle-v4.strict.json"
        regular(oracle_path)
        assert strict["pairedOracleStrictSha256"] == sha256(oracle_path)
        oracle = verify_strict(oracle_path)
        assert oracle["mode"] == "oracle"
        assert all(
            item["status"] == "expected"
            for run in oracle["runs"]
            for item in run["selected"]
        )
    else:
        for key in (
            "candidateProfile",
            "candidateArtifactSha256",
            "candidateSnapshot",
            "candidateSnapshotManifestSha256",
            "candidateSourceSha256",
            "pairedOracleStrictSha256",
        ):
            assert strict[key] is None

    runs = strict["runs"]
    assert isinstance(runs, list) and len(runs) == len(capture.PROJECTS)
    assert [run["project"] for run in runs] == list(capture.PROJECTS)
    all_ids: set[str] = set()
    for run in runs:
        assert isinstance(run, dict) and set(run) == RUN_FIELDS
        project = run["project"]
        assert run["suite"] == capture.PROJECTS[project]["suite"]
        output = capture.paths(mode, project)
        assert run["result"] == str(output["result"])
        for field, path in (
            ("resultSha256", output["result"]),
            ("provenanceSha256", output["provenance"]),
            ("guardReceiptSha256", output["receipt"]),
            ("logSha256", output["log"]),
        ):
            regular(path)
            assert run[field] == sha256(path)
        provenance = verify_provenance(
            json.loads(output["provenance"].read_text()),
            provenance_path=output["provenance"],
            manifest_override=capture.MANIFEST,
            result_override=output["result"],
            upstream_override=capture.UPSTREAM,
            candidate_override=capture.FROZEN_SNAPSHOT if mode == "candidate" else None,
            tooling_root=capture.ROOT,
        )
        assert provenance["mode"] == mode
        assert provenance["suite"] == run["suite"]
        assert provenance["project"] == project
        assert provenance["adapter"] == (
            "test-mode-playwright-core" if mode == "candidate" else None
        )
        assert provenance["workers"] == 1 and provenance["headed"] is False
        assert provenance["filters"] == [capture.FILTER]
        assert provenance["resultSha256"] == run["resultSha256"]
        assert provenance["exitCode"] == run["exitCode"]
        command = provenance["command"]
        assert command.count("--config=tests/bidi/playwright.config.ts") == 1
        assert command.count(f"--project={project}") == 1
        assert command.count(capture.FILTER) == 1
        assert not any(
            argument.startswith("--grep=") or argument == "--headed"
            for argument in command
        )
        fixture_host = provenance["fixtureHost"]
        fixture_bind = provenance["fixtureBind"]
        assert isinstance(fixture_host, str) and re.fullmatch(
            r"[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net", fixture_host
        )
        assert isinstance(fixture_bind, str)
        assert ipaddress.ip_address(fixture_bind) in ipaddress.ip_network(
            "100.64.0.0/10"
        )
        if mode == "candidate":
            assert provenance["candidate"] == str(capture.FROZEN_SNAPSHOT)
            assert provenance["candidateProfile"] == capture.PROFILE
            assert provenance["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
            assert provenance["candidateSourceSha256"] == capture.FROZEN_SOURCE_SHA256
        else:
            assert provenance["candidate"] is None
            assert provenance["candidateArtifactSha256"] is None
        selected = capture.selected_tests(
            json.loads(output["result"].read_text()), project
        )
        assert run["selected"] == selected
        assert run["exitCode"] == (0 if selected[0]["status"] == "expected" else 1)
        if mode == "oracle":
            assert selected[0]["status"] == "expected"
        assert ("[pwright-candidate]" in output["log"].read_text()) == (
            mode == "candidate"
        )
        checked_receipts(output["receipt"], selected, fixture_host)
        spec_id = selected[0]["specId"]
        assert spec_id not in all_ids
        all_ids.add(spec_id)
    assert all_ids == {item["specId"] for item in capture.PROJECTS.values()}
    return strict


def verify_pair(
    oracle_path: Path, candidate_path: Path
) -> tuple[dict[str, Any], dict[str, Any]]:
    oracle = verify_strict(oracle_path)
    candidate = verify_strict(candidate_path)
    assert oracle["mode"] == "oracle" and candidate["mode"] == "candidate"
    assert candidate["pairedOracleStrictSha256"] == sha256(oracle_path)
    assert oracle["upstreamInputs"] == candidate["upstreamInputs"]
    assert oracle["inputs"] == candidate["inputs"]
    for oracle_run, candidate_run in zip(
        oracle["runs"], candidate["runs"], strict=True
    ):
        assert oracle_run["project"] == candidate_run["project"]
        assert [item["specId"] for item in oracle_run["selected"]] == [
            item["specId"] for item in candidate_run["selected"]
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
                f"verified Bidi-config CDP Close {strict['mode']}: "
                f"{[item['status'] for run in strict['runs'] for item in run['selected']]}"
            )
        else:
            oracle, candidate = verify_pair(
                args.strict_sidecar.absolute(), args.candidate_sidecar.absolute()
            )
            print(
                "verified Bidi-config CDP Close pair: "
                f"oracle={[item['status'] for run in oracle['runs'] for item in run['selected']]}, "
                f"candidate={[item['status'] for run in candidate['runs'] for item in run['selected']]}"
            )
    except (
        AssertionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"strict Bidi-config CDP Close evidence invalid: {error}")


if __name__ == "__main__":
    main()
