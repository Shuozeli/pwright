"""Check the focused official ElementHandle.evaluate_handle preview selections."""

from __future__ import annotations

import hashlib
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[1]
UPSTREAM = Path.home() / "projects/thirdparty/playwright-python"
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
OUTCOMES = (
    ROOT / ".work/official-conformance/outcomes/"
    "playwright-python-candidate-route-deferred-intercept14-20261011-v10.jsonl",
    ROOT / ".work/official-conformance/outcomes/"
    "playwright-python-oracle-intercept14-tap10-cdp11-20261011-v7.jsonl",
)


def _manifest(api: str) -> dict[str, object]:
    path = (
        ROOT / "packages/pwright-python/tests/official/"
        f"{api}-element-handle-evaluate-handle-preview-chromium-v1-manifest.json"
    )
    return json.loads(path.read_text())


def _row(path: Path, case_id: str) -> dict[str, object]:
    with path.open() as stream:
        matches = [json.loads(line) for line in stream if f'"id":"{case_id}"' in line]
    if len(matches) != 1:
        raise AssertionError(
            f"expected one inventory row for {case_id}: {len(matches)}"
        )
    return matches[0]


class ElementHandleEvaluateHandlePreviewManifestTests(unittest.TestCase):
    def test_selection_pins_exact_upstream_sources_and_public_chromium_rows(
        self,
    ) -> None:
        if not UPSTREAM.is_dir() or not INVENTORY.is_file():
            self.skipTest("pinned upstream checkout or inventory unavailable")

        for api in ("async", "sync"):
            with self.subTest(api=api):
                # Arrange
                manifest = _manifest(api)
                source = f"tests/{api}/test_element_handle.py"
                case_id = f"{source}::test_a_nice_preview"

                # Act
                selected = manifest["cases"]
                row = _row(INVENTORY, f"{case_id}[chromium]")

                # Assert
                self.assertEqual(
                    manifest["commit"], "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
                )
                self.assertEqual(manifest["bootstrapSelectionOnly"], False)
                self.assertEqual(
                    manifest["execution"],
                    {"requiresBrowser": True, "requiresFixtureServers": True},
                )
                self.assertEqual(
                    selected,
                    [
                        {
                            "id": case_id,
                            "capabilities": [
                                "handles",
                                "element-handle.evaluate-handle",
                                f"{api}-api",
                            ],
                        }
                    ],
                )
                self.assertEqual(row["api"], api)
                self.assertEqual(row["privateImports"], [])
                self.assertEqual(row["sourceClassification"], "public-api-source")
                for relative, expected in manifest["files"].items():
                    self.assertEqual(
                        hashlib.sha256((UPSTREAM / relative).read_bytes()).hexdigest(),
                        expected,
                    )

    def test_both_strict_ledgers_leave_the_selected_rows_unrun(self) -> None:
        if not all(path.is_file() for path in OUTCOMES):
            self.skipTest("strict ledgers unavailable")

        for api in ("async", "sync"):
            case_id = (
                f"tests/{api}/test_element_handle.py::test_a_nice_preview[chromium]"
            )
            for ledger in OUTCOMES:
                with self.subTest(api=api, ledger=ledger.name):
                    row = _row(ledger, case_id)
                    self.assertEqual(row["outcome"], "not-run")
                    self.assertEqual(row["attempts"], 0)


if __name__ == "__main__":
    unittest.main()
