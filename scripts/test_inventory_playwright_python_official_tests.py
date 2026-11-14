#!/usr/bin/env python3
"""Tests for profile-aware Playwright Python inventory."""

from __future__ import annotations

import unittest

from inventory_playwright_python_official_tests import (
    CollectionProfile,
    Manifest,
    _collection_node_ids,
    _verify_profile_runtime,
)


class CollectionOutputTests(unittest.TestCase):
    def test_collection_parser_keeps_only_requested_profile(self) -> None:
        # Arrange
        output = (
            "tests/test_a.py::test_a[chromium]\n"
            "examples/todomvc/test_b.py::test_b[chromium]\n"
            "2 tests collected in 0.01s"
        )

        # Act
        node_ids = _collection_node_ids(output, "tests/")

        # Assert
        self.assertEqual(node_ids, ["tests/test_a.py::test_a[chromium]"])

    def test_collection_parser_rejects_duplicate_ids(self) -> None:
        # Arrange
        output = "tests/test_a.py::test_a\ntests/test_a.py::test_a\n"

        # Act and Assert
        with self.assertRaisesRegex(SystemExit, "duplicate node IDs"):
            _collection_node_ids(output, "tests/")


class RuntimeVerificationTests(unittest.TestCase):
    def test_examples_runtime_must_match_all_pins(self) -> None:
        # Arrange
        profile: CollectionProfile = {
            "name": "examples",
            "selection": "examples/todomvc",
            "expandedTests": 25,
            "pythonVersion": "3.10",
            "pytestVersion": "8.4.1",
            "pytestPlaywrightVersion": "0.7.2",
        }
        manifest: Manifest = {
            "schema": 1,
            "repository": "https://example.test/playwright-python.git",
            "tag": "v1.58.0",
            "commit": "0123456789abcdef",
            "corpus": {
                "policy": "profiled",
                "testsTreeModules": 1,
                "testsTreeTopLevelTestFunctions": 1,
                "examplesTreeModules": 1,
                "expandedTests": 1,
                "collectionProfiles": [profile],
                "defaultBrowsers": ["chromium"],
                "ciPlatforms": ["darwin"],
                "ciPythonVersions": ["3.10"],
                "stableChannels": ["chrome"],
                "nonCollectedModules": [],
            },
        }
        identity = {
            "pythonVersion": "3.10.21",
            "playwrightVersion": "1.58.0",
            "pytestVersion": "8.4.1",
            "pytestPlaywrightVersion": "0.7.2",
        }

        # Act
        result = _verify_profile_runtime(profile, identity, manifest)

        # Assert
        self.assertIsNone(result)

    def test_examples_runtime_rejects_plugin_drift(self) -> None:
        # Arrange
        profile: CollectionProfile = {
            "name": "examples",
            "selection": "examples/todomvc",
            "expandedTests": 25,
            "pytestPlaywrightVersion": "0.7.2",
        }
        manifest: Manifest = {
            "schema": 1,
            "repository": "https://example.test/playwright-python.git",
            "tag": "v1.58.0",
            "commit": "0123456789abcdef",
            "corpus": {
                "policy": "profiled",
                "testsTreeModules": 1,
                "testsTreeTopLevelTestFunctions": 1,
                "examplesTreeModules": 1,
                "expandedTests": 1,
                "collectionProfiles": [profile],
                "defaultBrowsers": ["chromium"],
                "ciPlatforms": ["darwin"],
                "ciPythonVersions": ["3.10"],
                "stableChannels": ["chrome"],
                "nonCollectedModules": [],
            },
        }
        identity = {
            "pythonVersion": "3.10.21",
            "playwrightVersion": "1.58.0",
            "pytestVersion": "8.4.1",
            "pytestPlaywrightVersion": "0.9.0",
        }

        # Act and Assert
        with self.assertRaisesRegex(SystemExit, "differs from profile"):
            _verify_profile_runtime(profile, identity, manifest)


if __name__ == "__main__":
    unittest.main()
