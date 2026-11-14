"""Check the complete source-pinned Chromium-page waitForSelector file 1."""

from __future__ import annotations

import hashlib
import json
import re
import unittest
from pathlib import Path

from run_playwright_official_conformance import _pinned_full_file_source

ROOT = Path(__file__).parents[1]
SOURCE_FILE = "tests/page/page-wait-for-selector-1.spec.ts"
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-page-wait-for-selector-1-public-v1-manifest.json"
)
UPSTREAM = Path.home() / "projects/thirdparty/playwright-v1.58.0"
LEDGER = (
    ROOT / ".work/official-conformance/outcomes/"
    "playwright-core-candidate-react60-trailing-space-frozen-20261011-v1.jsonl"
)
INVOCATION_AUDIT = (
    ROOT / ".work/official-conformance/"
    "playwright-core-v1.58.0-invocation-classifications-core445-oracle.jsonl"
)


def _matching_rows(path: Path) -> list[dict[str, object]]:
    with path.open() as stream:
        return [
            json.loads(line)
            for line in stream
            if f'"sourceFile":"{SOURCE_FILE}"' in line
            and '"projectId":"chromium-page"' in line
        ]


class PageWaitForSelectorOneManifestTests(unittest.TestCase):
    def test_manifest_pins_complete_chromium_page_file(self) -> None:
        # Arrange
        manifest = json.loads(MANIFEST.read_text())
        self.assertEqual(len(manifest["suites"]), 1)
        suite = manifest["suites"][0]
        source_path = UPSTREAM / SOURCE_FILE

        # Act
        source_selection = (
            _pinned_full_file_source(suite, UPSTREAM) if source_path.is_file() else None
        )

        # Assert
        self.assertEqual(manifest["schema"], 1)
        self.assertEqual(manifest["tag"], "v1.58.0")
        self.assertEqual(manifest["commit"], "961381ec73a9fa2661d92504eab1a6bd6eaffc75")
        self.assertEqual(
            suite["id"], "library-page-wait-for-selector-1-public-chromium"
        )
        self.assertEqual(suite["args"], [SOURCE_FILE])
        self.assertEqual(suite["project"], "chromium-page")
        self.assertEqual(suite["expectedTestsPerProject"], 17)
        self.assertEqual(suite["expectedFiles"], 1)
        self.assertEqual(
            suite["upstreamTestSourceSha256"],
            "f449d8619753aac3224a34e800bb9122e81755f10a1be48eaecf8f9df496394e",
        )
        self.assertEqual(
            suite["expectedSpecIdsSha256"],
            "1fbb409ad9501b7dcc92bef4701f45dbf9d4c0de9592ffdb82bc266e2e786f25",
        )
        if source_selection is not None:
            self.assertEqual(
                source_selection,
                (
                    "page/page-wait-for-selector-1.spec.ts",
                    17,
                    "chromium-page",
                    suite["expectedSpecIdsSha256"],
                ),
            )
            source = source_path.read_text()
            self.assertEqual(len(re.findall(r"^it\('", source, flags=re.MULTILINE)), 17)
            self.assertNotIn("it.skip(", source)
            self.assertNotIn("it.fixme(", source)

    def test_frozen_inventory_has_exact_public_unrun_ids_and_no_skips(self) -> None:
        if not LEDGER.is_file():
            self.skipTest("frozen Core inventory is unavailable")

        # Arrange
        suite = json.loads(MANIFEST.read_text())["suites"][0]
        rows = _matching_rows(LEDGER)

        # Act
        spec_ids = sorted(str(row["specId"]) for row in rows)
        digest = hashlib.sha256(
            "".join(f"{spec_id}\n" for spec_id in spec_ids).encode()
        ).hexdigest()

        # Assert
        self.assertEqual(len(rows), 17)
        self.assertEqual(len(spec_ids), len(set(spec_ids)))
        self.assertTrue(
            all(row["sourceClassification"] == "public-api-candidate" for row in rows)
        )
        self.assertTrue(all(row["expectedStatus"] == "passed" for row in rows))
        self.assertTrue(all(row["outcome"] == "not-run" for row in rows))
        self.assertEqual(digest, suite["expectedSpecIdsSha256"])

    def test_pinned_invocation_audit_has_no_internal_exclusions(self) -> None:
        if not INVOCATION_AUDIT.is_file():
            self.skipTest("frozen invocation audit is unavailable")

        # Arrange
        rows = _matching_rows(INVOCATION_AUDIT)
        suite = json.loads(MANIFEST.read_text())["suites"][0]

        # Act
        classifications = {row["invocationClassification"] for row in rows}
        spec_ids = sorted(str(row["specId"]) for row in rows)
        digest = hashlib.sha256(
            "".join(f"{spec_id}\n" for spec_id in spec_ids).encode()
        ).hexdigest()

        # Assert
        self.assertEqual(len(rows), 17)
        self.assertEqual(classifications, {"public-api-candidate"})
        self.assertTrue(
            all(row["invocationClassificationReasons"] == [] for row in rows)
        )
        self.assertEqual(digest, suite["expectedSpecIdsSha256"])


if __name__ == "__main__":
    unittest.main()
