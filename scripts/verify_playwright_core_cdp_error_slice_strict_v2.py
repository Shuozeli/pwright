#!/usr/bin/env python3
"""Verify source-pinned official Core CDP errors with restarted workers."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import run_playwright_core_cdp_error_slice_strict_v1 as base
import run_playwright_core_cdp_error_slice_strict_v2 as capture
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
    assert strict["schema"] == 3 and strict["scope"] == capture.SCOPE
    assert strict["mode"] in ("candidate", "oracle")
    assert strict["upstreamCommit"] == base.COMMIT
    assert strict["upstreamSourceSha256"] == base.SOURCE_SHA256
    assert strict["serverFixtureSha256"] == base.SERVER_FIXTURE_SHA256
    assert strict["commonPreloadSha256"] == base.COMMON_PRELOAD_SHA256
    base.check_manifest()
    inputs = strict["inputs"]
    assert isinstance(inputs, dict) and set(inputs) == {
        str(base.MANIFEST.relative_to(base.ROOT)),
        str(base.PRELOAD.relative_to(base.ROOT)),
        str(base.COMMON_PRELOAD.relative_to(base.ROOT)),
        str(capture.BASE_RUNNER.relative_to(base.ROOT)),
        "scripts/run_playwright_core_cdp_error_slice_strict_v2.py",
    }
    assert all(base.sha256(base.ROOT / name) == digest for name, digest in inputs.items())
    result = Path(strict["result"])
    assert result.is_absolute() and result.parent == capture.RESULT_PARENT
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
    provenance = verify_provenance(
        json.loads(provenance_path.read_text()),
        provenance_path=provenance_path,
        manifest_override=base.MANIFEST,
        result_override=result,
        upstream_override=base.UPSTREAM,
        candidate_override=base.ROOT / "packages/pwright",
        tooling_root=base.ROOT,
    )
    assert provenance["mode"] == strict["mode"]
    assert provenance["suite"] == base.SUITE
    assert provenance["project"] == "chromium-library"
    assert provenance["workers"] == 1
    assert provenance["filters"] == [f"{base.SOURCE}:{line}" for line in base.SELECTED_LINES]
    host = provenance["fixtureHost"]
    assert isinstance(host, str) and host.endswith(".ts.net")
    assert provenance["resultSha256"] == strict["resultSha256"]
    assert provenance["exitCode"] == strict["exitCode"]
    log = log_path.read_text()
    if strict["mode"] == "candidate":
        assert strict["candidateProfile"] == base.PROFILE
        assert strict["candidateArtifactSha256"] == base.ARTIFACT_SHA256
        artifact = base.ROOT / "packages/pwright/native" / base.PROFILE / "pwright.node"
        _regular(artifact)
        assert base.sha256(artifact) == base.ARTIFACT_SHA256
        assert provenance["candidateProfile"] == base.PROFILE
        assert provenance["candidateArtifactSha256"] == base.ARTIFACT_SHA256
        assert provenance["candidate"] is not None
        assert "[pwright-candidate]" in log
    else:
        assert strict["candidateProfile"] is None
        assert strict["candidateArtifactSha256"] is None
        assert provenance["candidate"] is None
        assert "[pwright-candidate]" not in log
    selected = capture.selected_tests(json.loads(result.read_text()))
    assert selected == strict["selected"]
    assert strict["exitCode"] == (
        0 if all(item["status"] == "expected" for item in selected) else 1
    )
    capture.checked_receipts(receipt_path, selected, host)
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
        f"verified strict Core CDP error slice v2 mode={strict['mode']} "
        f"statuses={[item['status'] for item in strict['selected']]}"
    )


if __name__ == "__main__":
    main()
