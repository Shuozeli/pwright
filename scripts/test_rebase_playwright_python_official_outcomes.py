#!/usr/bin/env python3
"""Tests for rebasing Python outcomes onto the profiled inventory."""

from __future__ import annotations

import unittest

from rebase_playwright_python_official_outcomes import _rebase_records, _source_id


def _inventory(node_id: str) -> dict[str, object]:
    return {
        "id": node_id,
        "file": node_id.split("::", maxsplit=1)[0],
        "api": "example" if node_id.startswith("examples/") else "sync",
        "sourceClassification": "public-api-source",
        "privateImports": [],
    }


class SourceIdTests(unittest.TestCase):
    def test_example_chromium_suffix_maps_to_preprofile_id(self) -> None:
        # Arrange
        target = "examples/todomvc/test_a.py::test_a[chromium]"
        source = {"examples/todomvc/test_a.py::test_a"}

        # Act
        result = _source_id(target, source)

        # Assert
        self.assertEqual(result, ("examples/todomvc/test_a.py::test_a", True))

    def test_non_example_id_drift_is_rejected(self) -> None:
        # Arrange
        target = "tests/test_a.py::test_a[chromium]"

        # Act and Assert
        with self.assertRaisesRegex(SystemExit, "no exact source outcome"):
            _source_id(target, {"tests/test_a.py::test_a"})


class RebaseTests(unittest.TestCase):
    def test_pristine_example_not_run_outcome_is_rebased(self) -> None:
        # Arrange
        source_id = "examples/todomvc/test_a.py::test_a"
        target_id = f"{source_id}[chromium]"
        source_inventory = [_inventory(source_id)]
        target_inventory = [_inventory(target_id)]
        source_outcomes = [{**source_inventory[0], "outcome": "not-run", "attempts": 0}]

        # Act
        result = _rebase_records(source_inventory, target_inventory, source_outcomes)

        # Assert
        self.assertEqual(result["rebasedIds"], 1)
        self.assertEqual(result["records"][0]["id"], target_id)
        self.assertEqual(result["records"][0]["outcome"], "not-run")

    def test_executed_example_outcome_cannot_be_rebased(self) -> None:
        # Arrange
        source_id = "examples/todomvc/test_a.py::test_a"
        source_inventory = [_inventory(source_id)]
        target_inventory = [_inventory(f"{source_id}[chromium]")]
        source_outcomes = [{**source_inventory[0], "outcome": "passed", "attempts": 1}]

        # Act and Assert
        with self.assertRaisesRegex(SystemExit, "pristine not-run"):
            _rebase_records(source_inventory, target_inventory, source_outcomes)

    def test_exact_executed_outcome_is_preserved(self) -> None:
        # Arrange
        node_id = "tests/test_a.py::test_a[chromium]"
        inventory = [_inventory(node_id)]
        outcomes = [
            {
                **inventory[0],
                "outcome": "passed",
                "attempts": 1,
                "durationSeconds": 0.25,
                "resultFile": "/result.xml",
            }
        ]

        # Act
        result = _rebase_records(inventory, inventory, outcomes)

        # Assert
        self.assertEqual(result["rebasedIds"], 0)
        self.assertEqual(result["records"][0], outcomes[0])


if __name__ == "__main__":
    unittest.main()
