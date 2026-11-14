#!/usr/bin/env python3
"""Tests for per-invocation Playwright Python classification."""

from __future__ import annotations

import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from audit_playwright_python_official_invocations import (
    ApplicabilityRule,
    Dependency,
    ModuleAnalysis,
    _pytest_parameters,  # pyright: ignore[reportPrivateUsage]
    _reviewed_classification,  # pyright: ignore[reportPrivateUsage]
)


class ModuleAnalysisTests(unittest.TestCase):
    def _analyze(self, source: str, node_id: str) -> tuple[str, list[Dependency]]:
        with TemporaryDirectory() as directory:
            path = Path(directory) / "test_sample.py"
            path.write_text(source)

            # Act
            _, classification, dependencies = ModuleAnalysis(path).invocation(node_id)

            return classification, list(dependencies)

    def test_private_type_annotation_does_not_make_behavior_internal(self) -> None:
        # Arrange
        source = (
            "from playwright._impl._page import Page\n"
            "async def test_public(page: Page):\n"
            "    return await page.title()\n"
        )

        # Act
        classification, dependencies = self._analyze(
            source, "test_sample.py::test_public[chromium]"
        )

        # Assert
        self.assertEqual(classification, "public-api-with-private-type-annotation")
        self.assertEqual(
            [(item.symbol, item.usage) for item in dependencies],
            [("playwright._impl._page.Page", "type-annotation")],
        )

    def test_direct_private_function_call_is_internal(self) -> None:
        # Arrange
        source = (
            "from playwright._impl._helper import url_matches\n"
            "def test_private():\n"
            "    assert url_matches(None, 'https://example.test', '**')\n"
        )

        # Act
        classification, dependencies = self._analyze(source, "test_sample.py::test_private")

        # Assert
        self.assertEqual(classification, "implementation-internal")
        self.assertEqual(dependencies[0].usage, "private-runtime")

    def test_transitive_module_assignment_is_a_private_test_helper(self) -> None:
        # Arrange
        source = (
            "from playwright._impl._path_utils import get_file_dirname\n"
            "ROOT = get_file_dirname()\n"
            "ASSET = ROOT / 'asset.txt'\n"
            "def test_public():\n"
            "    assert ASSET.name == 'asset.txt'\n"
        )

        # Act
        classification, dependencies = self._analyze(source, "test_sample.py::test_public")

        # Assert
        self.assertEqual(classification, "public-api-with-private-test-helper")
        self.assertEqual(
            [(item.symbol, item.usage) for item in dependencies],
            [
                (
                    "playwright._impl._path_utils.get_file_dirname",
                    "private-test-helper",
                )
            ],
        )

    def test_unused_private_module_import_keeps_invocation_public(self) -> None:
        # Arrange
        source = (
            "from playwright._impl._page import Page\ndef test_public():\n    assert 1 + 1 == 2\n"
        )

        # Act
        classification, dependencies = self._analyze(source, "test_sample.py::test_public")

        # Assert
        self.assertEqual(classification, "public-api-candidate")
        self.assertEqual(dependencies, [])

    def test_pytest_parameters_preserve_declared_order(self) -> None:
        # Arrange
        source = (
            "async def test_public(page, value, *, server, option=None):\n"
            "    return page, value, server, option\n"
        )
        node_id = "test_sample.py::test_public[chromium-value]"
        with TemporaryDirectory() as directory:
            path = Path(directory) / "test_sample.py"
            path.write_text(source)
            node, _, _ = ModuleAnalysis(path).invocation(node_id)

            # Act
            parameters = _pytest_parameters(node_id, node)

            # Assert
            self.assertEqual(parameters, ["page", "value", "server", "option"])

    def test_pytest_parameters_exclude_method_receiver(self) -> None:
        # Arrange
        source = (
            "class TestPublic:\n"
            "    def test_method(self, context, *values, **options):\n"
            "        return context, values, options\n"
        )
        node_id = "test_sample.py::TestPublic::test_method[webkit]"
        with TemporaryDirectory() as directory:
            path = Path(directory) / "test_sample.py"
            path.write_text(source)
            node, _, _ = ModuleAnalysis(path).invocation(node_id)

            # Act
            parameters = _pytest_parameters(node_id, node)

            # Assert
            self.assertEqual(parameters, ["context", "*values", "**options"])


class ApplicabilityReviewTests(unittest.TestCase):
    def _analyze(self, source: str, node_id: str) -> tuple[str, list[Dependency]]:
        with TemporaryDirectory() as directory:
            path = Path(directory) / "test_sample.py"
            path.write_text(source)

            # Act
            _, classification, dependencies = ModuleAnalysis(path).invocation(node_id)

            return classification, list(dependencies)

    def test_direct_private_dependency_requires_review(self) -> None:
        # Arrange
        policy: dict[str, ApplicabilityRule] = {}

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "lacks an applicability rule"):
            _reviewed_classification(
                "tests/test_sample.py::test_private[chromium]",
                "implementation-internal",
                policy,
            )

    def test_private_assertion_oracle_remains_applicable(self) -> None:
        # Arrange
        policy: dict[str, ApplicabilityRule] = {
            "tests/test_sample.py::test_repr": {
                "id": "tests/test_sample.py::test_repr",
                "applicability": "applicable",
                "classification": "public-api-with-private-assertion-oracle",
                "reason": "private value only constructs the public expectation",
            }
        }

        # Act
        classification, applicability, reason = _reviewed_classification(
            "tests/test_sample.py::test_repr[webkit]",
            "implementation-internal",
            policy,
        )

        # Assert
        self.assertEqual(classification, "public-api-with-private-assertion-oracle")
        self.assertEqual(applicability, "applicable")
        self.assertIsNotNone(reason)

    def test_policy_cannot_hide_a_now_public_invocation(self) -> None:
        # Arrange
        policy: dict[str, ApplicabilityRule] = {
            "tests/test_sample.py::test_public": {
                "id": "tests/test_sample.py::test_public",
                "applicability": "non-applicable",
                "classification": "implementation-internal",
                "reason": "stale rule",
            }
        }

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "no longer points"):
            _reviewed_classification(
                "tests/test_sample.py::test_public[firefox]",
                "public-api-candidate",
                policy,
            )

    def test_internal_selector_engine_is_internal_without_private_import(self) -> None:
        # Arrange
        source = "def test_internal(page):\n    page.locator('div >> internal:and=.foo')\n"

        # Act
        classification, dependencies = self._analyze(source, "test_sample.py::test_internal")

        # Assert
        self.assertEqual(classification, "implementation-internal")
        self.assertEqual(dependencies[0].symbol, "selector-engine:internal:and")

    def test_external_private_attribute_is_internal(self) -> None:
        # Arrange
        source = "def test_internal(browser):\n    assert browser._impl_obj._browser_type\n"

        # Act
        classification, dependencies = self._analyze(source, "test_sample.py::test_internal")

        # Assert
        self.assertEqual(classification, "implementation-internal")
        self.assertEqual(
            {item.symbol for item in dependencies},
            {"attribute:_browser_type", "attribute:_impl_obj"},
        )

    def test_locally_defined_private_method_is_not_playwright_internal(self) -> None:
        # Arrange
        source = (
            "class Resource:\n"
            "    def _verify(self):\n"
            "        return True\n"
            "    def test_public(self):\n"
            "        assert self._verify()\n"
        )

        # Act
        classification, dependencies = self._analyze(
            source, "test_sample.py::Resource::test_public[webkit]"
        )

        # Assert
        self.assertEqual(classification, "public-api-candidate")
        self.assertEqual(dependencies, [])


if __name__ == "__main__":
    unittest.main()
