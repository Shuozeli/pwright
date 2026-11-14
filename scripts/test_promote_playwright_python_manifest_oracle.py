#!/usr/bin/env python3
"""Hermetic checks for complete-manifest oracle selection and strict merge."""

from __future__ import annotations

import hashlib
import json
import unittest
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

import promote_playwright_python_manifest_oracle as promotion
import promote_playwright_python_verified_subset as common
import summarize_playwright_python_official_results as summarizer


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _junit(ids: list[str]) -> bytes:
    suite = ET.Element(
        "testsuite",
        {
            "tests": str(len(ids)),
            "failures": "0",
            "errors": "0",
            "skipped": "0",
            "time": str(len(ids)),
        },
    )
    for node_id in ids:
        classname, name = common._node_key(node_id)
        ET.SubElement(
            suite, "testcase", {"classname": classname, "name": name, "time": "1.000"}
        )
    root = ET.Element("testsuites")
    root.append(suite)
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)


class CompleteManifestOracleTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.upstream = self.root / "upstream"
        self.source = self.upstream / "tests/async/test_example.py"
        self.source.parent.mkdir(parents=True)
        self.source.write_text("# pinned upstream source\n")
        self.ids = [
            "tests/async/test_example.py::test_public[chromium]",
            "tests/async/test_example.py::test_internal[chromium]",
        ]
        self.manifest = self.root / "manifest.json"
        self.manifest.write_text(
            json.dumps(
                {
                    "schema": 1,
                    "repository": promotion.PINNED_REPOSITORY,
                    "tag": promotion.PINNED_TAG,
                    "commit": promotion.PINNED_COMMIT,
                    "cases": [
                        {"id": node_id.removesuffix("[chromium]")}
                        for node_id in self.ids
                    ],
                    "files": {"tests/async/test_example.py": _sha(self.source)},
                }
            )
        )
        self.inventory = self.root / "inventory.jsonl"
        self.base = self.root / "base.jsonl"
        rows = [
            {"id": node_id, "api": "async", "outcome": outcome, "attempts": attempts}
            for node_id, outcome, attempts in (
                (self.ids[0], "not-run", 0),
                (self.ids[1], "passed", 1),
            )
        ]
        rows.extend(
            {
                "id": f"tests/async/test_other.py::test_{index:04d}[chromium]",
                "outcome": "not-run",
                "attempts": 0,
            }
            for index in range(6524)
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
                    "outcomes": {"not-run": 6525, "passed": 1},
                }
            )
        )
        self.audit = self.root / "audit.jsonl"
        self.audit.write_text(
            "".join(
                json.dumps(
                    {
                        "id": row["id"],
                        "applicability": "non-applicable"
                        if index == 1
                        else "applicable",
                        "invocationClassification": "implementation-internal"
                        if index == 1
                        else "public-api-candidate",
                    }
                )
                + "\n"
                for index, row in enumerate(rows)
            )
        )
        self.audit.with_suffix(".summary.json").write_text(
            json.dumps(
                {
                    "ledgerSha256": _sha(self.audit),
                    "outcomeLedger": str(self.base),
                    "outcomeLedgerSha256": _sha(self.base),
                    "records": 6526,
                }
            )
        )
        audit_pin = patch.object(
            promotion, "PINNED_INVOCATION_SHA256", _sha(self.audit)
        )
        audit_pin.start()
        self.addCleanup(audit_pin.stop)
        self.report = self.root / "oracle.xml"
        self.report.write_bytes(_junit(self.ids))
        case_dir = self.root / "oracle.xml.cases"
        case_dir.mkdir()
        for index, node_id in enumerate(self.ids):
            (case_dir / f"{index:05d}.xml").write_bytes(_junit([node_id]))
        self.provenance = self.root / "oracle.provenance.json"
        self.provenance.write_text(
            json.dumps(
                {
                    "schema": 5,
                    "mode": "oracle",
                    "repository": promotion.PINNED_REPOSITORY,
                    "tag": promotion.PINNED_TAG,
                    "commit": promotion.PINNED_COMMIT,
                    "manifestCases": True,
                    "manifestApi": "async",
                    "isolateCases": True,
                    "isolatedCaseCount": 2,
                    "browsers": ["chromium"],
                    "launchMode": "cdp-adapter",
                    "browserChannel": "chrome",
                    "exitCode": 0,
                    "manifestSha256": _sha(self.manifest),
                    "manifest": str(self.manifest),
                    "result": str(self.report),
                    "upstream": str(self.upstream),
                    "caseReports": str(case_dir),
                    "inventory": str(self.inventory),
                    "inventorySha256": _sha(self.inventory),
                    "playwrightPackageSha256": "a" * 64,
                }
            )
        )
        verifier = patch.object(
            promotion, "verify_provenance", side_effect=lambda raw, **_: raw
        )
        verifier.start()
        self.addCleanup(verifier.stop)
        self.filtered = self.root / "filtered.xml"
        self.receipt = self.root / "receipt.json"
        self.spec = {
            "manifest": str(self.manifest),
            "provenance": str(self.provenance),
            "oracleBase": str(self.base),
            "invocationAudit": str(self.audit),
            "filteredReport": str(self.filtered),
        }

    def _merge(self) -> Path:
        rows, keys = summarizer._load_inventory(self.inventory)
        summarizer._load_base(self.base, rows)
        summarizer._apply_report(rows, keys, self.filtered, set(), replace_base=True)
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

    def test_only_applicable_public_case_is_promoted(self) -> None:
        # Arrange
        original_report = self.report.read_bytes()

        # Act
        receipt = promotion.create(self.spec, self.receipt)
        successor = self._merge()
        result = promotion.verify_ledger(self.receipt, successor)

        # Assert
        self.assertEqual(receipt, promotion.verify(self.receipt))
        self.assertEqual(receipt["selectedIds"], [self.ids[0]])
        self.assertEqual(
            [entry["id"] for entry in receipt["excludedIds"]], [self.ids[1]]
        )
        self.assertEqual(result["transitions"], {"not-run->passed": 1})
        self.assertEqual(self.report.read_bytes(), original_report)

    def test_upstream_source_drift_is_rejected(self) -> None:
        # Arrange
        self.source.write_text("# drift\n")

        # Act / Assert
        with self.assertRaisesRegex(common.SelectionError, "upstream source drift"):
            promotion.create(self.spec, self.receipt)

    def test_missing_isolated_case_is_rejected(self) -> None:
        # Arrange
        next((self.root / "oracle.xml.cases").iterdir()).unlink()

        # Act / Assert
        with self.assertRaisesRegex(
            common.SelectionError, "isolated report file count"
        ):
            promotion.create(self.spec, self.receipt)

    def test_non_applicable_public_case_is_rejected(self) -> None:
        # Arrange
        rows = [json.loads(line) for line in self.audit.read_text().splitlines()]
        rows[1]["invocationClassification"] = "public-api-candidate"
        self.audit.write_text("".join(json.dumps(row) + "\n" for row in rows))
        summary = json.loads(self.audit.with_suffix(".summary.json").read_text())
        summary["ledgerSha256"] = _sha(self.audit)
        self.audit.with_suffix(".summary.json").write_text(json.dumps(summary))

        # Act / Assert
        with (
            patch.object(promotion, "PINNED_INVOCATION_SHA256", _sha(self.audit)),
            self.assertRaisesRegex(common.SelectionError, "unclassified oracle"),
        ):
            promotion.create(self.spec, self.receipt)

    def test_unrelated_successor_change_is_rejected(self) -> None:
        # Arrange
        promotion.create(self.spec, self.receipt)
        successor = self._merge()
        rows = [json.loads(line) for line in successor.read_text().splitlines()]
        rows[2]["outcome"] = "passed"
        successor.write_bytes(
            b"".join(summarizer._stable_json_line(row) for row in rows)
        )
        summary = json.loads(successor.with_suffix(".summary.json").read_text())
        summary["ledgerSha256"] = _sha(successor)
        summary["outcomes"] = dict(Counter(row["outcome"] for row in rows))
        successor.with_suffix(".summary.json").write_text(json.dumps(summary))

        # Act / Assert
        with self.assertRaisesRegex(common.SelectionError, "unrelated oracle row"):
            promotion.verify_ledger(self.receipt, successor)


if __name__ == "__main__":
    unittest.main()
