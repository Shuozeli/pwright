"""Provenance gates for ordinary Playwright Python ledger promotion."""

from __future__ import annotations

import hashlib
import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

from summarize_playwright_python_official_results import (
    _apply_report,
    _generic_result_mode,
    _verified_generic_result,
    main,
)

_ROOT = Path(__file__).parents[1]
_CANDIDATE_RESULT = (
    _ROOT / ".work/official-conformance/results/"
    "playwright-python-candidate-async-select-option-24-abi108-v2.xml"
)
_ORACLE_RESULT = (
    _ROOT / ".work/official-conformance/results/"
    "playwright-python-oracle-sync-select-option-23-pair-20261011-v1.xml"
)


class GenericResultProvenanceTests(unittest.TestCase):
    def test_source_bound_candidate_and_oracle_results_pass_current_verifier(self) -> None:
        # Arrange
        if not _CANDIDATE_RESULT.is_file() or not _ORACLE_RESULT.is_file():
            self.skipTest("source-bound Python select-option receipts are unavailable")

        # Act
        candidate = _verified_generic_result(_CANDIDATE_RESULT, "candidate")
        oracle = _verified_generic_result(_ORACLE_RESULT, "oracle")

        # Assert
        self.assertEqual(candidate, hashlib.sha256(_CANDIDATE_RESULT.read_bytes()).hexdigest())
        self.assertEqual(oracle, hashlib.sha256(_ORACLE_RESULT.read_bytes()).hexdigest())

    def test_generic_result_rejects_missing_or_tampered_sidecar(self) -> None:
        # Arrange
        if not _CANDIDATE_RESULT.is_file():
            self.skipTest("source-bound Python select-option receipt is unavailable")
        with TemporaryDirectory() as directory:
            report = Path(directory) / "report.xml"
            report.write_bytes(_CANDIDATE_RESULT.read_bytes())
            sidecar = report.with_suffix(".provenance.json")

            # Act / Assert
            with self.assertRaisesRegex(SystemExit, "sibling provenance"):
                _verified_generic_result(report, "candidate")
            provenance = json.loads(_CANDIDATE_RESULT.with_suffix(".provenance.json").read_text())
            provenance["result"] = str(report)
            provenance["toolingSources"][1]["sha256"] = "0" * 64
            sidecar.write_text(json.dumps(provenance))
            with self.assertRaisesRegex(SystemExit, "provenance verification failed"):
                _verified_generic_result(report, "candidate")

    def test_generic_result_rejects_wrong_lane_collection_or_legacy_receipt(self) -> None:
        # Arrange
        if not _CANDIDATE_RESULT.is_file():
            self.skipTest("source-bound Python select-option receipt is unavailable")

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "source-bound execution provenance"):
            _verified_generic_result(_CANDIDATE_RESULT, "oracle")
        with (
            patch("verify_playwright_python_official_provenance.verify_provenance", return_value={
                "schema": 5, "mode": "candidate", "result": str(_CANDIDATE_RESULT),
                "collectOnly": True, "bootstrapOnly": False,
            }),
            self.assertRaisesRegex(SystemExit, "source-bound execution provenance"),
        ):
            _verified_generic_result(_CANDIDATE_RESULT, "candidate")
        with (
            patch("verify_playwright_python_official_provenance.verify_provenance", return_value={
                "schema": 1, "mode": "candidate", "result": str(_CANDIDATE_RESULT),
                "collectOnly": False, "bootstrapOnly": False,
            }),
            self.assertRaisesRegex(SystemExit, "source-bound execution provenance"),
        ):
            _verified_generic_result(_CANDIDATE_RESULT, "candidate")

    def test_generic_result_mode_rejects_cross_lane_base(self) -> None:
        # Arrange
        output = Path("playwright-python-candidate-new.jsonl")
        oracle_base = Path("playwright-python-oracle-old.jsonl")

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "same lane"):
            _generic_result_mode(output, oracle_base)
        self.assertEqual(_generic_result_mode(Path("playwright-python-candidate.jsonl"), None), "candidate")

    def test_report_mutation_after_verification_cannot_change_row(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            report = Path(directory) / "report.xml"
            report.write_text("<testsuite />")
            original_digest = hashlib.sha256(report.read_bytes()).hexdigest()
            report.write_text('<testsuite><testcase classname="tests.async.test_page" name="test_one" /></testsuite>')
            records = [{"id": "tests/async/test_page.py::test_one", "outcome": "not-run"}]
            keys = {("tests.async.test_page", "test_one"): 0}

            # Act / Assert
            with self.assertRaisesRegex(SystemExit, "changed after provenance verification"):
                _apply_report(records, keys, report, set(), replace_base=False, verified_digest=original_digest)
            self.assertEqual(records[0]["outcome"], "not-run")

    def test_cli_missing_sidecar_does_not_create_successor(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            inventory = root / "inventory.jsonl"
            inventory.write_text(json.dumps({"id": "tests/async/test_page.py::test_one"}) + "\n")
            report = root / "report.xml"
            report.write_text("<testsuite />")
            output = root / "playwright-python-candidate-gate.jsonl"
            argv = [
                "summarize_playwright_python_official_results.py", "--inventory", str(inventory),
                "--result", str(report), "--output", str(output),
            ]

            # Act / Assert
            with patch("sys.argv", argv), self.assertRaisesRegex(SystemExit, "sibling provenance"):
                main()
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
