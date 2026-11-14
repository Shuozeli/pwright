#!/usr/bin/env python3
"""Verify paired official BrowserType path evidence and the fixture-only shim."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from run_playwright_core_browser_type_owner_strict_v2 import (
    ARTIFACT_SHA256,
    COMMIT,
    FIXTURE_SHA256,
    MANIFEST,
    PRELOAD,
    PROFILE,
    ROOT,
    SOURCE_SHA256,
    SUITE,
    UPSTREAM,
    check_source,
    selected_tests,
    sha256,
)
from verify_playwright_official_provenance import verify_provenance


def verify_strict(path: Path) -> dict:
    strict = json.loads(path.read_text())
    assert set(strict) == {
        "schema",
        "scope",
        "mode",
        "upstreamCommit",
        "upstreamSourceSha256",
        "upstreamFixtureSha256",
        "inputs",
        "candidateProfile",
        "candidateArtifactSha256",
        "result",
        "resultSha256",
        "provenanceSha256",
        "ownerReceiptSha256",
        "logSha256",
        "exitCode",
        "selected",
    }
    assert strict["schema"] == 1
    assert (
        strict["scope"]
        == "unchanged-official-assertions-browserless-private-owner-fixture-adaptation-v2"
    )
    assert strict["mode"] in {"candidate", "oracle"}
    assert strict["upstreamCommit"] == COMMIT
    assert strict["upstreamSourceSha256"] == SOURCE_SHA256
    assert strict["upstreamFixtureSha256"] == FIXTURE_SHA256
    check_source()
    inputs = strict["inputs"]
    assert set(inputs) == {
        str(MANIFEST.relative_to(ROOT)),
        str(PRELOAD.relative_to(ROOT)),
        "scripts/run_playwright_core_browser_type_owner_strict_v2.py",
    }
    assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())

    result = Path(strict["result"])
    assert result.is_absolute() and result.is_file() and not result.is_symlink()
    provenance_path = result.with_suffix(".provenance.json")
    receipt_path = result.with_suffix(".owner-receipt.jsonl")
    log_path = result.with_suffix(".log")
    assert sha256(result) == strict["resultSha256"]
    assert sha256(provenance_path) == strict["provenanceSha256"]
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
    assert provenance["project"] is None
    assert provenance["resultSha256"] == strict["resultSha256"]
    assert provenance["fixtureHost"].endswith(".ts.net")
    assert provenance["exitCode"] == strict["exitCode"]
    selected = selected_tests(json.loads(result.read_text()))
    assert selected == strict["selected"]
    assert strict["exitCode"] == (0 if all(row["status"] == "expected" for row in selected) else 1)
    if strict["mode"] == "candidate":
        assert strict["candidateProfile"] == PROFILE
        assert strict["candidateArtifactSha256"] == ARTIFACT_SHA256
        assert provenance["candidateProfile"] == PROFILE
        assert provenance["candidateArtifactSha256"] == ARTIFACT_SHA256
        assert sha256(receipt_path) == strict["ownerReceiptSha256"]
        records = [json.loads(line) for line in receipt_path.read_text().splitlines() if line]
        assert len(records) >= 3
        assert all(
            set(record) == {"pid", "workerIndex", "candidate", "ownerBound"} for record in records
        )
        assert all(
            record["candidate"] == str(ROOT / "packages/pwright") and record["ownerBound"] is True
            for record in records
        )
        assert {
            record["workerIndex"] for record in records if record["workerIndex"] is not None
        } == {str(row["workerIndex"]) for row in selected}
        assert "[pwright-candidate]" in log_path.read_text()
    else:
        assert strict["candidateProfile"] is None
        assert strict["candidateArtifactSha256"] is None
        assert strict["ownerReceiptSha256"] is None
        assert provenance["candidate"] is None
        assert not receipt_path.exists()
    return strict


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("strict_sidecar", type=Path)
    args = parser.parse_args()
    try:
        strict = verify_strict(args.strict_sidecar)
    except (AssertionError, OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        parser.error(f"strict BrowserType owner evidence invalid: {error}")
    print(
        f"verified strict BrowserType evidence mode={strict['mode']} "
        f"statuses={[row['status'] for row in strict['selected']]}"
    )


if __name__ == "__main__":
    main()
