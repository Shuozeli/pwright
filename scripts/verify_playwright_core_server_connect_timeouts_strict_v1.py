"""Independently verify the browser-free Core timeout diagnostic pair."""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter
from pathlib import Path
from typing import Any

import run_playwright_core_server_connect_timeouts_strict_v1 as capture
from official_candidate_package_snapshot_v1 import sha256, verify_snapshot
from verify_playwright_official_provenance import verify_provenance

RECEIPT_COMMON = {
    "event",
    "pid",
    "workerIndex",
    "mode",
    "fixtureHost",
    "fixtureBind",
    "ownerBound",
    "cdpEndpointPresent",
}


def _regular(path: Path) -> None:
    assert path.is_file() and not path.is_symlink()


def checked_receipts(
    path: Path, mode: str, selected: list[dict[str, Any]], host: str, bind: str
) -> dict[str, int]:
    _regular(path)
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    assert records and all(isinstance(record, dict) for record in records)
    selected_workers = {str(item["attempt"]["workerIndex"]) for item in selected}
    adapted_workers: set[str] = set()
    upgrades: Counter[str] = Counter()
    for record in records:
        assert isinstance(record["pid"], int) and record["pid"] > 0
        assert record["mode"] == mode
        assert record["fixtureHost"] == host and record["fixtureBind"] == bind
        assert record["ownerBound"] == (True if mode == "candidate" else None)
        assert record["cdpEndpointPresent"] is False
        if record["event"] == "source-adapted":
            assert set(record) == RECEIPT_COMMON | {
                "sourceSha256",
                "adaptedSha256",
                "replacements",
            }
            assert record["sourceSha256"] == capture.SOURCE_SHA256
            assert record["adaptedSha256"] == capture.ADAPTED_SOURCE_SHA256
            assert record["replacements"] == 2
            if record["workerIndex"] is not None:
                worker = str(record["workerIndex"])
                assert worker not in adapted_workers
                adapted_workers.add(worker)
        else:
            assert record["event"] == "fixture-upgrade-request"
            assert set(record) == RECEIPT_COMMON | {"requestPath", "requestHost"}
            assert record["workerIndex"] is not None
            assert record["requestPath"] in {"/ws", "/ws-slow"}
            assert re.fullmatch(re.escape(host) + r":\d{1,5}", record["requestHost"])
            upgrades[record["requestPath"]] += 1
    assert selected_workers <= adapted_workers
    for line, request_path in capture.PATH_BY_LINE.items():
        passed = sum(
            item["line"] == line and item["status"] == "expected" for item in selected
        )
        assert passed <= upgrades[request_path] <= len(selected) // 2
    assert sum(upgrades.values()) <= len(selected)
    return dict(sorted(upgrades.items()))


def verify_directory(
    result_dir: Path, *, require_attestation: bool = True
) -> dict[str, object]:
    result_dir = result_dir.absolute()
    assert result_dir.parent.resolve() == capture.RESULT_PARENT.resolve()
    assert result_dir.name.startswith("core-server-connect-timeouts-strict-v1-")
    assert result_dir.is_dir() and not result_dir.is_symlink()
    capture.check_sources()
    inputs = {
        str(path.relative_to(capture.ROOT)): sha256(path)
        for path in capture.INPUT_FILES
    }
    snapshot_path = result_dir / "candidate-package-snapshot"
    snapshot_manifest = result_dir / "candidate-package-snapshot.files.json"
    snapshot = verify_snapshot(
        snapshot_path,
        snapshot_manifest,
        source_package=capture.SOURCE_PACKAGE,
        profile=capture.PROFILE,
        artifact_sha256=capture.ARTIFACT_SHA256,
    )
    dns: dict[str, object] | None = None
    oracle_ids: dict[str, list[tuple[str, str, int, str]]] = {}
    lanes: dict[str, dict[str, object]] = {}
    for mode in ("oracle", "candidate"):
        suite_lanes: dict[str, dict[str, object]] = {}
        for suite_id in capture.PROJECTS_BY_SUITE:
            result = result_dir / f"{mode}-{suite_id}.json"
            provenance_path = result_dir / f"{mode}-{suite_id}.provenance.json"
            receipt_path = result_dir / f"{mode}-{suite_id}.receipt.jsonl"
            log_path = result_dir / f"{mode}-{suite_id}.log"
            for path in (result, provenance_path, receipt_path, log_path):
                _regular(path)
            selected = capture.selected_tests(json.loads(result.read_text()), suite_id)
            identities = [
                (item["specId"], item["projectId"], item["line"], item["kind"])
                for item in selected
            ]
            if mode == "oracle":
                oracle_ids[suite_id] = identities
            else:
                assert oracle_ids[suite_id] == identities
            provenance = verify_provenance(
                json.loads(provenance_path.read_text()),
                provenance_path=provenance_path,
                manifest_override=capture.MANIFEST,
                result_override=result,
                upstream_override=capture.UPSTREAM,
                candidate_override=snapshot_path if mode == "candidate" else None,
                tooling_root=capture.ROOT,
            )
            assert provenance["mode"] == mode and provenance["suite"] == suite_id
            assert provenance["project"] is None and provenance["workers"] == 1
            assert provenance["filters"] == [
                f"{capture.SOURCE}:221",
                f"{capture.SOURCE}:228",
            ]
            assert provenance["headed"] is False
            assert (
                provenance["suiteRole"]
                == "browser-independent-fixture-host-adapted-diagnostic"
            )
            assert provenance["resultSha256"] == sha256(result)
            assert provenance["exitCode"] == (
                0 if all(item["status"] == "expected" for item in selected) else 1
            )
            if mode == "candidate":
                assert provenance["candidate"] == str(snapshot_path)
                assert provenance["candidateProfile"] == capture.PROFILE
                assert provenance["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
                assert (
                    provenance["candidateSourceSha256"]
                    == snapshot["candidateSourceSha256"]
                )
            else:
                assert provenance["candidate"] is None
            lane_dns = {
                "fixtureHost": provenance["fixtureHost"],
                "fixtureBind": provenance["fixtureBind"],
                "resolvedIPv4": [provenance["fixtureBind"]],
            }
            assert re.fullmatch(
                r"[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net",
                str(lane_dns["fixtureHost"]),
            )
            assert re.fullmatch(
                r"100\.(?:6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.(?:\d{1,3})\.(?:\d{1,3})",
                str(lane_dns["fixtureBind"]),
            )
            if dns is None:
                dns = lane_dns
            else:
                assert dns == lane_dns
            upgrade_counts = checked_receipts(
                receipt_path,
                mode,
                selected,
                str(lane_dns["fixtureHost"]),
                str(lane_dns["fixtureBind"]),
            )
            log = log_path.read_text()
            assert ("[pwright-candidate]" in log) == (mode == "candidate")
            suite_lanes[suite_id] = {
                "result": str(result),
                "resultSha256": sha256(result),
                "provenanceSha256": sha256(provenance_path),
                "receiptSha256": sha256(receipt_path),
                "logSha256": sha256(log_path),
                "exitCode": provenance["exitCode"],
                "passed": sum(item["status"] == "expected" for item in selected),
                "unexpected": sum(item["status"] == "unexpected" for item in selected),
                "upgradeCounts": upgrade_counts,
                "selected": selected,
            }
        lanes[mode] = {
            "passed": sum(int(suite["passed"]) for suite in suite_lanes.values()),
            "unexpected": sum(
                int(suite["unexpected"]) for suite in suite_lanes.values()
            ),
            "suites": suite_lanes,
        }
    assert dns is not None and set(oracle_ids) == set(capture.PROJECTS_BY_SUITE)
    attestation: dict[str, object] = {
        "schema": 1,
        "scope": capture.SCOPE,
        "sourceClassification": "implementation-internal",
        "fixtureAdaptation": "two-literal-localhost-ws-urls-to-tailscale-magicdns",
        "unadaptedOfficialPass": False,
        "publicApiParityEvidence": False,
        "realBrowserServer": False,
        "promotedToCoreLedger": False,
        "upstreamCommit": capture.COMMIT,
        "upstreamSourceSha256": capture.SOURCE_SHA256,
        "adaptedSourceSha256": capture.ADAPTED_SOURCE_SHA256,
        "expectedIdentitiesSha256": capture.EXPECTED_IDENTITIES_SHA256,
        "expectedSuiteIdentitiesSha256": capture.IDENTITIES_SHA256_BY_SUITE,
        "fixtureDns": dns,
        "candidateProfile": capture.PROFILE,
        "candidateArtifactSha256": capture.ARTIFACT_SHA256,
        "candidateSnapshot": str(snapshot_path),
        "candidateSnapshotManifestSha256": sha256(snapshot_manifest),
        "candidateSourceSha256": snapshot["candidateSourceSha256"],
        "inputs": inputs,
        "lanes": lanes,
    }
    if require_attestation:
        attestation_path = result_dir / "attestation.json"
        _regular(attestation_path)
        assert json.loads(attestation_path.read_text()) == attestation
    return attestation


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    try:
        attestation = verify_directory(args.result_dir)
    except (
        AssertionError,
        KeyError,
        OSError,
        TypeError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"strict timeout evidence invalid: {error}")
    for mode in ("oracle", "candidate"):
        lane = attestation["lanes"][mode]
        print(
            f"verified {mode}: {lane['passed']}/36 expected, {lane['unexpected']} unexpected"
        )


if __name__ == "__main__":
    main()
