"""Source-only pin for an Oracle-only, two-case trusted-HTTPS diagnostic."""

from __future__ import annotations

import ast
import hashlib
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
UPSTREAM = Path.home() / "projects/thirdparty/playwright-python"
MANIFEST = (
    ROOT
    / "packages/pwright-python/tests/official/"
    "sync-network-service-worker-secure-chromium-oracle-v1-manifest.json"
)
ORIGINAL_SIX_CASE_MANIFEST = (
    ROOT / "packages/pwright-python/tests/official/sync-network-full-v1-manifest.json"
)
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
SOURCE = "tests/sync/test_network.py"
NAMES = (
    "test_should_report_if_response_was_from_service_worker",
    "test_should_report_service_worker_request",
)
EXPECTED_IDS = [f"{SOURCE}::{name}[chromium]" for name in NAMES]
EXPECTED_EXPERIMENT = {
    "scope": "oracle-only-trusted-tailscale-https-fixture-diagnostic",
    "api": "sync",
    "browser": "chromium",
    "fixtureFlag": "--secure-http-fixture",
    "selectionFlags": ["--manifest-cases", "--manifest-api", "sync", "--browser", "chromium"],
    "ledgerPolicy": "diagnostic-only-preserve-original-six-case-results",
    "hypothesis": (
        "Trusted HTTPS may enable service-worker registration and activation; "
        "this manifest does not establish that cause."
    ),
}
EXPECTED_SOURCE_FILES = {
    "tests/conftest.py",
    "tests/sync/conftest.py",
    "tests/server.py",
    "tests/utils.py",
    SOURCE,
}
EXPECTED_ASSETS = {
    "tests/assets/empty.html",
    "tests/assets/simple.json",
    "tests/assets/serviceworkers/fetch/sw.html",
    "tests/assets/serviceworkers/fetch/sw.js",
    "tests/assets/serviceworkers/fetch/style.css",
    "tests/testserver/cert.pem",
    "tests/testserver/key.pem",
}


def _manifest(path: Path) -> dict[str, object]:
    return json.loads(path.read_text())


def _hash_ids(ids: list[str]) -> str:
    return hashlib.sha256("".join(f"{item}\n" for item in sorted(ids)).encode()).hexdigest()


class SyncNetworkServiceWorkerSecureManifestTests(unittest.TestCase):
    def test_exact_oracle_only_scope_is_a_subset_of_original_six_cases(self) -> None:
        # Arrange
        manifest = _manifest(MANIFEST)
        original = _manifest(ORIGINAL_SIX_CASE_MANIFEST)
        cases = manifest["cases"]

        # Act
        ids = [case["id"] for case in cases]

        # Assert
        self.assertEqual(manifest["schema"], 1)
        self.assertEqual(manifest["repository"], original["repository"])
        self.assertEqual(manifest["tag"], "v1.58.0")
        self.assertEqual(manifest["commit"], "47a5d35ef4f815a2021349f86ae391f7c20c02d6")
        self.assertEqual(manifest["corpus"], original["corpus"])
        self.assertEqual(manifest["execution"], {"requiresBrowser": True, "requiresFixtureServers": True})
        self.assertEqual(manifest["experiment"], EXPECTED_EXPERIMENT)
        self.assertEqual(ids, [f"{SOURCE}::{name}" for name in NAMES])
        self.assertEqual(cases, original["cases"][-2:])
        self.assertEqual(set(manifest["files"]), EXPECTED_SOURCE_FILES)
        self.assertEqual(set(manifest["assets"]), EXPECTED_ASSETS)
        self.assertEqual(manifest["files"], original["files"])
        self.assertEqual(manifest["assets"], original["assets"])
        self.assertEqual(manifest["expandedChromiumIds"], EXPECTED_IDS)
        self.assertEqual(manifest["expandedChromiumIdsSha256"], _hash_ids(EXPECTED_IDS))

    def test_selected_source_fixture_and_assets_match_pinned_upstream(self) -> None:
        if not UPSTREAM.is_dir():
            self.skipTest("pinned upstream checkout is unavailable")

        # Arrange
        manifest = _manifest(MANIFEST)
        source = (UPSTREAM / SOURCE).read_bytes()
        tree = ast.parse(source, filename=SOURCE)
        functions = {
            node.name: node
            for node in tree.body
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
            and node.name.startswith("test_")
        }

        # Act
        observed_hashes = {
            path: hashlib.sha256((UPSTREAM / path).read_bytes()).hexdigest()
            for path in {**manifest["files"], **manifest["assets"]}
        }

        # Assert
        self.assertEqual(set(NAMES), set(functions).intersection(NAMES))
        self.assertTrue(all(isinstance(functions[name], ast.FunctionDef) for name in NAMES))
        self.assertEqual(observed_hashes, {**manifest["files"], **manifest["assets"]})

    def test_inventory_expands_exactly_two_public_chromium_ids(self) -> None:
        if not INVENTORY.is_file():
            self.skipTest("pinned upstream inventory is unavailable")

        # Arrange
        manifest = _manifest(MANIFEST)
        base_ids = {case["id"] for case in manifest["cases"]}

        # Act
        rows = []
        with INVENTORY.open() as stream:
            for line in stream:
                row = json.loads(line)
                if row["id"].split("[", 1)[0] in base_ids:
                    rows.append(row)
        chromium_rows = [row for row in rows if row["id"].endswith("[chromium]")]
        expanded_ids = [row["id"] for row in chromium_rows]

        # Assert
        self.assertEqual(len(rows), 6)
        self.assertEqual({row["id"] for row in rows}, {
            f"{base}[{browser}]"
            for base in base_ids
            for browser in ("chromium", "firefox", "webkit")
        })
        self.assertEqual(expanded_ids, EXPECTED_IDS)
        self.assertEqual(_hash_ids(expanded_ids), manifest["expandedChromiumIdsSha256"])
        self.assertTrue(all(row["api"] == "sync" for row in rows))
        self.assertTrue(all(row["sourceClassification"] == "public-api-source" for row in rows))
        self.assertTrue(all(row["privateImports"] == [] for row in rows))


if __name__ == "__main__":
    unittest.main()
