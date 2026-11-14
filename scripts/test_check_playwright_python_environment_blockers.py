#!/usr/bin/env python3
"""Browserless guards for the four historical Python fixture-host failures."""

from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from check_playwright_python_environment_blockers import (
    EXPECTED_CASES,
    EvidenceError,
    failure_matches_signature,
    verify_shared_raw_failures,
)


def _failed_rows() -> dict[str, dict[str, object]]:
    return {
        node_id: {
            "id": node_id,
            "outcome": "failed",
            "sourceClassification": "public-api-source",
            "privateImports": [],
            "attempts": 1,
            "error": "AssertionError: fixture host differs",
        }
        for node_id in EXPECTED_CASES
    }


class PythonEnvironmentBlockerTests(unittest.TestCase):
    def test_registry_pins_only_the_four_known_public_failures(self) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        path = (
            root / "packages/pwright-python/tests/official/python-fixture-environment-blockers.json"
        )

        # Act
        registry = json.loads(path.read_text())
        cases = {case["id"]: (case["run"], case["signature"]) for case in registry["cases"]}

        # Assert
        self.assertEqual(registry["ledgerRows"], 6526)
        self.assertEqual(cases, EXPECTED_CASES)

    def test_shared_public_failures_pass_without_mutating_outcomes(self) -> None:
        # Arrange
        candidate = _failed_rows()
        oracle = copy.deepcopy(candidate)
        original_candidate = copy.deepcopy(candidate)
        original_oracle = copy.deepcopy(oracle)

        # Act
        verify_shared_raw_failures(candidate, oracle, set(EXPECTED_CASES))

        # Assert
        self.assertEqual(candidate, original_candidate)
        self.assertEqual(oracle, original_oracle)
        self.assertEqual({row["outcome"] for row in candidate.values()}, {"failed"})

    def test_candidate_pass_promotion_is_rejected(self) -> None:
        # Arrange
        candidate = _failed_rows()
        oracle = _failed_rows()
        first_id = next(iter(EXPECTED_CASES))
        candidate[first_id]["outcome"] = "passed"

        # Act and Assert
        with self.assertRaisesRegex(EvidenceError, "candidate failed IDs differ"):
            verify_shared_raw_failures(candidate, oracle, set(EXPECTED_CASES))

    def test_oracle_failure_id_mismatch_is_rejected(self) -> None:
        # Arrange
        candidate = _failed_rows()
        oracle = _failed_rows()
        oracle["tests/async/test_other.py::test_other[chromium]"] = {"outcome": "failed"}

        # Act and Assert
        with self.assertRaisesRegex(EvidenceError, "oracle failed IDs differ"):
            verify_shared_raw_failures(candidate, oracle, set(EXPECTED_CASES))

    def test_public_classification_cannot_be_relabelled_internal(self) -> None:
        # Arrange
        candidate = _failed_rows()
        oracle = _failed_rows()
        first_id = next(iter(EXPECTED_CASES))
        candidate[first_id]["sourceClassification"] = "implementation-internal"

        # Act and Assert
        with self.assertRaisesRegex(EvidenceError, "lost public classification"):
            verify_shared_raw_failures(candidate, oracle, set(EXPECTED_CASES))

    def test_cookie_default_signature_requires_both_domains(self) -> None:
        # Arrange
        host = "macbookpro-yuacx.tail8f3b66.ts.net"
        failure = f"'domain': '{host}' != 'domain': 'localhost'"

        # Act
        matching = failure_matches_signature(failure, "cookie-default-domain", host)
        missing_expected = failure_matches_signature(
            f"'domain': '{host}'", "cookie-default-domain", host
        )

        # Assert
        self.assertTrue(matching)
        self.assertFalse(missing_expected)

    def test_cookie_path_signature_requires_magicdns_page_and_empty_cookie(
        self,
    ) -> None:
        # Arrange
        host = "macbookpro-yuacx.tail8f3b66.ts.net"
        failure = f"Page url='http://{host}:1234/grid.html'\nassert '' == 'gridcookie=GRID'"

        # Act
        matching = failure_matches_signature(failure, "cookie-path-domain", host)
        wrong_host = failure_matches_signature(
            "Page url='http://localhost:1234/grid.html'\nassert '' == 'gridcookie=GRID'",
            "cookie-path-domain",
            host,
        )

        # Assert
        self.assertTrue(matching)
        self.assertFalse(wrong_host)

    def test_indexeddb_signature_requires_actual_and_expected_origins(self) -> None:
        # Arrange
        host = "macbookpro-yuacx.tail8f3b66.ts.net"
        failure = f"'origin': 'http://{host}:1234' != 'origin': 'http://localhost:1234'"

        # Act
        matching = failure_matches_signature(failure, "indexeddb-origin", host)
        missing_actual = failure_matches_signature(
            "'origin': 'http://localhost:1234'", "indexeddb-origin", host
        )

        # Assert
        self.assertTrue(matching)
        self.assertFalse(missing_actual)


if __name__ == "__main__":
    unittest.main()
