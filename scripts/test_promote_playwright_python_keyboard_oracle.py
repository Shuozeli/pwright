#!/usr/bin/env python3
"""Hermetic source and ledger checks for the pinned keyboard oracle tranche."""

from __future__ import annotations

import hashlib
import json
import unittest
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

import promote_playwright_python_keyboard_oracle as keyboard
import promote_playwright_python_verified_subset as common
import summarize_playwright_python_official_results as summarizer


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _report(ids: list[str], statuses: dict[str, str]) -> bytes:
    suite = ET.Element(
        "testsuite",
        {
            "tests": str(len(ids)),
            "failures": "0",
            "errors": "0",
            "skipped": str(sum(statuses[node_id] == "skipped" for node_id in ids)),
            "time": str(len(ids)),
        },
    )
    for node_id in ids:
        classname, name = common._node_key(node_id)
        case = ET.SubElement(
            suite, "testcase", {"classname": classname, "name": name, "time": "1.000"}
        )
        if statuses[node_id] == "skipped":
            ET.SubElement(case, "skipped", {"message": "upstream platform skip"})
    root = ET.Element("testsuites")
    root.append(suite)
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)


class KeyboardOraclePromotionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.upstream = self.root / "upstream"
        source = self.upstream / keyboard.SOURCE
        source.parent.mkdir(parents=True)
        source.write_text("# pinned upstream test\n")
        unprofiled = [
            f"{keyboard.SOURCE}::test_case_{index:02d}" for index in range(29)
        ]
        self.ids = [node_id + "[chromium]" for node_id in unprofiled]
        self.statuses = {node_id: "passed" for node_id in self.ids}
        self.statuses[self.ids[-1]] = "skipped"
        self.manifest = self.root / "manifest.json"
        self.manifest.write_text(
            json.dumps(
                {
                    "schema": 1,
                    "cases": [{"id": node_id} for node_id in unprofiled],
                    "files": {keyboard.SOURCE: _sha(source)},
                }
            )
        )
        self.inventory = self.root / "inventory.jsonl"
        self.base = self.root / "base.jsonl"
        rows = [
            {
                "id": node_id,
                "api": "async",
                "sourceClassification": "public-api-source",
                "privateImports": [],
                "outcome": "not-run",
                "attempts": 0,
            }
            for node_id in self.ids
        ]
        rows.extend(
            {
                "id": f"tests/async/test_other.py::test_{index:04d}[chromium]",
                "outcome": "not-run",
                "attempts": 0,
            }
            for index in range(6526 - 29)
        )
        self.inventory.write_text(
            "".join(
                json.dumps(
                    {
                        key: value
                        for key, value in row.items()
                        if key not in common.GENERATED_FIELDS
                    }
                )
                + "\n"
                for row in rows
            )
        )
        self.base.write_text("".join(json.dumps(row) + "\n" for row in rows))
        self.base.with_suffix(".summary.json").write_text(
            json.dumps(
                {
                    "ledgerSha256": _sha(self.base),
                    "ledger": str(self.base),
                    "inventory": str(self.inventory),
                    "records": 6526,
                    "outcomes": {"not-run": 6526},
                }
            )
        )
        self.report = self.root / "oracle.xml"
        self.report.write_bytes(_report(self.ids, self.statuses))
        case_dir = self.root / "oracle.cases"
        case_dir.mkdir()
        for index, node_id in enumerate(self.ids):
            (case_dir / f"{index:05d}.xml").write_bytes(
                _report([node_id], self.statuses)
            )
        self.provenance = self.root / "oracle.provenance.json"
        self.provenance.write_text(
            json.dumps(
                {
                    "schema": 5,
                    "mode": "oracle",
                    "manifestCases": True,
                    "manifestApi": "async",
                    "isolateCases": True,
                    "isolatedCaseCount": 29,
                    "browsers": ["chromium"],
                    "launchMode": "cdp-adapter",
                    "browserChannel": "chrome",
                    "exitCode": 0,
                    "manifestSha256": _sha(self.manifest),
                    "manifest": str(self.manifest),
                    "playwrightPackageSha256": keyboard.PINNED_SHA256["oraclePackage"],
                    "result": str(self.report),
                    "upstream": str(self.upstream),
                    "caseReports": str(case_dir),
                    "inventory": str(self.inventory),
                    "inventorySha256": _sha(self.inventory),
                }
            )
        )
        self.pins = {
            "manifest": _sha(self.manifest),
            "oracleBase": _sha(self.base),
            "provenance": _sha(self.provenance),
            "report": _sha(self.report),
            "oraclePackage": keyboard.PINNED_SHA256["oraclePackage"],
        }
        pin_patch = patch.object(keyboard, "PINNED_SHA256", self.pins)
        pin_patch.start()
        self.addCleanup(pin_patch.stop)
        verifier = patch.object(
            keyboard, "verify_provenance", side_effect=lambda raw, **_: raw
        )
        verifier.start()
        self.addCleanup(verifier.stop)
        self.spec = {
            "manifest": str(self.manifest),
            "provenance": str(self.provenance),
            "oracleBase": str(self.base),
        }
        self.receipt = self.root / "receipt.json"

    def _merge(self) -> Path:
        rows, keys = summarizer._load_inventory(self.inventory)
        summarizer._load_base(self.base, rows)
        summarizer._apply_report(rows, keys, self.report, set(), replace_base=True)
        output = self.root / "successor.jsonl"
        output.write_bytes(b"".join(summarizer._stable_json_line(row) for row in rows))
        output.with_suffix(".summary.json").write_text(
            json.dumps(
                {
                    "ledgerSha256": _sha(output),
                    "ledger": str(output),
                    "inventory": str(self.inventory),
                    "records": 6526,
                    "outcomes": dict(Counter(row["outcome"] for row in rows)),
                }
            )
        )
        return output

    def test_exact_source_bound_receipt_and_successor(self) -> None:
        # Arrange
        source = (self.upstream / keyboard.SOURCE).read_bytes()

        # Act
        receipt = keyboard.create(self.spec, self.receipt)
        successor = self._merge()
        verified = keyboard.verify_ledger(self.receipt, successor)

        # Assert
        self.assertEqual(receipt, keyboard.verify(self.receipt))
        self.assertEqual(
            verified["transitions"], {"not-run->passed": 28, "not-run->skipped": 1}
        )
        self.assertEqual((self.upstream / keyboard.SOURCE).read_bytes(), source)

    def test_source_hash_drift_is_rejected(self) -> None:
        # Arrange
        (self.upstream / keyboard.SOURCE).write_text("# drift\n")

        # Act / Assert
        with self.assertRaisesRegex(common.SelectionError, "upstream source drift"):
            keyboard.create(self.spec, self.receipt)

    def test_missing_isolated_case_is_rejected(self) -> None:
        # Arrange
        next((self.root / "oracle.cases").iterdir()).unlink()

        # Act / Assert
        with self.assertRaisesRegex(
            common.SelectionError, "isolated report file count"
        ):
            keyboard.create(self.spec, self.receipt)

    def test_unrelated_successor_mutation_is_rejected(self) -> None:
        # Arrange
        keyboard.create(self.spec, self.receipt)
        successor = self._merge()
        rows = [json.loads(line) for line in successor.read_text().splitlines()]
        rows[-1]["outcome"] = "passed"
        successor.write_bytes(
            b"".join(summarizer._stable_json_line(row) for row in rows)
        )
        summary = json.loads(successor.with_suffix(".summary.json").read_text())
        summary["ledgerSha256"] = _sha(successor)
        summary["outcomes"] = dict(Counter(row["outcome"] for row in rows))
        successor.with_suffix(".summary.json").write_text(json.dumps(summary))

        # Act / Assert
        with self.assertRaisesRegex(common.SelectionError, "unrelated oracle row"):
            keyboard.verify_ledger(self.receipt, successor)

    def test_existing_receipt_is_not_overwritten(self) -> None:
        # Arrange
        self.receipt.write_text("keep")

        # Act / Assert
        with self.assertRaisesRegex(common.SelectionError, "destination exists"):
            keyboard.create(self.spec, self.receipt)
        self.assertEqual(self.receipt.read_text(), "keep")


if __name__ == "__main__":
    unittest.main()
