#!/usr/bin/env python3
"""Independently verify the sealed post-fix Core CDP error pair."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

import run_playwright_core_cdp_error_slice_strict_v1 as base
import run_playwright_core_cdp_error_slice_strict_v2 as worker_aware
import run_playwright_core_cdp_error_slice_strict_v3 as capture
from official_candidate_package_snapshot_v1 import sha256, verify_snapshot
from verify_playwright_official_provenance import verify_provenance

EXPECTED_FIELDS = {
    "schema",
    "scope",
    "mode",
    "upstreamCommit",
    "upstreamSourceSha256",
    "serverFixtureSha256",
    "commonPreloadSha256",
    "inputs",
    "candidateProfile",
    "candidateArtifactSha256",
    "pairedOracleStrictSha256",
    "candidateSnapshot",
    "candidateSnapshotManifestSha256",
    "candidateSourceSha256",
    "result",
    "resultSha256",
    "provenanceSha256",
    "guardReceiptSha256",
    "logSha256",
    "exitCode",
    "selected",
}


def _regular(path: Path) -> None:
    assert path.is_file() and not path.is_symlink()


def verify_strict(strict_path: Path) -> dict[str, Any]:
    _regular(strict_path)
    strict = json.loads(strict_path.read_text())
    assert isinstance(strict, dict) and set(strict) == EXPECTED_FIELDS
    assert strict["schema"] == 4 and strict["scope"] == capture.SCOPE
    assert strict["mode"] in ("candidate", "oracle")
    assert strict["upstreamCommit"] == base.COMMIT
    assert strict["upstreamSourceSha256"] == base.SOURCE_SHA256
    assert strict["serverFixtureSha256"] == base.SERVER_FIXTURE_SHA256
    assert strict["commonPreloadSha256"] == base.COMMON_PRELOAD_SHA256
    capture.check_manifest()
    assert re.fullmatch(r"[0-9a-f]{64}", capture.ARTIFACT_SHA256)
    inputs = strict["inputs"]
    assert isinstance(inputs, dict) and set(inputs) == {
        str(path.relative_to(base.ROOT)) for path in capture.INPUT_FILES
    }
    assert all(base.sha256(base.ROOT / name) == digest for name, digest in inputs.items())
    result = Path(strict["result"])
    assert result.is_absolute()
    assert result == capture.RESULT_PARENT / f"{strict['mode']}-v1.json"
    _regular(result)
    assert strict_path == result.with_suffix(".strict.json")
    assert base.sha256(result) == strict["resultSha256"]
    provenance_path = result.with_suffix(".provenance.json")
    receipt_path = result.with_suffix(".guard-receipt.jsonl")
    log_path = result.with_suffix(".log")
    for path in (provenance_path, receipt_path, log_path):
        _regular(path)
    assert base.sha256(provenance_path) == strict["provenanceSha256"]
    assert base.sha256(receipt_path) == strict["guardReceiptSha256"]
    assert base.sha256(log_path) == strict["logSha256"]
    snapshot = None
    if strict["mode"] == "candidate":
        assert strict["candidateSnapshot"] == str(capture.SNAPSHOT_PACKAGE)
        snapshot_path = capture.SNAPSHOT_PACKAGE
        manifest_path = capture.SNAPSHOT_MANIFEST
        _regular(manifest_path)
        assert sha256(manifest_path) == strict["candidateSnapshotManifestSha256"]
        snapshot = verify_snapshot(
            snapshot_path,
            manifest_path,
            source_package=capture.SOURCE_PACKAGE,
            profile=capture.PROFILE,
            artifact_sha256=capture.ARTIFACT_SHA256,
        )
        assert snapshot["candidateSourceSha256"] == strict["candidateSourceSha256"]
    else:
        assert strict["candidateSnapshot"] is None
        assert strict["candidateSnapshotManifestSha256"] is None
        assert strict["candidateSourceSha256"] is None
    provenance = verify_provenance(
        json.loads(provenance_path.read_text()),
        provenance_path=provenance_path,
        manifest_override=capture.MANIFEST,
        result_override=result,
        upstream_override=base.UPSTREAM,
        candidate_override=capture.SNAPSHOT_PACKAGE if snapshot is not None else None,
        tooling_root=base.ROOT,
    )
    assert provenance["mode"] == strict["mode"]
    assert provenance["suite"] == capture.SUITE
    assert provenance["project"] == "chromium-library"
    assert provenance["workers"] == 1
    assert provenance["filters"] == [f"{base.SOURCE}:{line}" for line in capture.SELECTED_LINES]
    host = provenance["fixtureHost"]
    assert isinstance(host, str)
    assert re.fullmatch(r"[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net", host)
    assert provenance["resultSha256"] == strict["resultSha256"]
    assert provenance["exitCode"] == strict["exitCode"]
    assert strict["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
    log = log_path.read_text()
    if strict["mode"] == "candidate":
        assert strict["candidateProfile"] == capture.PROFILE
        assert provenance["candidate"] == str(capture.SNAPSHOT_PACKAGE)
        assert provenance["candidateSourceSha256"] == strict["candidateSourceSha256"]
        assert provenance["candidateProfile"] == capture.PROFILE
        assert provenance["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
        assert provenance["candidate"] is not None
        assert "[pwright-candidate]" in log
        assert strict["pairedOracleStrictSha256"] == base.sha256(capture.ORACLE_SIDECAR)
        oracle = verify_strict(capture.ORACLE_SIDECAR)
        assert oracle["mode"] == "oracle"
        assert all(item["status"] == "expected" for item in oracle["selected"])
    else:
        assert strict["candidateProfile"] is None
        assert strict["pairedOracleStrictSha256"] is None
        assert provenance["candidate"] is None
        assert provenance["candidateArtifactSha256"] is None
        assert "[pwright-candidate]" not in log
    selected = worker_aware.selected_tests(json.loads(result.read_text()))
    assert selected == strict["selected"]
    assert strict["exitCode"] == (
        0 if all(item["status"] == "expected" for item in selected) else 1
    )
    if strict["mode"] == "oracle":
        assert all(item["status"] == "expected" for item in selected)
    worker_aware.checked_receipts(receipt_path, selected, host)
    return strict


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("strict_sidecar", type=Path)
    args = parser.parse_args()
    try:
        strict = verify_strict(args.strict_sidecar.absolute())
    except (
        AssertionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"strict Core CDP error slice v3 evidence invalid: {error}")
    print(
        f"verified strict Core CDP error slice v3 mode={strict['mode']} "
        f"statuses={[item['status'] for item in strict['selected']]}"
    )


if __name__ == "__main__":
    main()
