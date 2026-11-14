#!/usr/bin/env python3
"""Tests for Playwright Python pytest fixture inventory."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from inventory_playwright_python_official_fixtures import (
    FixtureRecord,
    _fixture_record,
    _inventory_profiles,
    _ordered_fixture_records,
    _verify_direct_inputs,
)


class _FixtureInfo:
    def __init__(
        self,
        *,
        argnames: tuple[str, ...],
        initialnames: tuple[str, ...],
        names_closure: list[str],
    ) -> None:
        self.argnames = argnames
        self.initialnames = initialnames
        self.names_closure = names_closure


class _Item:
    def __init__(self) -> None:
        self.nodeid = "tests/sync/test_page.py::test_title[chromium]"
        self.fixturenames = ["page", "context", "browser", "start_server"]
        self._fixtureinfo = _FixtureInfo(
            argnames=("page",),
            initialnames=("page", "start_server"),
            names_closure=list(self.fixturenames),
        )


class FixtureRecordTests(unittest.TestCase):
    def test_fixture_record_preserves_all_three_dependency_layers(self) -> None:
        # Arrange
        item = _Item()

        # Act
        record = _fixture_record(item)

        # Assert
        self.assertEqual(record["directFixtureNames"], ["page"])
        self.assertEqual(record["initialFixtureNames"], ["page", "start_server"])
        self.assertEqual(
            record["fixtureClosure"],
            ["page", "context", "browser", "start_server"],
        )

    def test_fixture_record_rejects_divergent_public_closure(self) -> None:
        # Arrange
        item = _Item()
        item.fixturenames = ["page"]

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "closure views differ"):
            _fixture_record(item)


class FixtureJoinTests(unittest.TestCase):
    def test_fixture_records_follow_canonical_inventory_order(self) -> None:
        # Arrange
        inventory: list[dict[str, object]] = [
            {"id": "tests/test_a.py::test_a"},
            {"id": "tests/test_b.py::test_b"},
        ]
        raw: list[dict[str, object]] = [
            {
                "id": "tests/test_b.py::test_b",
                "directFixtureNames": [],
                "initialFixtureNames": ["autouse"],
                "fixtureClosure": ["autouse"],
            },
            {
                "id": "tests/test_a.py::test_a",
                "directFixtureNames": [],
                "initialFixtureNames": ["autouse"],
                "fixtureClosure": ["autouse"],
            },
        ]

        # Act
        ordered = _ordered_fixture_records(inventory, raw)

        # Assert
        self.assertEqual(
            [record["id"] for record in ordered],
            ["tests/test_a.py::test_a", "tests/test_b.py::test_b"],
        )

    def test_fixture_records_reject_incomplete_collection(self) -> None:
        # Arrange
        inventory: list[dict[str, object]] = [{"id": "tests/test_a.py::test_a"}]

        # Act and Assert
        with self.assertRaisesRegex(SystemExit, "differs from canonical inventory"):
            _ordered_fixture_records(inventory, [])

    def test_source_parameters_match_pytest_direct_inputs(self) -> None:
        # Arrange
        audit = [
            {
                "id": "tests/test_a.py::test_a",
                "pytestParameters": ["page", "server"],
            }
        ]
        fixtures: list[FixtureRecord] = [
            {
                "id": "tests/test_a.py::test_a",
                "directFixtureNames": ["page", "server"],
                "initialFixtureNames": ["page", "server", "autouse"],
                "fixtureClosure": ["page", "server", "autouse"],
            }
        ]

        # Act
        result = _verify_direct_inputs(audit, fixtures)

        # Assert
        self.assertIsNone(result)

    def test_source_parameter_drift_is_rejected(self) -> None:
        # Arrange
        audit = [
            {
                "id": "tests/test_a.py::test_a",
                "pytestParameters": ["page"],
            }
        ]
        fixtures: list[FixtureRecord] = [
            {
                "id": "tests/test_a.py::test_a",
                "directFixtureNames": ["context"],
                "initialFixtureNames": ["context"],
                "fixtureClosure": ["context"],
            }
        ]

        # Act and Assert
        with self.assertRaisesRegex(SystemExit, "direct inputs differ"):
            _verify_direct_inputs(audit, fixtures)


class CollectionProfileTests(unittest.TestCase):
    def test_inventory_profiles_preserve_core_then_examples(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            path = Path(directory) / "inventory.jsonl"
            path.write_text("")
            path.with_suffix(".summary.json").write_text(
                json.dumps(
                    {
                        "collectionProfiles": [
                            {
                                "name": "core",
                                "selection": "tests",
                                "expandedTests": 6501,
                            },
                            {
                                "name": "examples",
                                "selection": "examples/todomvc",
                                "expandedTests": 25,
                            },
                        ]
                    }
                )
            )

            # Act
            profiles = _inventory_profiles(path)

            # Assert
            self.assertEqual(
                [profile["name"] for profile in profiles], ["core", "examples"]
            )


if __name__ == "__main__":
    unittest.main()
