"""Fail-closed tests for source-pinned browserless official selections."""

from __future__ import annotations

import ast
import copy
import unittest

from check_playwright_python_official_source import (
    PINNED_BROWSER_TYPE_EXECUTABLE_PATH_ARGUMENTS,
    PINNED_BROWSER_TYPE_EXECUTABLE_PATH_SOURCES,
    PINNED_BROWSER_TYPE_NAME_ARGUMENTS,
    PINNED_BROWSER_TYPE_NAME_SOURCES,
    PINNED_PERSISTENT_PAGE_ARGUMENT_ARGUMENTS,
    PINNED_PERSISTENT_PAGE_ARGUMENT_CAPABILITIES,
    PINNED_PERSISTENT_PAGE_ARGUMENT_CASE_ID,
    PINNED_PERSISTENT_PAGE_ARGUMENT_SOURCES,
    PINNED_PROXY_BAD_SERVER_ARGUMENTS,
    PINNED_PROXY_BAD_SERVER_CAPABILITIES,
    PINNED_PROXY_BAD_SERVER_CASE_ID,
    PINNED_PROXY_BAD_SERVER_SOURCES,
    PERSISTENT_PAGE_ARGUMENT_MODE,
    PROXY_BAD_SERVER_MODE,
    _browserless_fixture_arguments,
    _validate_persistent_page_argument_manifest,
    _validate_proxy_bad_server_manifest,
    _verify_browserless_function,
)


def _functions(source: str) -> dict[str, ast.FunctionDef | ast.AsyncFunctionDef]:
    tree = ast.parse(source)
    return {
        node.name: node
        for node in tree.body
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
    }


PERSISTENT_PAGE_ARGUMENT_FUNCTION = """
@pytest.mark.skip_browser("firefox")
async def test_should_throw_if_page_argument_is_passed(
    browser_type: BrowserType, server: Server, tmp_path: Path, launch_arguments: Dict
) -> None:
    options = {**launch_arguments, "args": [server.EMPTY_PAGE]}
    with pytest.raises(Error) as exc:
        await browser_type.launch_persistent_context(tmp_path, **options)
    assert "can not specify page" in exc.value.message
"""
PROXY_BAD_SERVER_FUNCTION = """
async def test_should_throw_for_bad_server_value(
    browser_factory: "Callable[..., asyncio.Future[Browser]]",
) -> None:
    with pytest.raises(Error) as exc_info:
        await browser_factory(proxy={"server": 123})
    assert "proxy.server: expected string, got number" in exc_info.value.message
"""


def _proxy_manifest() -> dict[str, object]:
    return {
        "repository": "https://github.com/microsoft/playwright-python.git",
        "tag": "v1.58.0",
        "commit": "47a5d35ef4f815a2021349f86ae391f7c20c02d6",
        "executionMode": PROXY_BAD_SERVER_MODE,
        "execution": {"requiresBrowser": False, "requiresFixtureServers": True},
        "files": PINNED_PROXY_BAD_SERVER_SOURCES.copy(),
        "cases": [
            {
                "id": PINNED_PROXY_BAD_SERVER_CASE_ID,
                "capabilities": PINNED_PROXY_BAD_SERVER_CAPABILITIES.copy(),
            }
        ],
    }


def _persistent_manifest() -> dict[str, object]:
    return {
        "repository": "https://github.com/microsoft/playwright-python.git",
        "tag": "v1.58.0",
        "commit": "47a5d35ef4f815a2021349f86ae391f7c20c02d6",
        "executionMode": PERSISTENT_PAGE_ARGUMENT_MODE,
        "execution": {"requiresBrowser": False, "requiresFixtureServers": True},
        "files": PINNED_PERSISTENT_PAGE_ARGUMENT_SOURCES.copy(),
        "cases": [
            {
                "id": PINNED_PERSISTENT_PAGE_ARGUMENT_CASE_ID,
                "capabilities": PINNED_PERSISTENT_PAGE_ARGUMENT_CAPABILITIES.copy(),
            }
        ],
    }


class BrowserlessOfficialSourceTests(unittest.TestCase):
    def test_exact_proxy_bad_server_preflight_is_allowed(self) -> None:
        # Arrange
        relative, function = PINNED_PROXY_BAD_SERVER_CASE_ID.split("::", 1)
        functions = _functions(PROXY_BAD_SERVER_FUNCTION)

        # Act
        allowed = _browserless_fixture_arguments(
            relative, function, PINNED_PROXY_BAD_SERVER_SOURCES
        )
        _verify_browserless_function(relative, function, functions, allowed)
        _validate_proxy_bad_server_manifest(_proxy_manifest())

        # Assert
        self.assertEqual(allowed, PINNED_PROXY_BAD_SERVER_ARGUMENTS)

    def test_proxy_bad_server_rejects_source_or_policy_drift(self) -> None:
        # Arrange
        relative, function = PINNED_PROXY_BAD_SERVER_CASE_ID.split("::", 1)

        # Act and Assert
        for path in PINNED_PROXY_BAD_SERVER_SOURCES:
            with self.subTest(path=path):
                hashes = dict(PINNED_PROXY_BAD_SERVER_SOURCES)
                hashes[path] = "0" * 64
                with self.assertRaisesRegex(SystemExit, "fixture source mismatch"):
                    _browserless_fixture_arguments(relative, function, hashes)
        for change in (
            {"executionMode": "ordinary"},
            {"execution": {"requiresBrowser": True, "requiresFixtureServers": True}},
            {"cases": []},
            {"files": {"tests/async/test_proxy.py": "0" * 64}},
        ):
            with self.subTest(change=change):
                manifest = _proxy_manifest()
                manifest.update(copy.deepcopy(change))
                with self.assertRaises(SystemExit):
                    _validate_proxy_bad_server_manifest(manifest)

    def test_proxy_bad_server_rejects_extra_browser_action_or_fixture(self) -> None:
        # Arrange
        relative, function = PINNED_PROXY_BAD_SERVER_CASE_ID.split("::", 1)
        allowed = _browserless_fixture_arguments(
            relative, function, PINNED_PROXY_BAD_SERVER_SOURCES
        )
        altered = (
            PROXY_BAD_SERVER_FUNCTION.replace(
                'await browser_factory(proxy={"server": 123})',
                'await browser_factory(proxy={"server": "valid"})',
            ),
            PROXY_BAD_SERVER_FUNCTION.replace(
                'await browser_factory(proxy={"server": 123})',
                'await browser_factory(proxy={"server": 123})\n'
                "        await browser_type.launch()",
            ),
            PROXY_BAD_SERVER_FUNCTION.replace(
                'browser_factory: "Callable[..., asyncio.Future[Browser]]",',
                'browser_factory: "Callable[..., asyncio.Future[Browser]]", page: Page,',
            ),
        )

        # Act and Assert
        for source in altered:
            with self.subTest(source=source), self.assertRaises(SystemExit):
                _verify_browserless_function(
                    relative, function, _functions(source), allowed
                )

    def test_exact_persistent_page_argument_preflight_is_allowed(self) -> None:
        # Arrange
        relative, function = PINNED_PERSISTENT_PAGE_ARGUMENT_CASE_ID.split("::", 1)
        functions = _functions(PERSISTENT_PAGE_ARGUMENT_FUNCTION)

        # Act
        allowed = _browserless_fixture_arguments(
            relative, function, PINNED_PERSISTENT_PAGE_ARGUMENT_SOURCES
        )
        _verify_browserless_function(relative, function, functions, allowed)
        _validate_persistent_page_argument_manifest(_persistent_manifest())

        # Assert
        self.assertEqual(allowed, PINNED_PERSISTENT_PAGE_ARGUMENT_ARGUMENTS)

    def test_persistent_preflight_rejects_changed_fixture_source_or_selection(
        self,
    ) -> None:
        # Arrange, Act, Assert
        relative, function = PINNED_PERSISTENT_PAGE_ARGUMENT_CASE_ID.split("::", 1)
        for path in PINNED_PERSISTENT_PAGE_ARGUMENT_SOURCES:
            with self.subTest(path=path):
                hashes = dict(PINNED_PERSISTENT_PAGE_ARGUMENT_SOURCES)
                hashes[path] = "0" * 64
                with self.assertRaisesRegex(SystemExit, "fixture source mismatch"):
                    _browserless_fixture_arguments(relative, function, hashes)
        for change in (
            {"executionMode": "ordinary"},
            {"execution": {"requiresBrowser": True, "requiresFixtureServers": True}},
            {"cases": []},
            {"files": {"tests/async/test_defaultbrowsercontext.py": "0" * 64}},
        ):
            with self.subTest(change=change):
                manifest = _persistent_manifest()
                manifest.update(copy.deepcopy(change))
                with self.assertRaises(SystemExit):
                    _validate_persistent_page_argument_manifest(manifest)

    def test_persistent_preflight_rejects_changed_ast_or_fixture_arguments(
        self,
    ) -> None:
        # Arrange
        relative, function = PINNED_PERSISTENT_PAGE_ARGUMENT_CASE_ID.split("::", 1)
        allowed = _browserless_fixture_arguments(
            relative, function, PINNED_PERSISTENT_PAGE_ARGUMENT_SOURCES
        )
        altered = (
            PERSISTENT_PAGE_ARGUMENT_FUNCTION.replace(
                'pytest.mark.skip_browser("firefox")',
                'pytest.mark.skip_browser("webkit")',
            ),
            PERSISTENT_PAGE_ARGUMENT_FUNCTION.replace(
                "await browser_type.launch_persistent_context(tmp_path, **options)",
                "await browser_type.launch_persistent_context(tmp_path, **options)\n"
                "        await browser_type.launch()",
            ),
            PERSISTENT_PAGE_ARGUMENT_FUNCTION.replace(
                "browser_type: BrowserType, server: Server, tmp_path: Path, launch_arguments: Dict",
                "browser_type: BrowserType, server: Server, tmp_path: Path, launch_arguments: Dict, page: Page",
            ),
        )

        # Act and Assert
        for source in altered:
            with self.subTest(source=source), self.assertRaises(SystemExit):
                _verify_browserless_function(
                    relative, function, _functions(source), allowed
                )

    def test_unpinned_persistent_launch_is_never_browserless(self) -> None:
        # Arrange
        relative = "tests/async/test_other.py"
        function = "test_unpinned"
        functions = _functions(
            f"async def {function}():\n"
            "    await browser_type.launch_persistent_context(tmp_path)\n"
        )

        # Act and Assert
        with self.assertRaisesRegex(SystemExit, "browser operation"):
            _verify_browserless_function(relative, function, functions)

    def test_exact_source_pinned_executable_path_cases_are_allowed(self) -> None:
        for case_id, hashes in PINNED_BROWSER_TYPE_EXECUTABLE_PATH_SOURCES.items():
            with self.subTest(case_id=case_id):
                # Arrange
                relative, function = case_id.split("::", maxsplit=1)
                arguments = ", ".join(PINNED_BROWSER_TYPE_EXECUTABLE_PATH_ARGUMENTS)
                functions = _functions(
                    f"def {function}({arguments}):\n"
                    "    return browser_type.executable_path\n"
                )

                # Act
                allowed = _browserless_fixture_arguments(relative, function, hashes)
                _verify_browserless_function(relative, function, functions, allowed)

                # Assert
                self.assertEqual(allowed, PINNED_BROWSER_TYPE_EXECUTABLE_PATH_ARGUMENTS)

    def test_executable_path_rejects_every_source_or_fixture_hash_change(self) -> None:
        for (
            case_id,
            expected_hashes,
        ) in PINNED_BROWSER_TYPE_EXECUTABLE_PATH_SOURCES.items():
            relative, function = case_id.split("::", maxsplit=1)
            for changed_path in expected_hashes:
                with self.subTest(case_id=case_id, changed_path=changed_path):
                    # Arrange
                    hashes = dict(expected_hashes)
                    hashes[changed_path] = "0" * 64

                    # Act and Assert
                    with self.assertRaisesRegex(SystemExit, "fixture source mismatch"):
                        _browserless_fixture_arguments(relative, function, hashes)

    def test_executable_path_rejects_added_or_reordered_fixture(self) -> None:
        # Arrange
        case_id = (
            "tests/sync/test_launcher.py::test_browser_type_executable_path_should_work"
        )
        relative, function = case_id.split("::", maxsplit=1)
        allowed = _browserless_fixture_arguments(
            relative, function, PINNED_BROWSER_TYPE_EXECUTABLE_PATH_SOURCES[case_id]
        )

        # Act and Assert
        for arguments in (
            "browser_type, browser_channel, page",
            "browser_channel, browser_type",
        ):
            with self.subTest(arguments=arguments):
                functions = _functions(
                    f"def {function}({arguments}):\n"
                    "    return browser_type.executable_path\n"
                )
                with self.assertRaisesRegex(SystemExit, "unapproved fixtures"):
                    _verify_browserless_function(relative, function, functions, allowed)

    def test_executable_path_rejects_browser_launch_in_body(self) -> None:
        # Arrange
        case_id = "tests/async/test_launcher.py::test_browser_type_executable_path_should_work"
        relative, function = case_id.split("::", maxsplit=1)
        allowed = _browserless_fixture_arguments(
            relative, function, PINNED_BROWSER_TYPE_EXECUTABLE_PATH_SOURCES[case_id]
        )
        functions = _functions(
            f"def {function}(browser_type, browser_channel):\n"
            "    return browser_type.launch()\n"
        )

        # Act and Assert
        with self.assertRaisesRegex(SystemExit, "browser operation"):
            _verify_browserless_function(relative, function, functions, allowed)

    def test_exact_source_pinned_browser_type_name_cases_are_allowed(self) -> None:
        for case_id, hashes in PINNED_BROWSER_TYPE_NAME_SOURCES.items():
            with self.subTest(case_id=case_id):
                # Arrange
                relative, function = case_id.split("::", maxsplit=1)
                arguments = ", ".join(PINNED_BROWSER_TYPE_NAME_ARGUMENTS)
                functions = _functions(
                    f"def {function}({arguments}):\n    return browser_type.name\n"
                )

                # Act
                allowed = _browserless_fixture_arguments(relative, function, hashes)
                _verify_browserless_function(relative, function, functions, allowed)

                # Assert
                self.assertEqual(allowed, PINNED_BROWSER_TYPE_NAME_ARGUMENTS)

    def test_other_fixture_bearing_case_remains_forbidden(self) -> None:
        # Arrange
        relative = "tests/async/test_other.py"
        function = "test_browser_type_name_should_work"
        functions = _functions(
            f"def {function}(browser_type):\n    return browser_type.name\n"
        )

        # Act
        allowed = _browserless_fixture_arguments(relative, function, {})

        # Assert
        self.assertEqual(allowed, ())
        with self.assertRaisesRegex(SystemExit, "unapproved fixtures"):
            _verify_browserless_function(relative, function, functions, allowed)

    def test_pinned_case_rejects_any_source_or_fixture_hash_change(self) -> None:
        for case_id, expected_hashes in PINNED_BROWSER_TYPE_NAME_SOURCES.items():
            relative, function = case_id.split("::", maxsplit=1)
            for changed_path in expected_hashes:
                with self.subTest(case_id=case_id, changed_path=changed_path):
                    # Arrange
                    hashes = dict(expected_hashes)
                    hashes[changed_path] = "0" * 64

                    # Act and Assert
                    with self.assertRaisesRegex(SystemExit, "fixture source mismatch"):
                        _browserless_fixture_arguments(relative, function, hashes)

    def test_pinned_case_rejects_extra_or_reordered_fixtures(self) -> None:
        # Arrange
        case_id = "tests/async/test_launcher.py::test_browser_type_name_should_work"
        relative, function = case_id.split("::", maxsplit=1)
        allowed = _browserless_fixture_arguments(
            relative, function, PINNED_BROWSER_TYPE_NAME_SOURCES[case_id]
        )

        # Act and Assert
        for arguments in (
            "browser_type, is_webkit, is_firefox, is_chromium, page",
            "is_webkit, browser_type, is_firefox, is_chromium",
        ):
            with self.subTest(arguments=arguments):
                functions = _functions(
                    f"def {function}({arguments}):\n    return browser_type.name\n"
                )
                with self.assertRaisesRegex(SystemExit, "unapproved fixtures"):
                    _verify_browserless_function(relative, function, functions, allowed)

    def test_pinned_case_rejects_browser_operation_in_body_or_helper(self) -> None:
        # Arrange
        case_id = "tests/sync/test_launcher.py::test_browser_type_name_should_work"
        relative, function = case_id.split("::", maxsplit=1)
        allowed = _browserless_fixture_arguments(
            relative, function, PINNED_BROWSER_TYPE_NAME_SOURCES[case_id]
        )
        arguments = ", ".join(allowed)
        for source in (
            f"def {function}({arguments}):\n    browser_type.launch()\n",
            f"def helper():\n    browser_type.launch()\ndef {function}({arguments}):\n    helper()\n",
        ):
            with (
                self.subTest(source=source),
                self.assertRaisesRegex(SystemExit, "browser operation"),
            ):
                # Act and Assert
                _verify_browserless_function(
                    relative, function, _functions(source), allowed
                )

    def test_zero_fixture_browserless_behavior_still_works(self) -> None:
        # Arrange
        relative = "tests/async/test_page_route.py"
        function = "test_should_work_with_glob"
        functions = _functions(f"def {function}():\n    return True\n")

        # Act
        allowed = _browserless_fixture_arguments(relative, function, {})
        _verify_browserless_function(relative, function, functions, allowed)

        # Assert
        self.assertEqual(allowed, ())


if __name__ == "__main__":
    unittest.main()
