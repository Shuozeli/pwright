"""Source-only gates for the complete pinned Core frame-hierarchy file.

These checks do not execute browser tests or attest candidate/Oracle parity.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path
from typing import cast

import pytest
from run_playwright_official_conformance import (
    Suite,
    _cross_origin_forward_enabled,
    _pinned_full_file_source,
)

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-frame-hierarchy-public-chromium-v1-manifest.json"
)
SOURCE = UPSTREAM / "tests/page/frame-hierarchy.spec.ts"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
V20 = (
    ROOT
    / ".work/official-conformance/outcomes/core-selectors-misc21-light-strict-paired-v20"
)
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE_SHA256 = "3bd104830e4cb8f17ed674b5818141b26101d0a2b09be793de974b416f726a7f"
IDS_SHA256 = "7eadee938cc59a04bfc7febb07cea12564972441f5d3742f61e055e062cce1fa"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
V20_SHA256 = {
    "candidate": "2453c74e80e5de5321e1d5b6a1d42cd6c1992cc7714ff76d0bb834848ecc3c8f",
    "oracle": "e9b921670232979db5d0fb3b6dfb045aad5d747c5f63f1385dfe372a9155c2a6",
}
FIXTURE_INPUT_SHA256 = {
    "tests/page/pageTest.ts": "ba3c28fbfff95bf482fc9647fdd4bc468997575cddcd60507ba0661e64b97b3d",
    "tests/config/utils.ts": "bc5818dc4ec7035ed3c22598f7bf9e8b1bd187de22014574e26f0d1146a38657",
    "tests/assets/empty.html": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "tests/assets/frames/nested-frames.html": "d9b5820d68d7425551b0c5b7b3d08f717c9fa46d8465a4b42208f57769ef8bdf",
    "tests/assets/frames/frame.html": "ab70ea3dc4c8129241745a7884ee2b321225f341846c475e2540813a2ea8950b",
    "tests/assets/frames/two-frames.html": "14cbb3de85b237d85b5870805500fa9f91a15cef80c25a65eb05706ca60e4962",
    "tests/assets/frames/frameset.html": "7e1a6e0d67f83a5533e0630c386aef2830ba402d3d2ffc6fe96809217a5e98e1",
    "tests/assets/frames/one-frame.html": "1426a921774b42ff8aa66b06e10ba1500bc0371f5f44f61ad0f4642b94626efe",
    "tests/assets/frames/style.css": "5c78f332a9044b12a3bf3818abd921968834e766bbdf2268b6369a94b1c4a835",
    "tests/assets/frames/script.js": "577ca60854a02937e3be20a0f332174293023325711b2af7df664812bb923f87",
    "tests/assets/shadow.html": "6c545e1581e1395f5ecc44cdd1b6792b5559c640a020383e814cf1d66ea44dc5",
}
SPEC_IDS = (
    "97193df5d3dc5207ad52-8a3bc33b12d920ca3f37",
    "97193df5d3dc5207ad52-4671b258e95cb887a303",
    "97193df5d3dc5207ad52-c0eac18c86514cabfa91",
    "97193df5d3dc5207ad52-6267dcd46f5b0d756d79",
    "97193df5d3dc5207ad52-9bd16410c8167bc4f22e",
    "97193df5d3dc5207ad52-3fe6c068950339dcc208",
    "97193df5d3dc5207ad52-4e2bbdc06d6b8688f700",
    "97193df5d3dc5207ad52-ac3a62ad737929a6c034",
    "97193df5d3dc5207ad52-80280501fd54dc2205b8",
    "97193df5d3dc5207ad52-894584637ea127fb81f9",
    "97193df5d3dc5207ad52-ccd164d67f3cab736f4e",
    "97193df5d3dc5207ad52-dfdca4dd4788c4521b2b",
    "97193df5d3dc5207ad52-0963cc33586674a2564a",
)
RELATIVE_SOURCE = "tests/page/frame-hierarchy.spec.ts"
EXPECTED_SUITE: Suite = {
    "id": "library-frame-hierarchy-public-chromium",
    "role": "browser-backed-official-file",
    "runner": "packages/playwright/cli.js",
    "config": "tests/library/playwright.config.ts",
    "args": [RELATIVE_SOURCE],
    "candidateAdapter": "test-mode-playwright-core",
    "oracleAdapter": "test-mode-playwright-core-cdp",
    "project": "chromium-page",
    "requiresBrowser": True,
    "expectedTestsPerProject": 13,
    "expectedFiles": 1,
    "upstreamTestSourceSha256": SOURCE_SHA256,
    "expectedSpecIdsSha256": IDS_SHA256,
}


def _sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def _selected_rows(path: Path) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    with path.open() as stream:
        for line in stream:
            if f'"sourceFile":"{RELATIVE_SOURCE}"' not in line:
                continue
            row = cast(dict[str, object], json.loads(line))
            if row.get("projectId") == "chromium-page":
                rows.append(row)
    return rows


def _id_digest(ids: list[str]) -> str:
    return hashlib.sha256(
        "".join(f"{spec_id}\n" for spec_id in sorted(ids)).encode()
    ).hexdigest()


def _report_specs(report: dict[str, object]) -> list[dict[str, object]]:
    specs: list[dict[str, object]] = []

    def visit(node: dict[str, object]) -> None:
        specs.extend(cast(list[dict[str, object]], node.get("specs", [])))
        for child in cast(list[dict[str, object]], node.get("suites", [])):
            visit(child)

    for suite in cast(list[dict[str, object]], report["suites"]):
        visit(suite)
    return specs


def test_manifest_pins_complete_upstream_file_and_transitive_assets() -> None:
    # Arrange
    manifest = json.loads(MANIFEST.read_text())
    suite = cast(Suite, manifest["suites"][0])
    revision = subprocess.run(
        ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()

    # Act
    selection = _pinned_full_file_source(suite, UPSTREAM)

    # Assert
    assert manifest == {
        "schema": 1,
        "repository": "https://github.com/microsoft/playwright.git",
        "tag": "v1.58.0",
        "commit": COMMIT,
        "suites": [EXPECTED_SUITE],
    }
    assert revision == COMMIT
    assert _sha256(SOURCE) == SOURCE_SHA256
    assert selection == (
        "page/frame-hierarchy.spec.ts",
        13,
        "chromium-page",
        IDS_SHA256,
    )
    assert _cross_origin_forward_enabled(suite) is False
    assert "pinnedGrep" not in suite and "pinnedGrepInvert" not in suite
    assert "excludedSpecIds" not in suite
    for relative, digest in FIXTURE_INPUT_SHA256.items():
        assert _sha256(UPSTREAM / relative) == digest


def test_manifest_rejects_source_hash_drift() -> None:
    # Arrange
    drifted_suite = dict(EXPECTED_SUITE)
    drifted_suite["upstreamTestSourceSha256"] = "0" * 64

    # Act / Assert
    with pytest.raises(ValueError, match="source drifted"):
        _pinned_full_file_source(cast(Suite, drifted_suite), UPSTREAM)


def test_official_list_selects_exactly_thirteen_chromium_page_invocations() -> None:
    # Arrange
    suite = EXPECTED_SUITE

    # Act
    result = subprocess.run(
        [
            "node",
            str(UPSTREAM / suite["runner"]),
            "test",
            "--list",
            "--reporter=json",
            f"--config={suite['config']}",
            f"--project={suite['project']}",
            *suite["args"],
        ],
        cwd=UPSTREAM,
        check=True,
        capture_output=True,
        text=True,
    )
    report = cast(dict[str, object], json.loads(result.stdout))
    specs = _report_specs(report)
    ids = [cast(str, spec["id"]) for spec in specs]

    # Assert
    assert report["errors"] == []
    assert len(ids) == len(set(ids)) == 13
    assert set(ids) == set(SPEC_IDS)
    assert _id_digest(ids) == IDS_SHA256
    assert all(spec["file"] == "page/frame-hierarchy.spec.ts" for spec in specs)
    assert all(
        len(cast(list[object], spec["tests"])) == 1
        and cast(list[dict[str, object]], spec["tests"])[0]["projectId"]
        == "chromium-page"
        and cast(list[dict[str, object]], spec["tests"])[0]["expectedStatus"]
        == "passed"
        for spec in specs
    )


def test_inventory_and_frozen_v20_ledgers_have_thirteen_public_not_run_rows() -> None:
    # Arrange
    sources = (
        (INVENTORY, INVENTORY_SHA256),
        *((V20 / f"{lane}.jsonl", digest) for lane, digest in V20_SHA256.items()),
    )

    # Act
    rows_by_source = [(_sha256(path), _selected_rows(path)) for path, _ in sources]

    # Assert
    for (path, digest), (actual_digest, rows) in zip(sources, rows_by_source):
        assert actual_digest == digest, path
        ids = [cast(str, row["specId"]) for row in rows]
        assert len(ids) == len(set(ids)) == 13
        assert set(ids) == set(SPEC_IDS)
        assert _id_digest(ids) == IDS_SHA256
        assert all(row["suite"] == "library" for row in rows)
        assert all(
            row["sourceClassification"] == "public-api-candidate" for row in rows
        )
        assert all(row["classificationReasons"] == [] for row in rows)
        assert all(row["expectedStatus"] == "passed" for row in rows)
        if path != INVENTORY:
            assert all(row["outcome"] == "not-run" for row in rows)
            assert all(row["attempts"] == 0 for row in rows)
