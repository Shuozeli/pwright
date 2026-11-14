"""Pin every upstream sync-network test, fixture, asset, and unrun v9 ID."""

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
    "sync-network-full-v1-manifest.json"
)
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
SOURCE = "tests/sync/test_network.py"
BROWSERS = ("chromium", "firefox", "webkit")
LEDGERS = {
    "candidate": (
        ROOT
        / ".work/official-conformance/outcomes/"
        "python-navigation-recheck-chromium-strict-paired-v9/"
        "playwright-python-candidate-navigation-recheck-chromium-strict-paired-v9.jsonl",
        "d7e2dcebe40d0646f6dc0ce215b584cc020743d7ce1def3fac0bb0a790e94756",
    ),
    "oracle": (
        ROOT
        / ".work/official-conformance/outcomes/"
        "python-navigation-recheck-chromium-strict-paired-v9/"
        "playwright-python-oracle-navigation-recheck-chromium-strict-paired-v9.jsonl",
        "5993775f11480d3e24ed6a56a4d9a7ccd4b3c4dd3722141ac4c27c3a4132907a",
    ),
}
SOURCE_FILES = {
    "tests/conftest.py",
    "tests/sync/conftest.py",
    "tests/server.py",
    "tests/utils.py",
    SOURCE,
}
ASSET_FILES = {
    "tests/assets/empty.html",
    "tests/assets/simple.json",
    "tests/assets/serviceworkers/fetch/sw.html",
    "tests/assets/serviceworkers/fetch/sw.js",
    "tests/assets/serviceworkers/fetch/style.css",
    "tests/testserver/cert.pem",
    "tests/testserver/key.pem",
}


def _manifest() -> dict[str, object]:
    return json.loads(MANIFEST.read_text())


def _rows(path: Path) -> list[dict[str, object]]:
    with path.open() as stream:
        return [
            row
            for line in stream
            if (row := json.loads(line)).get("file") == SOURCE
        ]


def _expected_ids(manifest: dict[str, object]) -> set[str]:
    cases = manifest["cases"]
    assert isinstance(cases, list)
    return {
        f"{case['id']}[{browser}]"
        for case in cases
        for browser in BROWSERS
    }


class SyncNetworkFullManifestTests(unittest.TestCase):
    def test_complete_source_and_fixture_assets_are_pinned(self) -> None:
        if not UPSTREAM.is_dir():
            self.skipTest("pinned upstream checkout is unavailable")

        # Arrange
        manifest = _manifest()
        source = (UPSTREAM / SOURCE).read_bytes()
        tree = ast.parse(source, filename=SOURCE)
        source_ids = {
            f"{SOURCE}::{node.name}"
            for node in tree.body
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
            and node.name.startswith("test_")
        }
        cases = manifest["cases"]
        files = manifest["files"]
        assets = manifest["assets"]

        # Act
        selected = {case["id"] for case in cases}

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
        self.assertEqual(len(cases), 6)
        self.assertEqual(len(selected), 6)
        self.assertEqual(selected, source_ids)
        self.assertEqual(set(files), SOURCE_FILES)
        self.assertEqual(set(assets), ASSET_FILES)
        self.assertTrue(all("sync-api" in case["capabilities"] for case in cases))
        for relative, expected_hash in {**files, **assets}.items():
            with self.subTest(relative=relative):
                self.assertEqual(
                    hashlib.sha256((UPSTREAM / relative).read_bytes()).hexdigest(),
                    expected_hash,
                )

    def test_inventory_has_exactly_all_eighteen_public_ids(self) -> None:
        if not INVENTORY.is_file():
            self.skipTest("pinned Python inventory is unavailable")

        # Arrange
        manifest = _manifest()
        expected = _expected_ids(manifest)

        # Act
        rows = _rows(INVENTORY)
        observed = {row["id"] for row in rows}
        digest = hashlib.sha256(
            "".join(f"{test_id}\n" for test_id in sorted(observed)).encode()
        ).hexdigest()

        # Assert
        self.assertEqual(len(rows), 18)
        self.assertEqual(observed, expected)
        self.assertEqual(digest, manifest["expandedFileIdsSha256"])
        self.assertTrue(all(row["api"] == "sync" for row in rows))
        self.assertTrue(
            all(row["sourceClassification"] == "public-api-source" for row in rows)
        )
        self.assertTrue(all(row["privateImports"] == [] for row in rows))

    def test_both_strict_v9_ledgers_leave_all_eighteen_ids_unrun(self) -> None:
        if not all(path.is_file() for path, _ in LEDGERS.values()):
            self.skipTest("frozen strict v9 ledgers are unavailable")

        # Arrange
        expected = _expected_ids(_manifest())

        for lane, (path, expected_hash) in LEDGERS.items():
            with self.subTest(lane=lane):
                # Act
                rows = _rows(path)
                observed_hash = hashlib.sha256(path.read_bytes()).hexdigest()

                # Assert
                self.assertEqual(observed_hash, expected_hash)
                self.assertEqual(len(rows), 18)
                self.assertEqual({row["id"] for row in rows}, expected)
                self.assertTrue(all(row["outcome"] == "not-run" for row in rows))
                self.assertTrue(all(row["attempts"] == 0 for row in rows))


if __name__ == "__main__":
    unittest.main()
