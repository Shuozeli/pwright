#!/usr/bin/env python3
"""Hermetic tests for source-bound 35-of-39 Python result derivation."""

from __future__ import annotations

import hashlib
import json
import unittest
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

import promote_playwright_python_verified_subset as selection
import summarize_playwright_python_official_results as summarizer


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _junit(ids: list[str], statuses: dict[str, str]) -> bytes:
    suite = ET.Element(
        "testsuite",
        {
            "name": "pytest-isolated",
            "tests": str(len(ids)),
            "failures": str(sum(statuses[node_id] == "failed" for node_id in ids)),
            "errors": "0",
            "skipped": "0",
            "time": str(len(ids)),
        },
    )
    for node_id in ids:
        classname, name = selection._node_key(node_id)
        case = ET.SubElement(
            suite,
            "testcase",
            {
                "classname": classname,
                "name": name,
                "time": "1.000",
            },
        )
        if statuses[node_id] == "failed":
            ET.SubElement(case, "failure", {"message": "known gap"})
    root = ET.Element("testsuites", {"name": "pytest isolated tests"})
    root.append(suite)
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)


class VerifiedSubsetTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.upstream = self.root / "upstream"
        source = self.upstream / selection.SOURCE
        source.parent.mkdir(parents=True)
        source.write_text("# pinned source fixture\n")
        manifest_ids = [f"{selection.SOURCE}::test_case_{index:02d}" for index in range(35)] + [
            node_id.removesuffix("[chromium]") for node_id in sorted(selection.EXCLUDED)
        ]
        self.ids = [node_id + "[chromium]" for node_id in manifest_ids]
        self.selected = self.ids[:35]
        self.manifest_path = self.root / "manifest.json"
        self.manifest_path.write_text(
            json.dumps(
                {
                    "schema": 1,
                    "cases": [{"id": node_id} for node_id in manifest_ids],
                    "files": {selection.SOURCE: _sha(source.read_bytes())},
                }
            )
        )
        manifest_sha = _sha(self.manifest_path.read_bytes())
        self.inventory = str((self.root / "inventory.jsonl").resolve())
        self.statuses = {
            "candidate": {
                node_id: (
                    "failed"
                    if node_id in self.selected[:13] or node_id == sorted(selection.EXCLUDED)[0]
                    else "passed"
                )
                for node_id in self.ids
            },
            "oracle": {
                node_id: ("failed" if node_id in selection.EXCLUDED else "passed")
                for node_id in self.ids
            },
        }
        self.spec = {
            "manifest": str(self.manifest_path),
            "selectedIds": self.selected,
            "excludedIds": sorted(selection.EXCLUDED),
        }
        self.pinned_base_hashes: dict[str, str] = {}
        for mode in ("candidate", "oracle"):
            result = self.root / f"{mode}.xml"
            result.write_bytes(_junit(self.ids, self.statuses[mode]))
            case_dir = self.root / f"{mode}.cases"
            case_dir.mkdir()
            for index, node_id in enumerate(self.ids):
                (case_dir / f"{index:05d}.xml").write_bytes(_junit([node_id], self.statuses[mode]))
            provenance = self.root / f"{mode}.provenance.json"
            provenance.write_text(
                json.dumps(
                    {
                        "schema": 5,
                        "mode": mode,
                        "manifestCases": True,
                        "manifestApi": "async",
                        "isolateCases": True,
                        "isolatedCaseCount": 39,
                        "browsers": ["chromium"],
                        "manifestSha256": manifest_sha,
                        "manifest": str(self.manifest_path),
                        "launchMode": "cdp-adapter",
                        "browserChannel": "chrome",
                        "result": str(result),
                        "inventory": self.inventory,
                        "upstream": str(self.upstream),
                        "caseReports": str(case_dir),
                        "commit": "pinned",
                        "tag": "test",
                        "repository": "https://example.test",
                        "fixtureHost": "fixture.test",
                        "fixtureBind": "100.0.0.1",
                        "isolatedCaseIdsSha256": "pinned",
                        "caseTimeout": 30,
                        "inventorySha256": "pinned",
                    }
                )
            )
            base = self.root / f"{mode}-base.jsonl"
            rows = []
            for node_id in self.ids:
                base_status = "not-run"
                if mode == "candidate" and node_id in self.selected[-14:]:
                    base_status = "passed"
                rows.append(
                    {
                        "id": node_id,
                        "outcome": base_status,
                        "attempts": 0 if base_status == "not-run" else 1,
                        "api": "async",
                        "sourceClassification": "public-api-source",
                        "privateImports": [],
                    }
                )
            for index in range(6526 - 39):
                rows.append(
                    {
                        "id": f"tests/async/test_other.py::test_{index:04d}[chromium]",
                        "outcome": "not-run",
                        "attempts": 0,
                    }
                )
            Path(self.inventory).write_text(
                "".join(
                    json.dumps(
                        {
                            key: value
                            for key, value in row.items()
                            if key not in selection.GENERATED_FIELDS
                        }
                    )
                    + "\n"
                    for row in rows
                )
            )
            base.write_text("".join(json.dumps(row) + "\n" for row in rows))
            counts: dict[str, int] = {}
            for row in rows:
                counts[row["outcome"]] = counts.get(row["outcome"], 0) + 1
            base.with_suffix(".summary.json").write_text(
                json.dumps(
                    {
                        "ledgerSha256": _sha(base.read_bytes()),
                        "ledger": str(base),
                        "inventory": self.inventory,
                        "records": 6526,
                        "outcomes": counts,
                    }
                )
            )
            self.spec[mode] = {
                "parentProvenance": str(provenance),
                "parentReport": str(result),
                "baseLedger": str(base),
                "expectedBaseSha256": _sha(base.read_bytes()),
                "filteredReport": str(self.root / f"{mode}-35.xml"),
            }
            self.pinned_base_hashes[mode] = _sha(base.read_bytes())
        self.receipt_path = self.root / "receipt.json"
        self.digest_patch = patch.object(selection, "PINNED_BASE_SHA256", self.pinned_base_hashes)
        self.digest_patch.start()
        self.addCleanup(self.digest_patch.stop)
        self.verify_patch = patch.object(
            selection, "verify_provenance", side_effect=lambda raw, **_: raw
        )
        self.verify_patch.start()
        self.addCleanup(self.verify_patch.stop)

    def _write_merged_ledgers(self) -> dict[str, Path]:
        selection.create(self.spec, self.receipt_path)
        outputs: dict[str, Path] = {}
        for mode in ("candidate", "oracle"):
            records, keys = summarizer._load_inventory(Path(self.inventory))
            summarizer._load_base(Path(self.spec[mode]["baseLedger"]), records)
            summarizer._apply_report(
                records,
                keys,
                Path(self.spec[mode]["filteredReport"]).resolve(),
                set(),
                replace_base=True,
            )
            output = self.root / f"{mode}-strict.jsonl"
            payload = b"".join(summarizer._stable_json_line(row) for row in records)
            output.write_bytes(payload)
            output.with_suffix(".summary.json").write_text(
                json.dumps(
                    {
                        "schema": 1,
                        "inventory": self.inventory,
                        "records": len(records),
                        "ledger": str(output),
                        "ledgerSha256": _sha(payload),
                        "outcomes": dict(
                            sorted(Counter(row["outcome"] for row in records).items())
                        ),
                    }
                )
            )
            outputs[mode] = output
        return outputs

    def _rewrite_ledger(self, path: Path, rows: list[dict[str, object]]) -> None:
        payload = b"".join(summarizer._stable_json_line(row) for row in rows)
        path.write_bytes(payload)
        summary = path.with_suffix(".summary.json")
        metadata = json.loads(summary.read_text())
        metadata["ledgerSha256"] = _sha(payload)
        metadata["outcomes"] = dict(sorted(Counter(row["outcome"] for row in rows).items()))
        summary.write_text(json.dumps(metadata))

    def test_creates_and_reverifies_exact_source_bound_derivation(self) -> None:
        # Arrange
        original = {
            mode: Path(self.spec[mode]["parentReport"]).read_bytes()
            for mode in ("candidate", "oracle")
        }

        # Act
        receipt = selection.create(self.spec, self.receipt_path)
        verified = selection.verify(self.receipt_path)

        # Assert
        self.assertEqual(receipt, verified)
        self.assertEqual(len(receipt["selectedIds"]), 35)
        self.assertEqual(len(receipt["originalStatuses"]), 39)
        self.assertEqual(
            sum(row["outcome"] == "failed" for row in receipt["candidate"]["selectedStatuses"]),
            13,
        )
        self.assertTrue(
            all(row["outcome"] == "passed" for row in receipt["oracle"]["selectedStatuses"])
        )
        for mode in ("candidate", "oracle"):
            self.assertEqual(Path(self.spec[mode]["parentReport"]).read_bytes(), original[mode])
            self.assertEqual(
                len(list(ET.parse(self.spec[mode]["filteredReport"]).iter("testcase"))),
                35,
            )

    def test_rejects_existing_destination_without_overwriting(self) -> None:
        # Arrange
        target = Path(self.spec["candidate"]["filteredReport"])
        target.write_text("keep me")

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "destination exists"):
            selection.create(self.spec, self.receipt_path)
        self.assertEqual(target.read_text(), "keep me")
        self.assertFalse(self.receipt_path.exists())

    def test_rejects_aliased_output_collision(self) -> None:
        # Arrange
        alias = self.root / "alias"
        alias.symlink_to(self.root, target_is_directory=True)
        self.spec["oracle"]["filteredReport"] = str(
            alias / Path(self.spec["candidate"]["filteredReport"]).name
        )

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "output paths collide"):
            selection.create(self.spec, self.receipt_path)
        self.assertFalse(Path(self.spec["candidate"]["filteredReport"]).exists())

    def test_rejects_base_summary_digest_drift(self) -> None:
        # Arrange
        base = Path(self.spec["candidate"]["baseLedger"])
        base.write_bytes(base.read_bytes() + b"\n")

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "base summary digest drift"):
            selection.create(self.spec, self.receipt_path)

    def test_rejects_valid_but_unpinned_base_revision(self) -> None:
        # Arrange
        base = Path(self.spec["oracle"]["baseLedger"])
        rows = [json.loads(line) for line in base.read_text().splitlines()]
        rows[-1]["outcome"] = "passed"  # unrelated older/newer revision, valid summary
        rows[-1]["attempts"] = 1
        base.write_text("".join(json.dumps(row) + "\n" for row in rows))
        summary = base.with_suffix(".summary.json")
        payload = json.loads(summary.read_text())
        payload["ledgerSha256"] = _sha(base.read_bytes())
        payload["outcomes"] = {"passed": 1, "not-run": 6525}
        summary.write_text(json.dumps(payload))

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "base SHA-256 drift"):
            selection.create(self.spec, self.receipt_path)
        self.assertFalse(self.receipt_path.exists())

    def test_rejects_unpinned_base_hash_argument(self) -> None:
        # Arrange
        self.spec["candidate"]["expectedBaseSha256"] = "0" * 64

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "expected base SHA-256"):
            selection.create(self.spec, self.receipt_path)

    def test_rejects_prior_conflicting_outcome(self) -> None:
        # Arrange
        base = Path(self.spec["candidate"]["baseLedger"])
        rows = [json.loads(line) for line in base.read_text().splitlines()]
        rows[0]["outcome"] = "passed"  # first selected case failed in new evidence
        base.write_text("".join(json.dumps(row) + "\n" for row in rows))
        summary = base.with_suffix(".summary.json")
        payload = json.loads(summary.read_text())
        payload["ledgerSha256"] = _sha(base.read_bytes())
        payload["outcomes"] = {"passed": 15, "not-run": 6511}
        summary.write_text(json.dumps(payload))

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "base outcome collision"):
            selection.create(self.spec, self.receipt_path)

    def test_rejects_aggregate_isolated_status_mismatch(self) -> None:
        # Arrange
        isolated = self.root / "candidate.cases" / "00000.xml"
        isolated.write_bytes(_junit([self.ids[0]], {self.ids[0]: "passed"}))

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "aggregate case status differs"):
            selection.create(self.spec, self.receipt_path)

    def test_rejects_unpinned_exclusion(self) -> None:
        # Arrange
        self.spec["excludedIds"][0] = self.selected[0]

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "four explicit exclusions"):
            selection.create(self.spec, self.receipt_path)

    def test_rejects_aggregate_missing_manifest_case(self) -> None:
        # Arrange
        result = Path(self.spec["oracle"]["parentReport"])
        result.write_bytes(_junit(self.ids[:-1], self.statuses["oracle"]))

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "exact manifest selection"):
            selection.create(self.spec, self.receipt_path)

    def test_reverify_rejects_filtered_report_tampering(self) -> None:
        # Arrange
        selection.create(self.spec, self.receipt_path)
        filtered = Path(self.spec["oracle"]["filteredReport"])
        filtered.write_bytes(filtered.read_bytes() + b"\n")

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "filtered report differs"):
            selection.verify(self.receipt_path)

    def test_reverify_rejects_parent_sidecar_drift(self) -> None:
        # Arrange
        selection.create(self.spec, self.receipt_path)
        provenance = Path(self.spec["oracle"]["parentProvenance"])
        provenance.write_bytes(provenance.read_bytes() + b"\n")

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "receipt differs"):
            selection.verify(self.receipt_path)

    def test_verify_ledgers_accepts_exact_strict_merge(self) -> None:
        # Arrange
        ledgers = self._write_merged_ledgers()

        # Act
        result = selection.verify_ledgers(
            self.receipt_path, ledgers["candidate"], ledgers["oracle"]
        )

        # Assert
        self.assertTrue(result["verified"])
        self.assertEqual(
            result["candidate"]["outcomes"],
            {
                "failed": 13,
                "not-run": 6491,
                "passed": 22,
            },
        )
        self.assertEqual(
            result["oracle"]["outcomes"],
            {
                "not-run": 6491,
                "passed": 35,
            },
        )

    def test_verify_ledgers_rejects_unrelated_row_mutation(self) -> None:
        # Arrange
        ledgers = self._write_merged_ledgers()
        path = ledgers["candidate"]
        rows = [json.loads(line) for line in path.read_text().splitlines()]
        rows[-1]["attempts"] = 7
        self._rewrite_ledger(path, rows)

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "unrelated row differs"):
            selection.verify_ledgers(self.receipt_path, path, ledgers["oracle"])

    def test_verify_ledgers_rejects_wrong_result_file(self) -> None:
        # Arrange
        ledgers = self._write_merged_ledgers()
        path = ledgers["candidate"]
        rows = [json.loads(line) for line in path.read_text().splitlines()]
        rows[0]["resultFile"] = self.spec["candidate"]["parentReport"]
        self._rewrite_ledger(path, rows)

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "selected row differs"):
            selection.verify_ledgers(self.receipt_path, path, ledgers["oracle"])

    def test_verify_ledgers_rejects_excluded_case_promotion(self) -> None:
        # Arrange
        ledgers = self._write_merged_ledgers()
        path = ledgers["oracle"]
        rows = [json.loads(line) for line in path.read_text().splitlines()]
        excluded = sorted(selection.EXCLUDED)[0]
        row = next(item for item in rows if item["id"] == excluded)
        row["outcome"] = "failed"
        row["attempts"] = 1
        self._rewrite_ledger(path, rows)

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "excluded case already"):
            selection.verify_ledgers(self.receipt_path, ledgers["candidate"], path)

    def test_verify_ledgers_rejects_new_summary_digest_drift(self) -> None:
        # Arrange
        ledgers = self._write_merged_ledgers()
        path = ledgers["candidate"]
        path.write_bytes(path.read_bytes() + b"\n")

        # Act / Assert
        with self.assertRaisesRegex(selection.SelectionError, "base summary digest drift"):
            selection.verify_ledgers(self.receipt_path, path, ledgers["oracle"])


if __name__ == "__main__":
    unittest.main()
