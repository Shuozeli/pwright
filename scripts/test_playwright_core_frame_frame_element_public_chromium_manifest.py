"""Source-only gates for the complete pinned Core Frame.frameElement file.

These checks do not execute browser tests or attest Pwright/Oracle parity.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path
from typing import cast

from run_playwright_official_conformance import (
    Suite,
    _cross_origin_forward_enabled,
    _pinned_full_file_source,
)

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-frame-frame-element-public-chromium-v1-manifest.json"
)
SOURCE = UPSTREAM / "tests/page/frame-frame-element.spec.ts"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
V18 = ROOT / ".work/official-conformance/outcomes/core-selectors-misc21-strict-paired-v18"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE_SHA256 = "293a4dbe8c6f5468faabe02ce6e9d4f5a1c7862377eb5208c5080277b3fa444c"
IDS_SHA256 = "e40152b56c16f9c34581fc82527de8cdd1572257a9db1ecc3f3cea2368c9482c"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
V18_SHA256 = {
    "candidate": "32fb5464c1131cf42f310f79234ba1833e3c798ab1a76d1b13fb19fca0e85a3e",
    "oracle": "34dee5e0d976505b4b7c29cc7a7a944968965753cc526800f25b01db6eb68179",
}
FIXTURE_INPUT_SHA256 = {
    "tests/page/pageTest.ts": "ba3c28fbfff95bf482fc9647fdd4bc468997575cddcd60507ba0661e64b97b3d",
    "tests/config/utils.ts": "bc5818dc4ec7035ed3c22598f7bf9e8b1bd187de22014574e26f0d1146a38657",
    "tests/assets/empty.html": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "tests/assets/frames/frameset.html": "7e1a6e0d67f83a5533e0630c386aef2830ba402d3d2ffc6fe96809217a5e98e1",
    "tests/assets/frames/frame.html": "ab70ea3dc4c8129241745a7884ee2b321225f341846c475e2540813a2ea8950b",
}
SPEC_IDS = (
    "320fa7c1239dab0b2b33-277125fd267ff47960bd",
    "320fa7c1239dab0b2b33-5ba897de805136998268",
    "320fa7c1239dab0b2b33-ad52ed21af9213d68b5b",
    "320fa7c1239dab0b2b33-6f8dad65f64e44bc798d",
    "320fa7c1239dab0b2b33-91e0512edda1e31f41bc",
    "320fa7c1239dab0b2b33-3f28d1b412e314d6eeb2",
)
RELATIVE_SOURCE = "tests/page/frame-frame-element.spec.ts"
EXPECTED_SUITE: Suite = {
    "id": "library-frame-frame-element-public-chromium",
    "role": "browser-backed-official-file",
    "runner": "packages/playwright/cli.js",
    "config": "tests/library/playwright.config.ts",
    "args": [RELATIVE_SOURCE],
    "candidateAdapter": "test-mode-playwright-core",
    "oracleAdapter": "test-mode-playwright-core-cdp",
    "project": "chromium-page",
    "requiresBrowser": True,
    "expectedTestsPerProject": 6,
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


def test_manifest_pins_complete_upstream_file_and_direct_fixture_inputs() -> None:
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
    assert selection == ("page/frame-frame-element.spec.ts", 6, "chromium-page", IDS_SHA256)
    assert _cross_origin_forward_enabled(suite) is False
    assert "pinnedGrep" not in suite and "pinnedGrepInvert" not in suite
    assert "excludedSpecIds" not in suite
    for relative, digest in FIXTURE_INPUT_SHA256.items():
        assert _sha256(UPSTREAM / relative) == digest


def test_official_list_selects_exactly_six_chromium_page_invocations() -> None:
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
    assert len(ids) == len(set(ids)) == 6
    assert set(ids) == set(SPEC_IDS)
    assert _id_digest(ids) == IDS_SHA256
    assert all(spec["file"] == "page/frame-frame-element.spec.ts" for spec in specs)
    assert all(
        len(cast(list[object], spec["tests"])) == 1
        and cast(list[dict[str, object]], spec["tests"])[0]["projectId"]
        == "chromium-page"
        for spec in specs
    )


def test_inventory_and_frozen_v18_ledgers_have_six_public_not_run_rows() -> None:
    # Arrange
    sources = (
        (INVENTORY, INVENTORY_SHA256),
        *((V18 / f"{lane}.jsonl", digest) for lane, digest in V18_SHA256.items()),
    )

    # Act
    rows_by_source = [(_sha256(path), _selected_rows(path)) for path, _ in sources]

    # Assert
    for (path, digest), (actual_digest, rows) in zip(sources, rows_by_source):
        assert actual_digest == digest, path
        ids = [cast(str, row["specId"]) for row in rows]
        assert len(ids) == len(set(ids)) == 6
        assert set(ids) == set(SPEC_IDS)
        assert _id_digest(ids) == IDS_SHA256
        assert all(row["suite"] == "library" for row in rows)
        assert all(row["sourceClassification"] == "public-api-candidate" for row in rows)
        assert all(row["classificationReasons"] == [] for row in rows)
        assert all(row["expectedStatus"] == "passed" for row in rows)
        if path != INVENTORY:
            assert all(row["outcome"] == "not-run" for row in rows)
            assert all(row["attempts"] == 0 for row in rows)
