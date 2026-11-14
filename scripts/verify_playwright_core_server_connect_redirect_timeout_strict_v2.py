"""Verify exact official Core redirect-timeout v2 oracle/candidate evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

import run_playwright_core_server_connect_redirect_timeout_strict_v1 as baseline
import run_playwright_core_server_connect_redirect_timeout_strict_v2 as capture
from official_candidate_package_snapshot_v1 import sha256
from verify_playwright_core_server_connect_redirect_timeout_strict_v1 import (
    checked_receipts,
)
from verify_playwright_official_provenance import verify_provenance


def _regular(path: Path) -> None:
    assert path.is_file() and not path.is_symlink()


def verify_directory(
    result_dir: Path, *, require_attestation: bool = True
) -> dict[str, object]:
    result_dir = result_dir.absolute()
    assert result_dir.parent.resolve() == capture.RESULT_PARENT.resolve()
    assert result_dir.name.startswith("core-server-connect-redirect-timeout-strict-v2-")
    assert result_dir.is_dir() and not result_dir.is_symlink()
    snapshot = capture.check_sources()
    inputs = {
        str(path.relative_to(capture.ROOT)): sha256(path)
        for path in capture.INPUT_FILES
    }
    dns: dict[str, object] | None = None
    oracle_ids: dict[str, list[tuple[str, str, int]]] = {}
    all_oracle_ids: list[str] = []
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
            selected = baseline.selected_tests(json.loads(result.read_text()), suite_id)
            identities = [
                (item["specId"], item["projectId"], item["line"]) for item in selected
            ]
            if mode == "oracle":
                oracle_ids[suite_id] = identities
                all_oracle_ids.extend("\t".join(map(str, item)) for item in identities)
            else:
                assert oracle_ids[suite_id] == identities
            provenance = verify_provenance(
                json.loads(provenance_path.read_text()),
                provenance_path=provenance_path,
                manifest_override=capture.MANIFEST,
                result_override=result,
                upstream_override=capture.UPSTREAM,
                candidate_override=capture.SNAPSHOT if mode == "candidate" else None,
                tooling_root=capture.ROOT,
            )
            assert provenance["mode"] == mode and provenance["suite"] == suite_id
            assert provenance["project"] is None and provenance["workers"] == 1
            assert provenance["filters"] == [f"{capture.SOURCE}:{capture.LINE}"]
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
                assert provenance["candidate"] == str(capture.SNAPSHOT)
                assert provenance["candidateProfile"] == capture.PROFILE
                assert provenance["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
                assert (
                    provenance["candidateSourceSha256"]
                    == capture.CANDIDATE_SOURCE_SHA256
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
            receipt_counts = checked_receipts(
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
                "receiptCounts": receipt_counts,
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
    digest = hashlib.sha256(
        ("\n".join(sorted(all_oracle_ids)) + "\n").encode()
    ).hexdigest()
    assert digest == capture.IDENTITIES_SHA256
    assert lanes["oracle"]["passed"] == 9
    assert lanes["oracle"]["unexpected"] == 0
    attestation: dict[str, object] = {
        "schema": 2,
        "scope": capture.SCOPE,
        "sourceClassification": "implementation-internal",
        "fixtureAdaptation": "common-testserver-tailscale-host-and-bind-only",
        "unadaptedOfficialTestBody": True,
        "unadaptedOfficialRun": False,
        "publicApiParityEvidence": False,
        "realBrowserServer": False,
        "promotedToCoreLedger": False,
        "upstreamCommit": capture.COMMIT,
        "upstreamSourceSha256": capture.SOURCE_SHA256,
        "expectedIdentitiesSha256": capture.IDENTITIES_SHA256,
        "expectedSuiteIdentitiesSha256": capture.IDENTITIES_SHA256_BY_SUITE,
        "fixtureDns": dns,
        "candidateProfile": capture.PROFILE,
        "candidateArtifactSha256": capture.ARTIFACT_SHA256,
        "candidateSourceSha256": capture.CANDIDATE_SOURCE_SHA256,
        "candidateSnapshot": str(capture.SNAPSHOT),
        "candidateSnapshotManifestSha256": sha256(capture.SNAPSHOT_MANIFEST),
        "predecessorRedAttestationSha256": capture.PREDECESSOR_SHA256,
        "inputs": inputs,
        "lanes": lanes,
    }
    assert snapshot["candidateSourceSha256"] == attestation["candidateSourceSha256"]
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
    except (AssertionError, KeyError, OSError, TypeError, ValueError) as error:
        parser.error(f"strict redirect timeout v2 evidence invalid: {error}")
    for mode in ("oracle", "candidate"):
        lane = attestation["lanes"][mode]
        print(
            f"verified {mode}: {lane['passed']}/9 expected, {lane['unexpected']} unexpected"
        )


if __name__ == "__main__":
    main()
