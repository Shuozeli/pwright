#!/usr/bin/env python3
"""Hermetic checks for the complete-report candidate-only 35-case refresh."""

from __future__ import annotations

import hashlib
import json
import unittest
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

import promote_playwright_python_verified_subset as prior
import refresh_playwright_python_browsercontext_candidate as refresh
import summarize_playwright_python_official_results as summarizer


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _junit(ids: list[str], statuses: dict[str, str]) -> bytes:
    suite = ET.Element(
        "testsuite",
        {
            "tests": str(len(ids)),
            "failures": str(sum(statuses[node_id] == "failed" for node_id in ids)),
            "errors": "0",
            "skipped": "0",
            "time": str(len(ids)),
        },
    )
    for node_id in ids:
        classname, name = prior._node_key(node_id)
        case = ET.SubElement(
            suite, "testcase", {"classname": classname, "name": name, "time": "1.000"}
        )
        if statuses[node_id] == "failed":
            ET.SubElement(case, "failure", {"message": "known gap"})
    root = ET.Element("testsuites")
    root.append(suite)
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)


class CandidateRefreshTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.upstream = self.root / "upstream"
        source = self.upstream / prior.SOURCE
        source.parent.mkdir(parents=True)
        source.write_text("# fixed source\n")
        unprofiled = [f"{prior.SOURCE}::test_case_{index:02d}" for index in range(35)]
        unprofiled.extend(
            node_id.removesuffix("[chromium]") for node_id in sorted(prior.EXCLUDED)
        )
        self.ids = [node_id + "[chromium]" for node_id in unprofiled]
        self.selected = self.ids[:35]
        self.new_statuses = {
            node_id: ("failed" if node_id in self.selected[:6] else "passed")
            for node_id in self.ids
        }
        self.new_statuses[min(prior.EXCLUDED)] = "failed"
        self.old_statuses = {
            node_id: ("failed" if node_id in self.selected[:13] else "passed")
            for node_id in self.selected
        }
        self.manifest = self.root / "manifest.json"
        self.manifest.write_text(
            json.dumps(
                {
                    "schema": 1,
                    "cases": [{"id": node_id} for node_id in unprofiled],
                    "files": {prior.SOURCE: _sha(source)},
                }
            )
        )
        self.inventory = self.root / "inventory.jsonl"
        self.base = self.root / "candidate-base.jsonl"
        rows = []
        for node_id in self.ids:
            outcome = self.old_statuses.get(node_id, "not-run")
            rows.append(
                {
                    "id": node_id,
                    "api": "async",
                    "sourceClassification": "public-api-source",
                    "privateImports": [],
                    "outcome": outcome,
                    "attempts": 0 if outcome == "not-run" else 1,
                }
            )
        rows.extend(
            {
                "id": f"tests/async/test_other.py::test_{index:04d}[chromium]",
                "outcome": "not-run",
                "attempts": 0,
            }
            for index in range(6526 - 39)
        )
        self.inventory.write_text(
            "".join(
                json.dumps(
                    {
                        key: value
                        for key, value in row.items()
                        if key not in prior.GENERATED_FIELDS
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
                    "outcomes": dict(Counter(row["outcome"] for row in rows)),
                }
            )
        )
        self.oracle_ledger = self.root / "oracle.jsonl"
        self.oracle_ledger.write_text("oracle evidence\n")
        self.prior_receipt = self.root / "prior.json"
        self.prior_receipt.write_text("{}\n")
        self.oracle_sidecar = self.root / "oracle.provenance.json"
        fields = {
            "commit": "fixed",
            "tag": "v1.58.0",
            "repository": "https://example.test/playwright-python.git",
            "manifestSha256": _sha(self.manifest),
            "fixtureHost": "fixture.test",
            "fixtureBind": "100.0.0.1",
            "isolatedCaseIdsSha256": "fixed",
            "caseTimeout": 30,
            "inventory": str(self.inventory),
            "inventorySha256": _sha(self.inventory),
        }
        self.oracle_sidecar.write_text(json.dumps(fields))
        self.previous = {
            "manifest": str(self.manifest),
            "manifestSha256": _sha(self.manifest),
            "inventory": str(self.inventory),
            "inventorySha256": _sha(self.inventory),
            "selectedIds": self.selected,
            "candidate": {
                "selectedStatuses": [
                    {"id": node_id, "outcome": self.old_statuses[node_id]}
                    for node_id in self.selected
                ]
            },
            "oracle": {"parentProvenance": str(self.oracle_sidecar)},
        }
        self.report = self.root / "candidate.xml"
        self.report.write_bytes(_junit(self.ids, self.new_statuses))
        case_dir = self.root / "candidate.cases"
        case_dir.mkdir()
        for index, node_id in enumerate(self.ids):
            (case_dir / f"{index:05d}.xml").write_bytes(
                _junit([node_id], self.new_statuses)
            )
        self.sidecar = self.root / "candidate.provenance.json"
        self.sidecar.write_text(
            json.dumps(
                {
                    **fields,
                    "schema": 5,
                    "mode": "candidate",
                    "manifestCases": True,
                    "manifestApi": "async",
                    "isolateCases": True,
                    "isolatedCaseCount": 39,
                    "browsers": ["chromium"],
                    "launchMode": "cdp-adapter",
                    "browserChannel": "chrome",
                    "manifest": str(self.manifest),
                    "exitCode": 1,
                    "pwrightArtifactSha256": refresh.PINNED_SHA256["candidateArtifact"],
                    "result": str(self.report),
                    "upstream": str(self.upstream),
                    "caseReports": str(case_dir),
                }
            )
        )
        self.filtered = self.root / "candidate-35.xml"
        self.receipt = self.root / "refresh.json"
        self.spec = {
            "manifest": str(self.manifest),
            "priorReceipt": str(self.prior_receipt),
            "candidateBase": str(self.base),
            "oracleLedger": str(self.oracle_ledger),
            "candidateProvenance": str(self.sidecar),
            "filteredReport": str(self.filtered),
        }

        def pinned(path: Path, _key: str) -> str:
            return prior._file_sha(path)

        def verified(_path: Path) -> dict[str, object]:
            return self.previous

        def ledgers(*_args: object) -> dict[str, bool]:
            return {"verified": True}

        def provenance(raw: dict[str, object], **_kwargs: object) -> dict[str, object]:
            return raw

        for target, replacement, handler in (
            (refresh, "_require_pinned", pinned),
            (prior, "verify", verified),
            (prior, "verify_ledgers", ledgers),
            (refresh, "verify_provenance", provenance),
        ):
            active = patch.object(target, replacement, side_effect=handler)
            active.start()
            self.addCleanup(active.stop)

    def _merge(self) -> Path:
        rows, keys = summarizer._load_inventory(self.inventory)
        summarizer._load_base(self.base, rows)
        summarizer._apply_report(rows, keys, self.filtered, set(), replace_base=True)
        output = self.root / "successor.jsonl"
        payload = b"".join(summarizer._stable_json_line(row) for row in rows)
        output.write_bytes(payload)
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

    def test_exact_refresh_and_successor_verify(self) -> None:
        # Arrange
        parent = self.report.read_bytes()

        # Act
        receipt = refresh.create(self.spec, self.receipt)
        successor = self._merge()
        ledger = refresh.verify_ledger(self.receipt, successor)

        # Assert
        self.assertEqual(refresh.verify(self.receipt), receipt)
        self.assertEqual(self.report.read_bytes(), parent)
        self.assertEqual(
            receipt["transitions"],
            {"failed->failed": 6, "failed->passed": 7, "passed->passed": 22},
        )
        self.assertEqual(len(receipt["originalStatuses"]), 39)
        self.assertEqual(len(receipt["selectedStatuses"]), 35)
        self.assertTrue(ledger["verified"])

    def test_report_status_drift_is_rejected(self) -> None:
        # Arrange
        self.new_statuses[self.selected[6]] = "failed"
        self.report.write_bytes(_junit(self.ids, self.new_statuses))
        (self.root / "candidate.cases/00006.xml").write_bytes(
            _junit([self.selected[6]], self.new_statuses)
        )

        # Act / Assert
        with self.assertRaisesRegex(prior.SelectionError, "29/6"):
            refresh.create(self.spec, self.receipt)
        self.assertFalse(self.receipt.exists())

    def test_missing_isolated_report_is_rejected(self) -> None:
        # Arrange
        isolated = next((self.root / "candidate.cases").iterdir())
        isolated.unlink()

        # Act / Assert
        with self.assertRaisesRegex(prior.SelectionError, "isolated report file count"):
            refresh.create(self.spec, self.receipt)

    def test_existing_output_is_not_overwritten(self) -> None:
        # Arrange
        self.filtered.write_text("keep")

        # Act / Assert
        with self.assertRaisesRegex(prior.SelectionError, "destination exists"):
            refresh.create(self.spec, self.receipt)
        self.assertEqual(self.filtered.read_text(), "keep")

    def test_unrelated_successor_mutation_is_rejected(self) -> None:
        # Arrange
        refresh.create(self.spec, self.receipt)
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
        with self.assertRaisesRegex(prior.SelectionError, "unrelated refresh row"):
            refresh.verify_ledger(self.receipt, successor)


if __name__ == "__main__":
    unittest.main()
