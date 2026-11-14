#!/usr/bin/env python3
"""Independently verify three source-pinned official Core CDP error outcomes."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from run_playwright_core_cdp_error_slice_strict_v1 import (
    ARTIFACT_SHA256,
    COMMIT,
    COMMON_PRELOAD,
    COMMON_PRELOAD_SHA256,
    MANIFEST,
    PRELOAD,
    PROFILE,
    ROOT,
    SCOPE,
    SELECTED_LINES,
    SERVER_FIXTURE_SHA256,
    SOURCE,
    SOURCE_SHA256,
    SUITE,
    UPSTREAM,
    adapted_source_sha256,
    check_manifest,
    selected_tests,
    sha256,
)
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
    assert strict["schema"] == 2 and strict["scope"] == SCOPE
    assert strict["mode"] in ("candidate", "oracle")
    assert strict["upstreamCommit"] == COMMIT
    assert strict["upstreamSourceSha256"] == SOURCE_SHA256
    assert strict["serverFixtureSha256"] == SERVER_FIXTURE_SHA256
    assert strict["commonPreloadSha256"] == COMMON_PRELOAD_SHA256
    check_manifest()
    inputs = strict["inputs"]
    assert isinstance(inputs, dict) and set(inputs) == {
        str(MANIFEST.relative_to(ROOT)),
        str(PRELOAD.relative_to(ROOT)),
        str(COMMON_PRELOAD.relative_to(ROOT)),
        "scripts/run_playwright_core_cdp_error_slice_strict_v1.py",
    }
    assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())
    result = Path(strict["result"])
    assert result.is_absolute()
    _regular(result)
    assert strict_path == result.with_suffix(".strict.json")
    assert sha256(result) == strict["resultSha256"]
    provenance_path = result.with_suffix(".provenance.json")
    receipt_path = result.with_suffix(".guard-receipt.jsonl")
    log_path = result.with_suffix(".log")
    for path in (provenance_path, receipt_path, log_path):
        _regular(path)
    assert sha256(provenance_path) == strict["provenanceSha256"]
    assert sha256(receipt_path) == strict["guardReceiptSha256"]
    assert sha256(log_path) == strict["logSha256"]
    provenance = verify_provenance(
        json.loads(provenance_path.read_text()),
        provenance_path=provenance_path,
        manifest_override=MANIFEST,
        result_override=result,
        upstream_override=UPSTREAM,
        candidate_override=ROOT / "packages/pwright",
        tooling_root=ROOT,
    )
    assert provenance["mode"] == strict["mode"]
    assert provenance["suite"] == SUITE
    assert provenance["project"] == "chromium-library"
    assert provenance["workers"] == 1
    assert provenance["filters"] == [f"{SOURCE}:{line}" for line in SELECTED_LINES]
    host = provenance["fixtureHost"]
    assert isinstance(host, str) and host.endswith(".ts.net")
    assert provenance["resultSha256"] == strict["resultSha256"]
    assert provenance["exitCode"] == strict["exitCode"]
    log = log_path.read_text()
    if strict["mode"] == "candidate":
        assert strict["candidateProfile"] == PROFILE
        assert strict["candidateArtifactSha256"] == ARTIFACT_SHA256
        artifact = ROOT / "packages/pwright/native" / PROFILE / "pwright.node"
        _regular(artifact)
        assert sha256(artifact) == ARTIFACT_SHA256
        assert provenance["candidateProfile"] == PROFILE
        assert provenance["candidateArtifactSha256"] == ARTIFACT_SHA256
        assert provenance["candidate"] is not None
        assert "[pwright-candidate]" in log
    else:
        assert strict["candidateProfile"] is None
        assert strict["candidateArtifactSha256"] is None
        assert provenance["candidate"] is None
        assert "[pwright-candidate]" not in log
    selected = selected_tests(json.loads(result.read_text()))
    assert selected == strict["selected"]
    assert strict["exitCode"] == (
        0 if all(item["status"] == "expected" for item in selected) else 1
    )
    lines = receipt_path.read_text().splitlines()
    assert lines and all(lines)
    records = [json.loads(line) for line in lines]
    assert any(record["workerIndex"] == "0" for record in records)
    assert all(
        set(record)
        == {
            "pid",
            "workerIndex",
            "sourceSha256",
            "adaptedSha256",
            "fixtureHost",
            "replacements",
        }
        for record in records
    )
    assert all(
        isinstance(record["pid"], int)
        and record["pid"] > 0
        and record["workerIndex"] in (None, "0")
        and record["sourceSha256"] == SOURCE_SHA256
        and record["adaptedSha256"] == adapted_source_sha256(host)
        and record["fixtureHost"] == host
        and record["replacements"] == {"websocket": 1, "serverPrefixCases": 0}
        for record in records
    )
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
        parser.error(f"strict Core CDP error slice evidence invalid: {error}")
    print(
        f"verified strict Core CDP error slice mode={strict['mode']} "
        f"statuses={[item['status'] for item in strict['selected']]}"
    )


if __name__ == "__main__":
    main()
