"""Verify the pinned complete Chromium-library CDPSession selection."""

from __future__ import annotations

import hashlib
import json
import re
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
MANIFEST = ROOT / "tests/official/playwright-core-cdp-session-full-chromium-v1-manifest.json"
UPSTREAM = Path.home() / "projects/thirdparty/playwright-v1.58.0"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
SOURCE = "tests/library/chromium/session.spec.ts"
PROJECT = "chromium-library"


class CoreCdpSessionFullChromiumManifestTests(unittest.TestCase):
    def test_manifest_pins_complete_upstream_file(self) -> None:
        # Arrange
        manifest = json.loads(MANIFEST.read_text())
        suite = manifest["suites"][0]

        # Act
        source = UPSTREAM / SOURCE

        # Assert
        self.assertEqual(manifest["schema"], 1)
        self.assertEqual(manifest["tag"], "v1.58.0")
        self.assertEqual(
            manifest["commit"], "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
        )
        self.assertEqual(len(manifest["suites"]), 1)
        self.assertEqual(suite["id"], "library-chromium-cdp-session-full")
        self.assertEqual(suite["args"], [SOURCE])
        self.assertEqual(suite["project"], PROJECT)
        self.assertEqual(suite["candidateAdapter"], "test-mode-playwright-core")
        self.assertEqual(suite["oracleAdapter"], "test-mode-playwright-core-cdp")
        self.assertEqual(suite["expectedTestsPerProject"], 12)
        self.assertEqual(suite["expectedFiles"], 1)
        self.assertIs(suite["requiresBrowser"], True)
        if not source.is_file():
            self.skipTest("pinned Core checkout is unavailable")
        encoded = source.read_bytes()
        self.assertEqual(
            hashlib.sha256(encoded).hexdigest(), suite["upstreamTestSourceSha256"]
        )
        self.assertEqual(len(re.findall(rb"(?m)^(?:it|browserTest)\(", encoded)), 12)

    def test_inventory_matches_complete_pinned_selection(self) -> None:
        if not INVENTORY.is_file():
            self.skipTest("pinned Core inventory is unavailable")

        # Arrange
        suite = json.loads(MANIFEST.read_text())["suites"][0]

        # Act
        ids: list[str] = []
        with INVENTORY.open() as stream:
            for line in stream:
                if f'"sourceFile":"{SOURCE}"' not in line:
                    continue
                row = json.loads(line)
                if row["projectId"] == PROJECT:
                    self.assertEqual(row["sourceClassification"], "public-api-candidate")
                    ids.append(row["specId"])
        digest = hashlib.sha256(
            "".join(f"{spec_id}\n" for spec_id in sorted(ids)).encode()
        ).hexdigest()

        # Assert
        self.assertEqual(len(ids), 12)
        self.assertEqual(len(set(ids)), 12)
        self.assertEqual(digest, suite["expectedSpecIdsSha256"])


if __name__ == "__main__":
    unittest.main()
