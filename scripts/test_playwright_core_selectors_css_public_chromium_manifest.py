"""Source and selection gates for the complete pinned Core CSS selectors file.

These checks are static: they do not execute browser tests or attest parity.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
from copy import deepcopy
from pathlib import Path
from typing import cast

import pytest
from run_playwright_official_conformance import (
    Suite,
    _cross_origin_forward_enabled,
    _pinned_full_file_source,
    _verify_pinned_full_file_result_selection,
)

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
SOURCE = UPSTREAM / "tests/page/selectors-css.spec.ts"
DEEP_SHADOW_ASSET = UPSTREAM / "tests/assets/deep-shadow.html"
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-selectors-css-public-chromium-v1-manifest.json"
)
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE_SHA256 = "f0a096c63febc7b2486dc15d3c26aca4f24f2beaa4f5ad1dc96b6eb5a1ba68f5"
DEEP_SHADOW_ASSET_SHA256 = (
    "a9dffedc4fbdfd7aeabfed432af48fb7b643496af5fc3c7808ad4a85bd9c9616"
)
IDS_SHA256 = "72b5dc160dfd74f0ff5a8f218dab4eff7a0cd22270e116d64b6101f5e7e6ba39"
SPEC_IDS = (
    "8032b197489196edfc05-86a5bccce87dfa6e0fe8",
    "8032b197489196edfc05-0e6fb681dc73f479cfc2",
    "8032b197489196edfc05-bf5716de0880e707b895",
    "8032b197489196edfc05-265d6de255beb9e51082",
    "8032b197489196edfc05-4795ba70e8259d015afb",
    "8032b197489196edfc05-87170c13ef3eead59327",
    "8032b197489196edfc05-4a08d52f3647c32e0b64",
    "8032b197489196edfc05-f72f47a302a52eb3f539",
    "8032b197489196edfc05-3bad72fa818f00da6b5c",
    "8032b197489196edfc05-2ea61140463b8a155f0d",
    "8032b197489196edfc05-92eb679aa1f76cebd808",
    "8032b197489196edfc05-2d387bb349bc25ecddc8",
    "8032b197489196edfc05-a8a607eae54401d65656",
    "8032b197489196edfc05-35ddbb088c8b7946ae32",
    "8032b197489196edfc05-94fd0e63b2a93b09a05c",
    "8032b197489196edfc05-843de30ce82a86e345ba",
    "8032b197489196edfc05-dcb07beb3b55955c8ae5",
    "8032b197489196edfc05-3aba2923ed9dedb01ee1",
    "8032b197489196edfc05-98be0c3a89c425cbb75a",
    "8032b197489196edfc05-032250e06ed85bfa6328",
    "8032b197489196edfc05-867f0965a3d06084e573",
    "8032b197489196edfc05-24c1128fbe5a3e7da75c",
    "8032b197489196edfc05-5b560ddda30d8b8dab22",
    "8032b197489196edfc05-85b1f4af37f363e61f92",
    "8032b197489196edfc05-ca3b2b07dfcbeeb08a52",
    "8032b197489196edfc05-98c05572825a4f8d169a",
    "8032b197489196edfc05-f611ee02ca3b63537109",
)
EXPECTED_SUITE: Suite = {
    "id": "library-selectors-css-public-chromium",
    "role": "browser-backed-official-file",
    "runner": "packages/playwright/cli.js",
    "config": "tests/library/playwright.config.ts",
    "args": ["tests/page/selectors-css.spec.ts"],
    "candidateAdapter": "test-mode-playwright-core",
    "oracleAdapter": "test-mode-playwright-core-cdp",
    "project": "chromium-page",
    "requiresBrowser": True,
    "expectedTestsPerProject": 27,
    "expectedFiles": 1,
    "upstreamTestSourceSha256": SOURCE_SHA256,
    "expectedSpecIdsSha256": IDS_SHA256,
}


def _sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


@pytest.fixture(scope="module")
def suite() -> Suite:
    if not SOURCE.is_file():
        pytest.skip("pinned Playwright v1.58.0 checkout is unavailable")
    return cast(Suite, json.loads(MANIFEST.read_text())["suites"][0])


def _executed_report(ids: tuple[str, ...] = SPEC_IDS) -> dict[str, object]:
    specs = [
        {
            "id": spec_id,
            "file": "page/selectors-css.spec.ts",
            "tests": [
                {
                    "projectId": "chromium-page",
                    "status": "unexpected" if index == 0 else "expected",
                    "results": [{"status": "failed" if index == 0 else "passed"}],
                }
            ],
        }
        for index, spec_id in enumerate(ids)
    ]
    return {"suites": [{"specs": specs}]}


def test_manifest_pins_one_complete_chromium_public_file(suite: Suite) -> None:
    # Arrange
    manifest = json.loads(MANIFEST.read_text())
    revision = subprocess.run(
        ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()

    # Act
    source = _pinned_full_file_source(suite, UPSTREAM)

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
    assert source == ("page/selectors-css.spec.ts", 27, "chromium-page", IDS_SHA256)
    assert _cross_origin_forward_enabled(suite) is False


def test_only_default_http_asset_is_required(suite: Suite) -> None:
    # Arrange
    source = SOURCE.read_text()

    # Act
    referenced_asset = source.count("server.PREFIX + '/deep-shadow.html'")

    # Assert
    assert referenced_asset == 8
    assert "server.CROSS_PROCESS_PREFIX" not in source
    assert "server.setRoute(" not in source
    assert "httpsServer" not in source
    assert _sha256(DEEP_SHADOW_ASSET) == DEEP_SHADOW_ASSET_SHA256
    assert _cross_origin_forward_enabled(suite) is False


def test_inventory_contains_exactly_twenty_seven_public_chromium_ids(
    suite: Suite,
) -> None:
    # Arrange
    if not INVENTORY.is_file():
        pytest.skip("pinned full Core source inventory is unavailable")
    rows: list[dict[str, object]] = []
    with INVENTORY.open() as stream:
        for line in stream:
            row = json.loads(line)
            if (
                row.get("suite") == "library"
                and row.get("projectId") == "chromium-page"
                and row.get("sourceFile") == "tests/page/selectors-css.spec.ts"
            ):
                rows.append(row)

    # Act
    ids = [cast(str, row["specId"]) for row in rows]
    digest = hashlib.sha256(
        "".join(f"{spec_id}\n" for spec_id in sorted(ids)).encode()
    ).hexdigest()

    # Assert
    assert _sha256(INVENTORY) == INVENTORY_SHA256
    assert len(ids) == len(set(ids)) == suite["expectedTestsPerProject"] == 27
    assert set(ids) == set(SPEC_IDS)
    assert digest == suite["expectedSpecIdsSha256"] == IDS_SHA256
    assert all(row["sourceClassification"] == "public-api-candidate" for row in rows)
    assert all(row["expectedStatus"] == "passed" for row in rows)


def test_runner_accepts_complete_executed_red_without_relabeling_it_green(
    suite: Suite, tmp_path: Path
) -> None:
    # Arrange
    report = tmp_path / "executed-red.json"
    report.write_text(json.dumps(_executed_report()))

    # Act
    count = _verify_pinned_full_file_result_selection(report, suite, UPSTREAM)

    # Assert
    assert count == 27
    assert (
        json.loads(report.read_text())["suites"][0]["specs"][0]["tests"][0]["status"]
        == "unexpected"
    )


@pytest.mark.parametrize(
    "mutation", ["missing", "duplicate", "wrong-project", "no-attempt"]
)
def test_runner_rejects_partial_or_invalid_selection(
    suite: Suite, tmp_path: Path, mutation: str
) -> None:
    # Arrange
    report = _executed_report()
    specs = report["suites"][0]["specs"]
    if mutation == "missing":
        specs.pop()
    elif mutation == "duplicate":
        specs[-1] = deepcopy(specs[0])
    elif mutation == "wrong-project":
        specs[0]["tests"][0]["projectId"] = "firefox-page"
    else:
        specs[0]["tests"][0]["results"] = []
    result = tmp_path / f"{mutation}.json"
    result.write_text(json.dumps(report))

    # Act / Assert
    with pytest.raises(ValueError, match="pinned full-file Core result"):
        _verify_pinned_full_file_result_selection(result, suite, UPSTREAM)


def test_runner_rejects_source_and_expected_ids_drift(
    suite: Suite, tmp_path: Path
) -> None:
    # Arrange
    source_drift = cast(Suite, {**suite, "upstreamTestSourceSha256": "0" * 64})
    ids_drift = cast(Suite, {**suite, "expectedSpecIdsSha256": "0" * 64})
    result = tmp_path / "executed-red.json"
    result.write_text(json.dumps(_executed_report()))

    # Act / Assert
    with pytest.raises(ValueError, match="source drifted"):
        _pinned_full_file_source(source_drift, UPSTREAM)
    with pytest.raises(ValueError, match="expected 27 unique"):
        _verify_pinned_full_file_result_selection(result, ids_drift, UPSTREAM)
