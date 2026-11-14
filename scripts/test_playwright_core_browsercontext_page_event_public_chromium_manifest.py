"""Source-only gates for the complete pinned Core Context page-event file.

Collection uses Playwright's --list mode; no browser tests are executed here.
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
    / "tests/official/playwright-core-browsercontext-page-event-public-chromium-v1-manifest.json"
)
RELATIVE_SOURCE = "tests/library/browsercontext-page-event.spec.ts"
SOURCE = UPSTREAM / RELATIVE_SOURCE
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE_SHA256 = "79de7a683bee11300a2ec0a037324c2ce18be2d0331db491588041ed61203ffb"
IDS_SHA256 = "a4a81e3b48b514fe6aa9fa18e36b2dd716b12e9d155594dae1ee6010c1bd0524"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
FIXTURE_INPUT_SHA256 = {
    "tests/library/playwright.config.ts": "338eb705bb4a17322dfba39d1c6770613bf8cf62cfec5c43bf7cece3e5db8146",
    "tests/config/browserTest.ts": "0ef5440e164d262e7c3d01523448b5d3c4d50b99060604c20f168f04419a13d5",
    "tests/config/baseTest.ts": "4fc41689120135a7cfbe066431ebc5b3751b1a6745e2d48e7013336da556a52c",
    "tests/config/serverFixtures.ts": "4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b",
    "tests/config/testserver/index.ts": "4330e0b94c88fdba07401efd3f4cd30a229bb48da037d8c880cd79ba0bae7307",
    "tests/config/utils.ts": "bc5818dc4ec7035ed3c22598f7bf9e8b1bd187de22014574e26f0d1146a38657",
    "tests/assets/empty.html": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "tests/assets/one-style.html": "998be9a4b3854e1b75c85a58debfcc5c59736af1dfd30e7900dd3cdd901a6e86",
    "tests/assets/one-style.css": "aae2798dbe4c81fb3b467df66899ef72fed0d732490cff8cf0df4717ac76697e",
    "tests/assets/injectedstyle.css": "3a77baf888e5eef1912bc0aa53871f57599efb8db801cbcf2e1bae8914d64229",
    "tests/assets/popup/window-open.html": "9cd086462febb5ecbaa09a8a058235715f7d843436337c4ca8d80cc420066b3e",
    "tests/assets/popup/popup.html": "f191d411a2503e2209fef15fa478830ee01fd30f92a4cd1cb9c53623affc5f85",
}
CASES = (
    (20, "should have url", "3b0c3bb67b2946d1289f-2e08bf926a08423cf507"),
    (31, "should have url after domcontentloaded", "3b0c3bb67b2946d1289f-a795ec2363da18cbbf60"),
    (43, "should have about:blank url with domcontentloaded", "3b0c3bb67b2946d1289f-2213e0aed63352c9eb17"),
    (55, "should have about:blank for empty url with domcontentloaded", "3b0c3bb67b2946d1289f-cf334f94ac6529ce19de"),
    (67, "should report when a new page is created and closed", "3b0c3bb67b2946d1289f-230fdeb294c2d0333e53"),
    (94, "should report initialized pages", "3b0c3bb67b2946d1289f-f408cec555aa51a69409"),
    (109, "should not crash while redirecting of original request was missed", "3b0c3bb67b2946d1289f-b34a49722f76f15c783f"),
    (129, "should have an opener", "3b0c3bb67b2946d1289f-df0da34874dbe3a0d0ac"),
    (143, "should fire page lifecycle events", "3b0c3bb67b2946d1289f-c6be120c1879e10e5cdc"),
    (160, "should work with Shift-clicking", "3b0c3bb67b2946d1289f-04dff7a920fe787de9f3"),
    (173, "should work with Ctrl-clicking", "3b0c3bb67b2946d1289f-e94d0addcf1fce929d15"),
)
EXPECTED_SUITE: Suite = {
    "id": "library-browsercontext-page-event-public-chromium",
    "role": "browser-backed-official-file",
    "runner": "packages/playwright/cli.js",
    "config": "tests/library/playwright.config.ts",
    "args": [RELATIVE_SOURCE],
    "candidateAdapter": "test-mode-playwright-core",
    "oracleAdapter": "test-mode-playwright-core-cdp",
    "project": "chromium-library",
    "requiresBrowser": True,
    "expectedTestsPerProject": 11,
    "expectedFiles": 1,
    "upstreamTestSourceSha256": SOURCE_SHA256,
    "expectedSpecIdsSha256": IDS_SHA256,
}


def _sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


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


def test_manifest_pins_complete_source_and_exact_fixture_assets() -> None:
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
    fixture_hashes = {
        relative: _sha256(UPSTREAM / relative)
        for relative in FIXTURE_INPUT_SHA256
    }

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
        "library/browsercontext-page-event.spec.ts",
        11,
        "chromium-library",
        IDS_SHA256,
    )
    assert fixture_hashes == FIXTURE_INPUT_SHA256
    assert "./one-style.css" in (UPSTREAM / "tests/assets/one-style.html").read_text()
    assert "./popup.html" in (UPSTREAM / "tests/assets/popup/window-open.html").read_text()
    assert _cross_origin_forward_enabled(suite) is False
    assert "pinnedGrep" not in suite and "pinnedGrepInvert" not in suite
    assert "excludedSpecIds" not in suite


def test_manifest_rejects_upstream_source_drift() -> None:
    # Arrange
    drifted = cast(Suite, {**EXPECTED_SUITE, "upstreamTestSourceSha256": "0" * 64})

    # Act / Assert
    with pytest.raises(ValueError, match="source drifted"):
        _pinned_full_file_source(drifted, UPSTREAM)


def test_official_list_selects_all_eleven_unchanged_chromium_library_cases() -> None:
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
    selected = sorted(
        (cast(int, spec["line"]), cast(str, spec["title"]), cast(str, spec["id"]))
        for spec in specs
    )

    # Assert
    assert report["errors"] == []
    assert selected == sorted(CASES)
    assert len(specs) == len({spec["id"] for spec in specs}) == 11
    assert _id_digest([cast(str, spec["id"]) for spec in specs]) == IDS_SHA256
    assert all(spec["file"] == "library/browsercontext-page-event.spec.ts" for spec in specs)
    assert all(
        len(cast(list[object], spec["tests"])) == 1
        and cast(list[dict[str, object]], spec["tests"])[0]["projectId"]
        == "chromium-library"
        and cast(list[dict[str, object]], spec["tests"])[0]["expectedStatus"]
        == "passed"
        for spec in specs
    )


def test_inventory_classifies_exact_eleven_ids_as_public_api_candidates() -> None:
    # Arrange
    rows: list[dict[str, object]] = []
    with INVENTORY.open() as stream:
        for line in stream:
            if f'"sourceFile":"{RELATIVE_SOURCE}"' not in line:
                continue
            row = cast(dict[str, object], json.loads(line))
            if row.get("projectId") == "chromium-library":
                rows.append(row)

    # Act
    selected = sorted(
        (
            cast(int, row["line"]),
            cast(list[str], row["titlePath"])[-1],
            cast(str, row["specId"]),
        )
        for row in rows
    )

    # Assert
    assert _sha256(INVENTORY) == INVENTORY_SHA256
    assert selected == sorted(CASES)
    assert len(rows) == 11
    assert all(row["suite"] == "library" for row in rows)
    assert all(row["sourceClassification"] == "public-api-candidate" for row in rows)
    assert all(row["classificationReasons"] == [] for row in rows)
    assert all(row["expectedStatus"] == "passed" for row in rows)
