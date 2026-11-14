"""Check the complete public Chromium async and sync CDP-session files."""

from __future__ import annotations

import ast
import hashlib
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[1]
UPSTREAM = Path.home() / "projects/thirdparty/playwright-python"
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
OUTCOMES = (
    ROOT / ".work/official-conformance/outcomes/"
    "playwright-python-candidate-fence-pred-intercept14-20261011-v7.jsonl",
    ROOT / ".work/official-conformance/outcomes/"
    "playwright-python-oracle-intercept14-20261011-v5.jsonl",
)
SOURCES = {
    "async": ("tests/async/test_cdp_session.py", 6),
    "sync": ("tests/sync/test_cdp_session.py", 5),
}


def _manifest(api: str) -> dict[str, object]:
    path = (
        ROOT / "packages/pwright-python/tests/official/"
        f"{api}-cdp-session-full-chromium-v1-manifest.json"
    )
    return json.loads(path.read_text())


def _chromium_rows(path: Path, source: str) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    with path.open() as stream:
        for line in stream:
            if f'"file":"{source}"' in line:
                row = json.loads(line)
                if str(row["id"]).endswith("[chromium]"):
                    rows.append(row)
    return rows


class CdpSessionFullChromiumManifestTests(unittest.TestCase):
    def test_manifests_pin_complete_upstream_files_and_fixtures(self) -> None:
        if not UPSTREAM.is_dir():
            self.skipTest("pinned upstream checkout is unavailable")

        for api, (source, count) in SOURCES.items():
            with self.subTest(api=api):
                # Arrange
                manifest = _manifest(api)
                tree = ast.parse((UPSTREAM / source).read_bytes())

                # Act
                cases = manifest["cases"]
                selected = {case["id"] for case in cases}
                source_ids = {
                    f"{source}::{node.name}"
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
                self.assertEqual(
                    manifest["execution"],
                    {"requiresBrowser": True, "requiresFixtureServers": True},
                )
                self.assertEqual(len(cases), count)
                self.assertEqual(selected, source_ids)
                self.assertTrue(all(case["capabilities"] for case in cases))
                self.assertTrue(
                    all(f"{api}-api" in case["capabilities"] for case in cases)
                )
                for relative, expected in manifest["files"].items():
                    self.assertEqual(
                        hashlib.sha256((UPSTREAM / relative).read_bytes()).hexdigest(),
                        expected,
                    )

    def test_chromium_inventory_matches_every_manifest_case(self) -> None:
        if not INVENTORY.is_file():
            self.skipTest("pinned inventory is unavailable")

        for api, (source, count) in SOURCES.items():
            with self.subTest(api=api):
                # Arrange
                manifest = _manifest(api)
                expected = {f"{case['id']}[chromium]" for case in manifest["cases"]}

                # Act
                rows = _chromium_rows(INVENTORY, source)

                # Assert
                self.assertEqual(len(rows), count)
                self.assertEqual({str(row["id"]) for row in rows}, expected)
                self.assertTrue(all(row["api"] == api for row in rows))
                self.assertTrue(
                    all(
                        row["sourceClassification"] == "public-api-source"
                        for row in rows
                    )
                )
                self.assertTrue(all(row["privateImports"] == [] for row in rows))

    def test_both_strict_ledgers_leave_every_case_unrun(self) -> None:
        if not all(path.is_file() for path in OUTCOMES):
            self.skipTest("frozen strict ledgers are unavailable")

        for api, (source, count) in SOURCES.items():
            # Arrange
            manifest = _manifest(api)
            expected = {f"{case['id']}[chromium]" for case in manifest["cases"]}

            for path in OUTCOMES:
                with self.subTest(api=api, ledger=path.name):
                    # Act
                    rows = _chromium_rows(path, source)

                    # Assert
                    self.assertEqual(len(rows), count)
                    self.assertEqual({str(row["id"]) for row in rows}, expected)
                    self.assertTrue(all(row["outcome"] == "not-run" for row in rows))
                    self.assertTrue(all(row["attempts"] == 0 for row in rows))


if __name__ == "__main__":
    unittest.main()
