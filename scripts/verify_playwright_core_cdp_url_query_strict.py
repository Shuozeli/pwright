#!/usr/bin/env python3
"""Verify source-pinned browser-free Core CDP URL-query evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from run_playwright_core_cdp_headers_strict import (
    ARTIFACT_SHA256,
    COMMIT,
    PROFILE,
    ROOT,
    SERVER_FIXTURE_SHA256,
    SOURCE,
    SOURCE_SHA256,
    UPSTREAM,
    sha256,
)
from run_playwright_core_cdp_url_query_strict import (
    COMMON,
    MANIFEST,
    PRELOAD,
    SUITE,
    check_manifest,
    selected_tests,
)
from verify_playwright_official_provenance import verify_provenance


def verify_strict(strict_path: Path) -> dict:
    strict = json.loads(strict_path.read_text())
    assert set(strict) == {
        "schema",
        "scope",
        "mode",
        "upstreamCommit",
        "upstreamSourceSha256",
        "serverFixtureSha256",
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
    assert strict["schema"] == 1
    assert strict["scope"] == "unchanged-upstream-assertions-browser-free-fixture-host-adaptation"
    assert strict["mode"] in ("candidate", "oracle")
    assert strict["upstreamCommit"] == COMMIT
    assert strict["upstreamSourceSha256"] == SOURCE_SHA256
    assert strict["serverFixtureSha256"] == SERVER_FIXTURE_SHA256
    check_manifest()
    inputs = strict["inputs"]
    assert set(inputs) == {
        str(MANIFEST.relative_to(ROOT)),
        str(PRELOAD.relative_to(ROOT)),
        str(COMMON.relative_to(ROOT)),
        "scripts/run_playwright_core_cdp_url_query_strict.py",
    }
    assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())

    result = Path(strict["result"])
    assert result.is_absolute() and result.is_file() and not result.is_symlink()
    assert sha256(result) == strict["resultSha256"]
    provenance_path = result.with_suffix(".provenance.json")
    receipt_path = result.with_suffix(".guard-receipt.jsonl")
    log_path = result.with_suffix(".log")
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
    assert provenance["fixtureHost"].endswith(".ts.net")
    assert provenance["resultSha256"] == strict["resultSha256"]
    assert provenance["exitCode"] == strict["exitCode"]
    if strict["mode"] == "candidate":
        assert strict["candidateProfile"] == PROFILE
        assert strict["candidateArtifactSha256"] == ARTIFACT_SHA256
        assert provenance["candidateProfile"] == PROFILE
        assert provenance["candidateArtifactSha256"] == ARTIFACT_SHA256
        assert "[pwright-candidate]" in log_path.read_text()
    else:
        assert strict["candidateProfile"] is None
        assert strict["candidateArtifactSha256"] is None
        assert provenance["candidate"] is None

    selected = selected_tests(json.loads(result.read_text()))
    assert selected == strict["selected"]
    assert strict["exitCode"] == (0 if all(x["status"] == "expected" for x in selected) else 1)
    host = provenance["fixtureHost"]
    source = (UPSTREAM / SOURCE).read_text()
    with_slash = (
        "browserType.connectOverCDP(`http://localhost:${server.PORT}/browser/?foo=bar`)"
        ".catch(() => {})"
    )
    without_slash = (
        "browserType.connectOverCDP(`http://localhost:${server.PORT}/browser?foo=bar`)"
        ".catch(() => {})"
    )
    assert source.count(with_slash) == source.count(without_slash) == 1
    adapted = source.replace(
        with_slash,
        f"browserType.connectOverCDP(`http://{host}:${{server.PORT}}/browser/?foo=bar`)"
        ".catch(() => {})",
    ).replace(
        without_slash,
        f"browserType.connectOverCDP(`http://{host}:${{server.PORT}}/browser?foo=bar`)"
        ".catch(() => {})",
    )
    adapted_sha256 = hashlib.sha256(adapted.encode()).hexdigest()
    records = [json.loads(line) for line in receipt_path.read_text().splitlines() if line]
    assert records and any(item["workerIndex"] == "0" for item in records)
    assert all(
        set(item)
        == {
            "pid",
            "workerIndex",
            "sourceSha256",
            "adaptedSha256",
            "fixtureHost",
            "replacements",
        }
        for item in records
    )
    assert all(
        item["sourceSha256"] == SOURCE_SHA256
        and item["adaptedSha256"] == adapted_sha256
        and item["fixtureHost"] == host
        and item["replacements"] == {"urlWithSlash": 1, "urlWithoutSlash": 1}
        for item in records
    )
    return strict


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("strict_sidecar", type=Path)
    args = parser.parse_args()
    try:
        strict = verify_strict(args.strict_sidecar)
    except (AssertionError, OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        parser.error(f"strict Core CDP URL evidence invalid: {error}")
    print(
        f"verified strict Core CDP URL evidence mode={strict['mode']} "
        f"statuses={[test['status'] for test in strict['selected']]}"
    )


if __name__ == "__main__":
    main()
