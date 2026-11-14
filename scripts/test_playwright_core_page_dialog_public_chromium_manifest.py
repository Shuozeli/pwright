"""Source and selection gates for the complete pinned Core Page dialog file.

These checks do not execute browser tests or attest conformance outcomes.
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
SOURCE = UPSTREAM / "tests/page/page-dialog.spec.ts"
MANIFEST = (
    ROOT / "tests/official/playwright-core-page-dialog-public-chromium-v1-manifest.json"
)
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE_SHA256 = "b758da2196bf60d560528fc77eec6dfef23e191de9211fb0e6570909a61e0d7f"
IDS_SHA256 = "bc8293399b2b2f323a1473312971d19cf9adc2920cac2dfd9c1e98e4ba68cb50"
SPEC_IDS = (
    "4234be2f84dbe1e7b3b6-0b499d2688cbc5d2e5dd",
    "4234be2f84dbe1e7b3b6-ec008d483744cb48339d",
    "4234be2f84dbe1e7b3b6-8afcd88777ba8e40af67",
    "4234be2f84dbe1e7b3b6-7d769c817525eaae6525",
    "4234be2f84dbe1e7b3b6-dcf281d3e5e7d8d823f8",
    "4234be2f84dbe1e7b3b6-bc3329a3363d6fae8c10",
    "4234be2f84dbe1e7b3b6-2bd38bfcbfa1ab44efcc",
    "4234be2f84dbe1e7b3b6-7f429de1d812f294dd7c",
    "4234be2f84dbe1e7b3b6-86b9cbd0315be32715a9",
    "4234be2f84dbe1e7b3b6-2e609ae90079fb25b1cd",
)
EXPECTED_SUITE: Suite = {
    "id": "library-page-dialog-public-chromium",
    "role": "browser-backed-official-file",
    "runner": "packages/playwright/cli.js",
    "config": "tests/library/playwright.config.ts",
    "args": ["tests/page/page-dialog.spec.ts"],
    "candidateAdapter": "test-mode-playwright-core",
    "oracleAdapter": "test-mode-playwright-core-cdp",
    "project": "chromium-page",
    "requiresBrowser": True,
    "expectedTestsPerProject": 10,
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
            "file": "page/page-dialog.spec.ts",
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
    assert source == ("page/page-dialog.spec.ts", 10, "chromium-page", IDS_SHA256)
    assert _cross_origin_forward_enabled(suite) is False


def test_inventory_contains_exactly_ten_public_chromium_ids(suite: Suite) -> None:
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
                and row.get("sourceFile") == "tests/page/page-dialog.spec.ts"
            ):
                rows.append(row)

    # Act
    ids = [cast(str, row["specId"]) for row in rows]
    digest = hashlib.sha256(
        "".join(f"{spec_id}\n" for spec_id in sorted(ids)).encode()
    ).hexdigest()

    # Assert
    assert _sha256(INVENTORY) == INVENTORY_SHA256
    assert len(ids) == len(set(ids)) == suite["expectedTestsPerProject"] == 10
    assert set(ids) == set(SPEC_IDS)
    assert digest == suite["expectedSpecIdsSha256"] == IDS_SHA256
    assert all(row["sourceClassification"] == "public-api-candidate" for row in rows)
    assert all(row["expectedStatus"] == "passed" for row in rows)


def test_runner_accepts_executed_red_without_relabeling_it_green(
    suite: Suite, tmp_path: Path
) -> None:
    # Arrange
    report = tmp_path / "executed-red.json"
    report.write_text(json.dumps(_executed_report()))

    # Act
    count = _verify_pinned_full_file_result_selection(report, suite, UPSTREAM)

    # Assert
    assert count == 10
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
    with pytest.raises(ValueError, match="expected 10 unique"):
        _verify_pinned_full_file_result_selection(result, ids_drift, UPSTREAM)
