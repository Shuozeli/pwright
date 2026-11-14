"""Check the pinned full-file Chromium-page page.$$eval selection."""

from __future__ import annotations

import hashlib
import json
import re
import unittest
from pathlib import Path

from run_playwright_official_conformance import _pinned_full_file_source

ROOT = Path(__file__).parents[1]
MANIFEST = (
    ROOT / "tests/official/playwright-core-eval-on-selector-all-public-v1-manifest.json"
)
UPSTREAM = Path.home() / "projects/thirdparty/playwright-v1.58.0"
LEDGER = (
    ROOT / ".work/official-conformance/outcomes/"
    "playwright-core-candidate-react60-trailing-space-frozen-20261011-v1.jsonl"
)


class EvalOnSelectorAllManifestTests(unittest.TestCase):
    def test_manifest_pins_the_entire_public_chromium_file(self) -> None:
        # Arrange
        manifest = json.loads(MANIFEST.read_text())
        self.assertEqual(len(manifest["suites"]), 1)
        suite = manifest["suites"][0]

        # Act
        selection = (
            _pinned_full_file_source(suite, UPSTREAM)
            if (UPSTREAM / "tests/page/eval-on-selector-all.spec.ts").is_file()
            else None
        )

        # Assert
        self.assertEqual(manifest["tag"], "v1.58.0")
        self.assertEqual(manifest["commit"], "961381ec73a9fa2661d92504eab1a6bd6eaffc75")
        self.assertEqual(suite["id"], "library-eval-on-selector-all-public-chromium")
        self.assertEqual(suite["args"], ["tests/page/eval-on-selector-all.spec.ts"])
        self.assertEqual(suite["project"], "chromium-page")
        self.assertEqual(suite["expectedTestsPerProject"], 9)
        self.assertEqual(suite["expectedFiles"], 1)
        self.assertEqual(
            suite["upstreamTestSourceSha256"],
            "a8e52ce058ff4f0a0b48e72f1fb5c3c78c8fd4db4375ef6d1e01e9caa44ecc91",
        )
        self.assertEqual(
            suite["expectedSpecIdsSha256"],
            "5e3010b5346745722102bc3bf62cdf842ff5543baec09063cbcab6041322a199",
        )
        if selection is not None:
            self.assertEqual(
                selection,
                (
                    "page/eval-on-selector-all.spec.ts",
                    9,
                    "chromium-page",
                    suite["expectedSpecIdsSha256"],
                ),
            )
            source = (UPSTREAM / suite["args"][0]).read_text()
            self.assertEqual(len(re.findall(r"^it\('", source, flags=re.MULTILINE)), 9)
            self.assertEqual(source.count("page.$$eval("), 16)

    def test_frozen_inventory_contains_exact_nine_unrun_public_rows(self) -> None:
        if not LEDGER.is_file():
            self.skipTest("frozen Core inventory is unavailable")

        # Arrange
        manifest = json.loads(MANIFEST.read_text())
        suite = manifest["suites"][0]
        with LEDGER.open() as stream:
            rows = [
                json.loads(line)
                for line in stream
                if '"sourceFile":"tests/page/eval-on-selector-all.spec.ts"' in line
                and '"projectId":"chromium-page"' in line
            ]

        # Act
        spec_ids = sorted(row["specId"] for row in rows)
        digest = hashlib.sha256(
            "".join(f"{spec_id}\n" for spec_id in spec_ids).encode()
        ).hexdigest()

        # Assert
        self.assertEqual(len(rows), 9)
        self.assertEqual(len(spec_ids), len(set(spec_ids)))
        self.assertTrue(
            all(row["sourceClassification"] == "public-api-candidate" for row in rows)
        )
        self.assertTrue(all(row["outcome"] == "not-run" for row in rows))
        self.assertEqual(digest, suite["expectedSpecIdsSha256"])


if __name__ == "__main__":
    unittest.main()
