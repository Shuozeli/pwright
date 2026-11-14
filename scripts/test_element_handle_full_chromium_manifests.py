"""Check complete pinned Chromium ElementHandle async and sync test files."""

from __future__ import annotations

import ast
import hashlib
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[1]
UPSTREAM = Path.home() / "projects/thirdparty/playwright-python"
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
PINNED_COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
SOURCES = {
    "async": ("tests/async/test_element_handle.py", 59),
    "sync": ("tests/sync/test_element_handle.py", 53),
}
COMMON_FILES = {
    "tests/conftest.py": "c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc",
    "tests/server.py": "2cf1a6b805f99c1c82c5351b67df995f06f2a9a18d21dbd48d4d4dc6079c3030",
    "tests/utils.py": "60b7d0915ffdaec4dda729fc38658d55bb216ec8baaf9087d77cef4c3c34abea",
}
API_FILES = {
    "async": {
        "tests/async/conftest.py": "3001008bb97ba0cc1ddb397a53df530b6881e3e3ba69a1ed2cac0de94cab4cdb",
        "tests/async/utils.py": "ee618a20d789a28f953ba23a2c24249f3b21f7bd00b1a2d2435aa75ce9b0b478",
        "tests/async/test_element_handle.py": "009e0cf02d942b5a898b2abc395797ebe8804edbe5f95def791a9e15a07e6e72",
    },
    "sync": {
        "tests/sync/conftest.py": "9ec5a65a98c312de67590792e80d3d9ea45312d56188f45101e71df6f7773d0b",
        "tests/sync/utils.py": "ae98bc4ec9132e71456246209de88e4300c9624bf355d763d1881c2919e29bc4",
        "tests/sync/test_element_handle.py": "be252d4988a72bb4a4b5d6295429cd96795931135bdec7512b892840294820c2",
    },
}


def _manifest(api: str, *, preview: bool = False) -> dict[str, object]:
    suffix = (
        "evaluate-handle-preview-chromium-v1-manifest.json"
        if preview
        else "full-chromium-v1-manifest.json"
    )
    path = (
        ROOT
        / "packages/pwright-python/tests/official"
        / f"{api}-element-handle-{suffix}"
    )
    return json.loads(path.read_text())


def _chromium_rows(source: str) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    with INVENTORY.open() as stream:
        for line in stream:
            if f'"file":"{source}"' in line:
                row = json.loads(line)
                if str(row["id"]).endswith("[chromium]"):
                    rows.append(row)
    return rows


class ElementHandleFullChromiumManifestTests(unittest.TestCase):
    def test_manifests_cover_every_pinned_source_function_and_fixture(self) -> None:
        if not UPSTREAM.is_dir():
            self.skipTest("pinned upstream checkout is unavailable")

        for api, (source, count) in SOURCES.items():
            with self.subTest(api=api):
                # Arrange
                manifest = _manifest(api)
                tree = ast.parse((UPSTREAM / source).read_bytes())
                source_ids = {
                    f"{source}::{node.name}"
                    for node in tree.body
                    if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
                    and node.name.startswith("test_")
                }
                expected_files = COMMON_FILES | API_FILES[api]

                # Act
                cases = manifest["cases"]
                selected = [case["id"] for case in cases]

                # Assert
                self.assertEqual(manifest["schema"], 1)
                self.assertEqual(
                    manifest["repository"],
                    "https://github.com/microsoft/playwright-python.git",
                )
                self.assertEqual(manifest["tag"], "v1.58.0")
                self.assertEqual(manifest["commit"], PINNED_COMMIT)
                self.assertEqual(manifest["corpus"]["expandedTests"], 6526)
                self.assertFalse(manifest["bootstrapSelectionOnly"])
                self.assertEqual(
                    manifest["execution"],
                    {"requiresBrowser": True, "requiresFixtureServers": True},
                )
                self.assertEqual(len(source_ids), count)
                self.assertEqual(len(cases), count)
                self.assertEqual(len(set(selected)), count)
                self.assertEqual(set(selected), source_ids)
                self.assertTrue(
                    all(
                        case["capabilities"] == ["element-handle", f"{api}-api"]
                        for case in cases
                    )
                )
                self.assertEqual(manifest["files"], expected_files)
                for relative, expected_hash in expected_files.items():
                    self.assertEqual(
                        hashlib.sha256((UPSTREAM / relative).read_bytes()).hexdigest(),
                        expected_hash,
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
                rows = _chromium_rows(source)

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

    def test_existing_single_case_previews_remain_selected(self) -> None:
        for api, (source, _) in SOURCES.items():
            with self.subTest(api=api):
                # Arrange
                full = _manifest(api)
                preview = _manifest(api, preview=True)
                preview_id = f"{source}::test_a_nice_preview"

                # Act
                selected = {case["id"] for case in full["cases"]}

                # Assert
                self.assertEqual(
                    [case["id"] for case in preview["cases"]], [preview_id]
                )
                self.assertIn(preview_id, selected)
                self.assertEqual(preview["files"], full["files"])


if __name__ == "__main__":
    unittest.main()
