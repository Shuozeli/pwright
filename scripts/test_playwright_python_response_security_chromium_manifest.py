"""Source-only pin for four unchanged Python response-security Chromium IDs."""

from __future__ import annotations

import ast
import hashlib
import json
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[1]
UPSTREAM = Path.home() / "projects/thirdparty/playwright-python"
OFFICIAL = ROOT / "packages/pwright-python/tests/official"
MANIFEST = OFFICIAL / "response-security-chromium-sync-async-v1-manifest.json"
SYNC_SOURCE = OFFICIAL / "sync-network-full-v1-manifest.json"
ASYNC_SOURCE = OFFICIAL / "async-network-full-chromium-v1-manifest.json"
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
NAMES = (
    "test_response_security_details",
    "test_response_security_details_none_without_https",
)
BASE_IDS = [
    f"tests/{api}/test_network.py::{name}"
    for api in ("sync", "async")
    for name in NAMES
]
CHROMIUM_IDS = [f"{base}[chromium]" for base in BASE_IDS]
ASSETS = {
    "tests/assets/empty.html": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "tests/testserver/cert.pem": "402a4ecc40aba35f8143673ac7c20d02c742954a79dc94523b06a5bd9c5fb18e",
    "tests/testserver/key.pem": "3302e5f7eb8485e832a10f54a63cc049f98e98a5a2d55671c9ed9b440c59053a",
}


def _read_json(path: Path) -> dict:
    return json.loads(path.read_text())


def _hash_ids(ids: list[str]) -> str:
    return hashlib.sha256(
        "".join(f"{item}\n" for item in sorted(ids)).encode()
    ).hexdigest()


class ResponseSecurityChromiumManifestTests(unittest.TestCase):
    def test_scope_is_exactly_four_unchanged_public_cases(self) -> None:
        # Arrange
        manifest = _read_json(MANIFEST)
        sync = _read_json(SYNC_SOURCE)
        async_ = _read_json(ASYNC_SOURCE)
        expected_cases = [
            {"id": base, "capabilities": ["response-security", f"{api}-api"]}
            for api in ("sync", "async")
            for base in BASE_IDS
            if base.startswith(f"tests/{api}/")
        ]

        # Act
        selected = [
            case for case in sync["cases"] + async_["cases"] if case["id"] in BASE_IDS
        ]

        # Assert
        self.assertEqual(manifest["schema"], 1)
        self.assertEqual(manifest["repository"], sync["repository"])
        self.assertEqual(manifest["tag"], "v1.58.0")
        self.assertEqual(manifest["commit"], COMMIT)
        self.assertEqual(manifest["corpus"], sync["corpus"])
        self.assertEqual(manifest["corpus"], async_["corpus"])
        self.assertFalse(manifest["bootstrapSelectionOnly"])
        self.assertEqual(
            manifest["execution"],
            {"requiresBrowser": True, "requiresFixtureServers": True},
        )
        self.assertEqual(
            manifest["experiment"],
            {
                "scope": "paired-four-case-response-security-diagnostic",
                "lanes": ["candidate", "oracle"],
                "apis": ["sync", "async"],
                "browser": "chromium",
                "fixturePolicy": "unchanged-upstream-http-and-https-server-fixtures",
                "ledgerPolicy": "focused-diagnostic-no-promotion-until-complete-source-files-rerun",
                "candidatePrerequisites": {
                    "allFourCases": [
                        "normal-page-navigation-response",
                        "response-security-details",
                    ],
                    "httpsCasesOnly": [
                        "ignore-https-errors",
                        "response-finished-on-navigation-response",
                    ],
                    "note": "The inherited cases.capabilities labels classify upstream test topics, not a complete candidate feature closure. In the current candidate, Page.goto must yield a Response; navigation-response-finished supplies Response.finished() on that navigation Response and selects response-finished.",
                },
            },
        )
        self.assertEqual(manifest["cases"], expected_cases)
        self.assertEqual(selected, expected_cases)
        self.assertEqual(manifest["files"], {**sync["files"], **async_["files"]})
        self.assertEqual(manifest["assets"], ASSETS)
        self.assertEqual(manifest["expandedChromiumIds"], CHROMIUM_IDS)
        self.assertEqual(manifest["expandedChromiumIdsSha256"], _hash_ids(CHROMIUM_IDS))

    def test_pinned_source_and_assets_match_upstream_bytes(self) -> None:
        if not UPSTREAM.is_dir():
            self.skipTest("pinned upstream checkout is unavailable")

        # Arrange
        manifest = _read_json(MANIFEST)
        expected_paths = {**manifest["files"], **manifest["assets"]}
        revision = subprocess.run(
            ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()

        # Act
        actual_hashes = {
            path: hashlib.sha256((UPSTREAM / path).read_bytes()).hexdigest()
            for path in expected_paths
        }
        source_functions = {}
        for api in ("sync", "async"):
            source = f"tests/{api}/test_network.py"
            tree = ast.parse((UPSTREAM / source).read_bytes(), filename=source)
            source_functions[api] = {
                node.name: node
                for node in tree.body
                if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
            }

        # Assert
        self.assertEqual(revision, COMMIT)
        self.assertEqual(actual_hashes, expected_paths)
        for api in ("sync", "async"):
            self.assertTrue(all(name in source_functions[api] for name in NAMES))
            expected_type = ast.FunctionDef if api == "sync" else ast.AsyncFunctionDef
            self.assertTrue(
                all(
                    isinstance(source_functions[api][name], expected_type)
                    for name in NAMES
                )
            )

    def test_inventory_contains_all_twelve_engine_expansions(self) -> None:
        if not INVENTORY.is_file():
            self.skipTest("pinned upstream inventory is unavailable")

        # Arrange
        manifest = _read_json(MANIFEST)
        selected = set(BASE_IDS)

        # Act
        rows = []
        with INVENTORY.open() as stream:
            for line in stream:
                row = json.loads(line)
                if row["id"].split("[", 1)[0] in selected:
                    rows.append(row)
        chromium = [row["id"] for row in rows if row["id"].endswith("[chromium]")]

        # Assert
        self.assertEqual(len(rows), 12)
        self.assertEqual(
            {row["id"] for row in rows},
            {
                f"{base}[{browser}]"
                for base in BASE_IDS
                for browser in ("chromium", "firefox", "webkit")
            },
        )
        self.assertEqual(set(chromium), set(CHROMIUM_IDS))
        self.assertEqual(_hash_ids(chromium), manifest["expandedChromiumIdsSha256"])
        self.assertTrue(
            all(row["sourceClassification"] == "public-api-source" for row in rows)
        )
        self.assertTrue(all(row["privateImports"] == [] for row in rows))


if __name__ == "__main__":
    unittest.main()
