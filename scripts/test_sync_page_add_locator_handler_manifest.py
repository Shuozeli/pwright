"""Check the complete public Chromium sync locator-handler Python file."""

from __future__ import annotations

import ast
import hashlib
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[1]
SOURCE = "tests/sync/test_page_add_locator_handler.py"
MANIFEST = (
    ROOT
    / "packages/pwright-python/tests/official/"
    "sync-page-add-locator-handler-full-chromium-v1-manifest.json"
)
UPSTREAM = Path.home() / "projects/thirdparty/playwright-python"
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
OUTCOMES = (
    ROOT
    / ".work/official-conformance/outcomes/"
    "playwright-python-candidate-headers-popup26-element-state22-20261011-v2.jsonl",
    ROOT
    / ".work/official-conformance/outcomes/"
    "playwright-python-oracle-element-state22-20261011-v2.jsonl",
)


def _chromium_rows(path: Path) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    with path.open() as stream:
        for line in stream:
            if f'"file":"{SOURCE}"' in line:
                row = json.loads(line)
                if str(row["id"]).endswith("[chromium]"):
                    rows.append(row)
    return rows


class SyncPageAddLocatorHandlerManifestTests(unittest.TestCase):
    def test_manifest_pins_the_complete_upstream_file_and_fixtures(self) -> None:
        # Arrange
        manifest = json.loads(MANIFEST.read_text())
        source_path = UPSTREAM / SOURCE
        if not source_path.is_file():
            self.skipTest("pinned upstream checkout is unavailable")

        # Act
        selected = {case["id"] for case in manifest["cases"]}
        tree = ast.parse(source_path.read_bytes())
        source_ids = {
            f"{SOURCE}::{node.name}"
            for node in tree.body
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
            and node.name.startswith("test_")
        }

        # Assert
        self.assertEqual(manifest["schema"], 1)
        self.assertEqual(manifest["tag"], "v1.58.0")
        self.assertEqual(
            manifest["commit"], "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
        )
        self.assertEqual(manifest["execution"], {
            "requiresBrowser": True,
            "requiresFixtureServers": True,
        })
        self.assertEqual(len(manifest["cases"]), 13)
        self.assertEqual(selected, source_ids)
        self.assertTrue(all(case["capabilities"] for case in manifest["cases"]))
        for relative, expected in manifest["files"].items():
            self.assertEqual(
                hashlib.sha256((UPSTREAM / relative).read_bytes()).hexdigest(),
                expected,
            )

    def test_chromium_inventory_matches_every_manifest_case(self) -> None:
        if not INVENTORY.is_file():
            self.skipTest("pinned inventory is unavailable")

        # Arrange
        manifest = json.loads(MANIFEST.read_text())
        expected = {f"{case['id']}[chromium]" for case in manifest["cases"]}

        # Act
        rows = _chromium_rows(INVENTORY)

        # Assert
        self.assertEqual(len(rows), 13)
        self.assertEqual({str(row["id"]) for row in rows}, expected)
        self.assertTrue(all(row["api"] == "sync" for row in rows))
        self.assertTrue(
            all(row["sourceClassification"] == "public-api-source" for row in rows)
        )

    def test_both_strict_ledgers_leave_all_thirteen_cases_unrun(self) -> None:
        if not all(path.is_file() for path in OUTCOMES):
            self.skipTest("frozen strict ledgers are unavailable")

        # Arrange
        manifest = json.loads(MANIFEST.read_text())
        expected = {f"{case['id']}[chromium]" for case in manifest["cases"]}

        # Act
        selections = [_chromium_rows(path) for path in OUTCOMES]

        # Assert
        for rows in selections:
            self.assertEqual(len(rows), 13)
            self.assertEqual({str(row["id"]) for row in rows}, expected)
            self.assertTrue(all(row["outcome"] == "not-run" for row in rows))
            self.assertTrue(all(row["attempts"] == 0 for row in rows))


if __name__ == "__main__":
    unittest.main()
