"""Focused guards for the immutable Core v22 reconciliation audit."""

from __future__ import annotations

import json
import unittest
from copy import deepcopy
from pathlib import Path
from tempfile import TemporaryDirectory

from audit_playwright_core_v22_reconciliation import (
    SOURCE_ONLY,
    _verify_pinned_file,
    reconcile,
)


def _rows() -> tuple[
    list[dict[str, object]],
    list[dict[str, object]],
    list[dict[str, object]],
    list[dict[str, object]],
]:
    inventory = [
        {
            "suite": "library",
            "sourceFile": "tests/library/public.spec.ts",
            "projectId": "chromium-library",
            "specId": "public-id",
            "sourceClassification": "public-api-candidate",
            "line": 10,
            "titlePath": ["public test"],
        },
        {
            "suite": "library",
            "sourceFile": "tests/library/private.spec.ts",
            "projectId": "chromium-library",
            "specId": "private-id",
            "sourceClassification": "implementation-internal",
            "line": 20,
            "titlePath": ["private test"],
        },
    ]
    reviewed = [
        {
            **row,
            "sourceStartLine": row["line"],
            "sourceEndLine": row["line"],
            "invocationClassification": classification,
            "invocationClassificationReasons": reasons,
        }
        for row, classification, reasons in zip(
            inventory,
            ("public-api-candidate", "implementation-internal"),
            ([], ["private-client-access"]),
            strict=True,
        )
    ]
    for row in reviewed:
        del row["line"]
    candidate = [
        {
            **row,
            "outcome": "expected",
            "evidenceScope": SOURCE_ONLY,
            "candidateRuntimeRequired": False,
        }
        for row in inventory
    ]
    oracle = deepcopy(candidate)
    return inventory, reviewed, candidate, oracle


class ReconciliationTests(unittest.TestCase):
    def test_public_source_only_is_unproven_in_both_lanes(self) -> None:
        # Arrange
        inventory, reviewed, candidate, oracle = _rows()

        # Act
        report = reconcile(inventory, reviewed, candidate, oracle, expected_records=2)

        # Assert
        self.assertEqual(report["recordsPerLane"], 2)
        self.assertEqual(report["reviewedPublicSourceOnlyCountPerLane"], 1)
        for lane in ("candidate", "oracle"):
            self.assertEqual(
                report["sourceOnlyByReviewedClass"][lane],
                {"implementation-internal": 1, "public-api-candidate": 1},
            )
            self.assertEqual(
                report["reviewedPublicSourceOnly"][lane][0]["runtimeAssessment"],
                "runtime-unproven",
            )
            self.assertEqual(
                report["reviewedPublicSourceOnly"][lane][0]["applicabilityAssessment"],
                "public-behavior-still-applicable",
            )
            self.assertEqual(sum(item["count"] for item in report["crossTab"][lane]), 2)

    def test_order_and_identity_mismatch_fail_closed(self) -> None:
        # Arrange
        inventory, reviewed, candidate, oracle = _rows()
        candidate.reverse()

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "identity/order mismatch"):
            reconcile(inventory, reviewed, candidate, oracle, expected_records=2)

    def test_missing_row_fails_closed(self) -> None:
        # Arrange
        inventory, reviewed, candidate, oracle = _rows()
        oracle.pop()

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "row-count mismatch"):
            reconcile(inventory, reviewed, candidate, oracle, expected_records=2)

    def test_duplicate_identity_fails_closed(self) -> None:
        # Arrange
        inventory, reviewed, candidate, oracle = _rows()
        for rows in (inventory, reviewed, candidate, oracle):
            rows.append(deepcopy(rows[0]))

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "duplicate inventory invocation"):
            reconcile(inventory, reviewed, candidate, oracle, expected_records=3)

    def test_source_only_cannot_claim_runtime_or_nonexpected_result(self) -> None:
        # Arrange
        inventory, reviewed, candidate, oracle = _rows()
        candidate[0]["candidateRuntimeRequired"] = True

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "claims candidate runtime"):
            reconcile(inventory, reviewed, candidate, oracle, expected_records=2)

        candidate[0]["candidateRuntimeRequired"] = False
        candidate[0]["outcome"] = "unexpected"
        with self.assertRaisesRegex(ValueError, "source-only row is not expected"):
            reconcile(inventory, reviewed, candidate, oracle, expected_records=2)

    def test_both_lanes_must_flag_same_public_source_only_ids(self) -> None:
        # Arrange
        inventory, reviewed, candidate, oracle = _rows()
        del oracle[0]["evidenceScope"]

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "ID sets differ"):
            reconcile(inventory, reviewed, candidate, oracle, expected_records=2)

    def test_source_line_mismatch_fails_closed(self) -> None:
        # Arrange
        inventory, reviewed, candidate, oracle = _rows()
        reviewed[0]["sourceStartLine"] = 11

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "source line mismatch"):
            reconcile(inventory, reviewed, candidate, oracle, expected_records=2)

    def test_pinned_input_sha_mismatch_fails_closed(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_text(json.dumps({"records": 2}))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
                _verify_pinned_file(path, "0" * 64)


if __name__ == "__main__":
    unittest.main()
