#!/usr/bin/env python3
"""Browserless checks for the next pinned public-API full-file selections."""

from __future__ import annotations

import ast
import hashlib
import json
import unittest
from pathlib import Path

from run_playwright_official_conformance import _pinned_full_file_source


ROOT = Path(__file__).parents[1]
CORE_MANIFEST = ROOT / "tests/official/playwright-core-eval-on-selector-public-v1-manifest.json"
PYTHON_MANIFEST = (
    ROOT
    / "packages/pwright-python/tests/official/async-popup-full-chromium-v1-manifest.json"
)
CORE_UPSTREAM = Path.home() / "projects/thirdparty/playwright-v1.58.0"
PYTHON_UPSTREAM = Path.home() / "projects/thirdparty/playwright-python"
CORE_LEDGER = (
    ROOT
    / ".work/official-conformance/outcomes/"
    "playwright-core-candidate-react60-trailing-space-frozen-20261011-v1.jsonl"
)
PYTHON_LEDGER = (
    ROOT
    / ".work/official-conformance/outcomes/"
    "playwright-python-candidate-abi108-select-option-async24-sync23-v1.jsonl"
)


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _ledger_rows(path: Path) -> list[dict[str, object]]:
    return [json.loads(line) for line in path.read_text().splitlines()]


class PublicApiExpansionManifestTests(unittest.TestCase):
    def test_core_eval_on_selector_pins_full_chromium_file(self) -> None:
        # Arrange
        manifest = json.loads(CORE_MANIFEST.read_text())
        self.assertEqual(len(manifest["suites"]), 1)
        suite = manifest["suites"][0]

        # Act and Assert
        self.assertEqual(manifest["tag"], "v1.58.0")
        self.assertEqual(
            manifest["commit"], "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
        )
        self.assertEqual(suite["args"], ["tests/page/eval-on-selector.spec.ts"])
        self.assertEqual(suite["project"], "chromium-page")
        self.assertEqual(suite["role"], "browser-backed-official-file")
        self.assertEqual(suite["expectedTestsPerProject"], 26)
        self.assertEqual(suite["expectedFiles"], 1)
        self.assertTrue(suite["requiresBrowser"])
        if (CORE_UPSTREAM / suite["args"][0]).is_file():
            self.assertEqual(
                _sha256(CORE_UPSTREAM / suite["args"][0]),
                suite["upstreamTestSourceSha256"],
            )
            self.assertEqual(
                _pinned_full_file_source(suite, CORE_UPSTREAM),
                (
                    "page/eval-on-selector.spec.ts",
                    26,
                    "chromium-page",
                    suite["expectedSpecIdsSha256"],
                ),
            )
        if CORE_LEDGER.is_file():
            rows = [
                row
                for row in _ledger_rows(CORE_LEDGER)
                if row["sourceFile"] == suite["args"][0]
                and row["projectId"] == suite["project"]
            ]
            self.assertEqual(len(rows), suite["expectedTestsPerProject"])
            self.assertTrue(
                all(row["sourceClassification"] == "public-api-candidate" for row in rows)
            )
            self.assertTrue(all(row["outcome"] == "not-run" for row in rows))
            spec_ids = sorted(row["specId"] for row in rows)
            digest = hashlib.sha256(
                "".join(f"{spec_id}\n" for spec_id in spec_ids).encode()
            ).hexdigest()
            self.assertEqual(digest, suite["expectedSpecIdsSha256"])

    def test_python_popup_pins_entire_chromium_file(self) -> None:
        # Arrange
        manifest = json.loads(PYTHON_MANIFEST.read_text())
        cases = manifest["cases"]
        ids = [case["id"] for case in cases]
        source = "tests/async/test_popup.py"

        # Act and Assert
        self.assertEqual(manifest["tag"], "v1.58.0")
        self.assertEqual(
            manifest["commit"], "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
        )
        self.assertEqual(manifest["execution"], {
            "requiresBrowser": True,
            "requiresFixtureServers": True,
        })
        self.assertEqual(len(ids), 26)
        self.assertEqual(len(ids), len(set(ids)))
        self.assertTrue(all(id.startswith(f"{source}::test_") for id in ids))
        self.assertTrue(all(case["capabilities"] for case in cases))
        if (PYTHON_UPSTREAM / source).is_file():
            for relative, expected in manifest["files"].items():
                self.assertEqual(_sha256(PYTHON_UPSTREAM / relative), expected)
            tree = ast.parse((PYTHON_UPSTREAM / source).read_bytes())
            functions = {
                f"{source}::{node.name}"
                for node in tree.body
                if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
                and node.name.startswith("test_")
            }
            self.assertEqual(set(ids), functions)
        if PYTHON_LEDGER.is_file():
            rows = [
                row
                for row in _ledger_rows(PYTHON_LEDGER)
                if row["file"] == source and str(row["id"]).endswith("[chromium]")
            ]
            self.assertEqual(len(rows), 26)
            self.assertEqual(set(ids), {str(row["id"]).removesuffix("[chromium]") for row in rows})
            self.assertEqual(
                {row["outcome"] for row in rows},
                {"passed", "not-run"},
            )
            self.assertEqual(sum(row["outcome"] == "not-run" for row in rows), 25)


if __name__ == "__main__":
    unittest.main()
