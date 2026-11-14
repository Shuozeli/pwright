"""Verify source-pinned official Core CDP default-User-Agent evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from run_playwright_core_cdp_default_user_agent_strict_v2 import (
    COMMIT,
    MANIFEST,
    PRELOAD,
    PROFILE,
    ROOT,
    SCOPE,
    SERVER_FIXTURE_SHA256,
    SOURCE,
    SOURCE_SHA256,
    SUITE,
    UPSTREAM,
    check_source,
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


def adapted_source_sha256(host: str) -> str:
    source = (UPSTREAM / SOURCE).read_text()
    title = "test('should send default User-Agent header with connect request'"
    marker = "wsEndpoint: `ws://localhost:${server.PORT}/ws`"
    assert source.count(title) == 1
    start = source.index(title)
    next_test = source.find("\ntest(", start + len(title))
    end = len(source) if next_test < 0 else next_test
    body = source[start:end]
    assert body.count(marker) == 1
    changed = body.replace(marker, f"wsEndpoint: `ws://{host}:${{server.PORT}}/ws`")
    adapted = source[:start] + changed + source[end:]
    return hashlib.sha256(adapted.encode()).hexdigest()


def verify_strict(strict_path: Path) -> dict:
    assert strict_path.is_file() and not strict_path.is_symlink()
    strict = json.loads(strict_path.read_text())
    assert isinstance(strict, dict) and set(strict) == EXPECTED_FIELDS
    assert strict["schema"] == 2 and strict["scope"] == SCOPE
    assert strict["mode"] in ("candidate", "oracle")
    assert strict["upstreamCommit"] == COMMIT
    assert strict["upstreamSourceSha256"] == SOURCE_SHA256
    assert strict["serverFixtureSha256"] == SERVER_FIXTURE_SHA256
    check_source()
    inputs = strict["inputs"]
    assert isinstance(inputs, dict) and set(inputs) == {
        str(MANIFEST.relative_to(ROOT)),
        str(PRELOAD.relative_to(ROOT)),
        "scripts/run_playwright_core_cdp_default_user_agent_strict_v2.py",
    }
    assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())
    result = Path(strict["result"])
    assert result.is_absolute() and result.is_file() and not result.is_symlink()
    assert strict_path.resolve() == result.with_suffix(".strict.json").resolve()
    assert sha256(result) == strict["resultSha256"]
    provenance_path = result.with_suffix(".provenance.json")
    receipt_path = result.with_suffix(".guard-receipt.jsonl")
    log_path = result.with_suffix(".log")
    for path in (provenance_path, receipt_path, log_path):
        assert path.is_file() and not path.is_symlink()
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
    assert provenance["filters"] == [f"{SOURCE}:299"]
    assert provenance["fixtureHost"].endswith(".ts.net")
    assert provenance["resultSha256"] == strict["resultSha256"]
    assert provenance["exitCode"] == strict["exitCode"]
    log = log_path.read_text()
    if strict["mode"] == "candidate":
        assert strict["candidateProfile"] == PROFILE
        assert isinstance(strict["candidateArtifactSha256"], str)
        artifact = ROOT / "packages/pwright/native" / PROFILE / "pwright.node"
        assert (
            artifact.is_file() and sha256(artifact) == strict["candidateArtifactSha256"]
        )
        assert provenance["candidateProfile"] == PROFILE
        assert (
            provenance["candidateArtifactSha256"] == strict["candidateArtifactSha256"]
        )
        assert provenance["candidate"] is not None
        assert "[pwright-candidate]" in log
    else:
        assert strict["candidateProfile"] is None
        assert strict["candidateArtifactSha256"] is None
        assert provenance["candidate"] is None
        assert "[pwright-candidate]" not in log
    selected = selected_tests(json.loads(result.read_text()))
    assert selected == strict["selected"]
    assert strict["exitCode"] == (0 if selected[0]["status"] == "expected" else 1)
    expected_adapted_sha256 = adapted_source_sha256(provenance["fixtureHost"])
    lines = receipt_path.read_text().splitlines()
    assert lines and all(line for line in lines)
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
        and record["adaptedSha256"] == expected_adapted_sha256
        and record["fixtureHost"] == provenance["fixtureHost"]
        and record["replacements"] == {"websocket": 1}
        for record in records
    )
    return strict


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("strict_sidecar", type=Path)
    args = parser.parse_args()
    try:
        strict = verify_strict(args.strict_sidecar)
    except (
        AssertionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"strict Core CDP default User-Agent evidence invalid: {error}")
    print(
        f"verified strict Core CDP default User-Agent evidence mode={strict['mode']} "
        f"status={strict['selected'][0]['status']}"
    )


if __name__ == "__main__":
    main()
