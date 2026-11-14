#!/usr/bin/env python3
"""Tests for Playwright Python fixture-definition inventory."""

from __future__ import annotations

import unittest
from collections.abc import Sequence
from pathlib import Path

from inventory_playwright_python_official_fixture_definitions import (
    BindingRecord,
    DefinitionRecord,
    _applicability_outcome_counts,
    _definition_payload,
    _definition_record,
    _fixture_runtime_band,
    _fixture_server_band,
    _normalized_origin,
    _ordered_bindings,
    _request_definition,
    _semantic_role,
    _unresolved_definition,
    _verified_definitions,
)


def _fixture_function(request: object) -> object:
    return request


class _Definition:
    argname: str = "page"
    baseid: str = "tests/sync"
    func = _fixture_function
    scope: str = "function"
    argnames: tuple[str, ...] = ("request",)
    params: Sequence[object] | None = None
    _autouse = False


class OriginTests(unittest.TestCase):
    def test_upstream_source_is_normalized_relative_to_pin(self) -> None:
        # Arrange
        upstream = Path("/checkout")

        # Act
        origin = _normalized_origin(
            Path("/checkout/tests/conftest.py"),
            upstream,
            Path("/venv"),
            Path("/python"),
        )

        # Assert
        self.assertEqual(origin, ("upstream", "tests/conftest.py"))

    def test_installed_source_is_normalized_relative_to_environment(self) -> None:
        # Arrange
        source = Path("/venv/lib/python3.12/site-packages/_pytest/tmpdir.py")

        # Act
        origin = _normalized_origin(
            source, Path("/checkout"), Path("/venv"), Path("/python")
        )

        # Assert
        self.assertEqual(
            origin,
            ("environment", "lib/python3.12/site-packages/_pytest/tmpdir.py"),
        )


class SemanticRoleTests(unittest.TestCase):
    def test_direct_parameter_definition_uses_parameter_role(self) -> None:
        # Arrange
        module = "_pytest.python"

        # Act
        role = _semantic_role("method", module, "get_direct_param_fixture_func")

        # Assert
        self.assertEqual(role, "parameter-value")

    def test_unknown_fixture_role_fails_closed(self) -> None:
        # Arrange
        fixture_name = "unreviewed_fixture"

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "lacks an explicit semantic role"):
            _semantic_role(fixture_name, "tests.conftest", fixture_name)

    def test_external_plugin_fixture_has_explicit_role(self) -> None:
        # Arrange
        fixture_name = "_artifacts_recorder"

        # Act
        role = _semantic_role(
            fixture_name, "pytest_playwright.pytest_playwright", fixture_name
        )

        # Assert
        self.assertEqual(role, "artifact-lifecycle")


class DefinitionTests(unittest.TestCase):
    def test_definition_records_source_scope_dependencies_and_role(self) -> None:
        # Arrange
        definition = _Definition()
        source = Path(__file__).resolve()
        upstream = source.parents[1]

        # Act
        payload = _definition_payload(
            "page", definition, upstream, Path("/venv"), Path("/python")
        )

        # Assert
        self.assertEqual(payload["scope"], "function")
        self.assertEqual(payload["dependencies"], ["request"])
        self.assertEqual(payload["originKind"], "upstream")
        self.assertEqual(payload["semanticRole"], "browser-object-lifecycle")

    def test_definition_id_is_content_addressed(self) -> None:
        # Arrange
        payload: dict[str, object] = {
            "fixtureName": "request",
            "scope": "function",
        }

        # Act
        first = _definition_record(payload)
        second = _definition_record(payload)

        # Assert
        self.assertEqual(first["id"], second["id"])
        self.assertTrue(first["id"].startswith("fixturedef:"))

    def test_request_has_an_explicit_synthetic_definition(self) -> None:
        # Arrange
        fixture_name = "request"

        # Act
        definition = _request_definition()

        # Assert
        self.assertEqual(definition["fixtureName"], fixture_name)
        self.assertEqual(definition["originKind"], "pytest-builtin")
        self.assertEqual(definition["semanticRole"], "pytest-runtime")

    def test_missing_plugin_fixture_is_recorded_as_unresolved(self) -> None:
        # Arrange
        fixture_name = "page"

        # Act
        definition = _unresolved_definition(fixture_name)

        # Assert
        self.assertEqual(definition["fixtureName"], fixture_name)
        self.assertEqual(definition["scope"], "unknown")
        self.assertEqual(definition["originKind"], "unresolved")


class RuntimeBandTests(unittest.TestCase):
    def test_fixture_runtime_band_reports_strongest_automatic_owner(self) -> None:
        # Arrange
        fixture_names = ["playwright", "browser_type", "browser", "context"]

        # Act
        band = _fixture_runtime_band(fixture_names)

        # Assert
        self.assertEqual(band, "fixture-creates-context")

    def test_unresolved_definition_has_its_own_runtime_band(self) -> None:
        # Arrange
        fixture_names = ["page"]

        # Act
        band = _fixture_runtime_band(fixture_names, ["page"])

        # Assert
        self.assertEqual(band, "unresolved-fixture-definition")

    def test_fixture_server_band_distinguishes_explicit_consumers(self) -> None:
        # Arrange
        fixture_names = ["start_server", "server"]

        # Act
        band = _fixture_server_band(fixture_names)

        # Assert
        self.assertEqual(band, "autouse-plus-explicit-server")

    def test_runtime_bands_cross_tabulate_applicability_and_outcome(self) -> None:
        # Arrange
        binding = BindingRecord(
            id="tests/test_a.py::test_a",
            fixtureRuntimeBand="fixture-creates-page",
            fixtureServerBand="autouse-server-only",
            fixtureBindings=[],
        )
        audit = [{"id": binding["id"], "applicability": "applicable"}]
        outcomes = [{"id": binding["id"], "outcome": "not-run"}]

        # Act
        counts = _applicability_outcome_counts(
            [binding], audit, outcomes, "fixtureRuntimeBand"
        )

        # Assert
        self.assertEqual(
            counts,
            {"fixture-creates-page": {"applicable": {"not-run": 1}}},
        )


class LedgerVerificationTests(unittest.TestCase):
    def test_binding_order_must_match_closure_names(self) -> None:
        # Arrange
        closure = [{"id": "tests/test_a.py::test_a", "fixtureClosure": ["page"]}]
        raw = [
            {
                "id": "tests/test_a.py::test_a",
                "fixtureRuntimeBand": "no-playwright-runtime-fixture",
                "fixtureServerBand": "no-server-fixture",
                "fixtureBindings": [],
            }
        ]

        # Act and Assert
        with self.assertRaisesRegex(SystemExit, "closure differs"):
            _ordered_bindings(closure, raw)

    def test_selected_definition_dependencies_must_exist_in_closure(self) -> None:
        # Arrange
        request = _request_definition()
        page = DefinitionRecord(
            id="fixturedef:page",
            fixtureName="page",
            scope="function",
            baseId="tests",
            originKind="upstream",
            originPath="tests/conftest.py",
            module="tests.conftest",
            line=1,
            function="page",
            dependencies=["context"],
            autouse=False,
            parameterized=False,
            parameterCount=0,
            semanticRole="browser-object-lifecycle",
        )
        binding = BindingRecord(
            id="tests/test_a.py::test_a",
            fixtureRuntimeBand="fixture-creates-page",
            fixtureServerBand="no-server-fixture",
            fixtureBindings=[
                {
                    "name": "page",
                    "definitionIds": [page["id"]],
                    "selectedDefinitionId": page["id"],
                },
                {
                    "name": "request",
                    "definitionIds": [request["id"]],
                    "selectedDefinitionId": request["id"],
                },
            ],
        )

        # Act and Assert
        with self.assertRaisesRegex(SystemExit, "dependencies are absent"):
            _verified_definitions([page, request], [binding])


if __name__ == "__main__":
    unittest.main()
