"""Browserless checks for the Python official owned-launch boundary."""

from __future__ import annotations

import asyncio
import importlib
import importlib.util
import os
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from types import ModuleType
from unittest.mock import patch
from xml.etree import ElementTree as ET

from run_playwright_python_official_conformance import (
    JUNIT_LAUNCH_MODE_PROPERTY,
    _configure_launch_environment,
    _label_junit_launch_mode,
    _prepare_isolated_launch_receipt,
    _require_owned_public_launch,
    _require_resumable_launch_mode,
    _select_launch_endpoint,
)

ROOT = Path(__file__).parents[1]
SHIM = ROOT / "packages/pwright-python/tests/official/shim/playwright"
ORACLE_RUNTIME = (
    ROOT
    / "packages/pwright-python/tests/official/runtime/pwright_official_oracle_runtime.py"
)


class PythonOfficialOwnedLaunchTests(unittest.TestCase):
    def test_adapter_default_and_owned_endpoint_selection(self) -> None:
        # Arrange
        inherited = {"PWRIGHT_CDP_ENDPOINT": "http://127.0.0.1:9333"}

        # Act
        adapter = _select_launch_endpoint("cdp-adapter", None)
        _configure_launch_environment(inherited, "owned", None)

        # Assert
        self.assertEqual(adapter, "http://127.0.0.1:9222")
        self.assertEqual(inherited["PWRIGHT_OFFICIAL_LAUNCH_MODE"], "owned")
        self.assertNotIn("PWRIGHT_CDP_ENDPOINT", inherited)
        with self.assertRaisesRegex(ValueError, "cannot be combined"):
            _select_launch_endpoint("owned", "http://127.0.0.1:9222")
        with self.assertRaisesRegex(ValueError, "must use http"):
            _select_launch_endpoint("cdp-adapter", "")

    def test_adapter_environment_keeps_explicit_loopback_endpoint(self) -> None:
        # Arrange
        environment: dict[str, str] = {}
        endpoint = _select_launch_endpoint("cdp-adapter", "http://127.0.0.1:9333")

        # Act
        _configure_launch_environment(environment, "cdp-adapter", endpoint)

        # Assert
        self.assertEqual(environment["PWRIGHT_CDP_ENDPOINT"], endpoint)
        self.assertEqual(environment["PWRIGHT_OFFICIAL_LAUNCH_MODE"], "cdp-adapter")

    def test_owned_bootstrap_is_rejected_before_browser_or_fixture_start(self) -> None:
        # Arrange
        runner = ROOT / "scripts/run_playwright_python_official_conformance.py"

        # Act
        result = subprocess.run(
            [
                sys.executable,
                str(runner),
                "candidate",
                "/missing-upstream",
                "--bootstrap-only",
                "--launch-mode=owned",
                "--server-prefix=http://fixture.tail8f3b66.ts.net:8000",
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )

        # Assert
        self.assertEqual(result.returncode, 2)
        self.assertIn("--bootstrap-only supports only cdp-adapter", result.stderr)

    def test_owned_preflight_rejects_missing_candidate_launch_without_browser(
        self,
    ) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "pwright"
            package.mkdir()
            (package / "__init__.py").write_text("")
            for name in ("async_api", "sync_api"):
                (package / f"{name}.py").write_text("class BrowserType:\n    pass\n")
            environment = os.environ.copy()
            environment["PYTHONPATH"] = str(root)

            # Act and Assert
            with self.assertRaisesRegex(
                ValueError, "BrowserType.launch is not a real public API"
            ):
                _require_owned_public_launch(
                    sys.executable, "candidate", cwd=root, env=environment
                )

    def test_owned_preflight_accepts_real_public_methods_without_browser(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "pwright"
            package.mkdir()
            (package / "__init__.py").write_text("")
            for name in ("async_api", "sync_api"):
                (package / f"{name}.py").write_text(
                    "class BrowserType:\n    def launch(self):\n        pass\n"
                )
            environment = os.environ.copy()
            environment["PYTHONPATH"] = str(root)

            # Act
            result = _require_owned_public_launch(
                sys.executable, "candidate", cwd=root, env=environment
            )

            # Assert
            self.assertIsNone(result)

    def test_junit_label_is_exact_and_conflicting_label_is_rejected(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            report = Path(directory) / "result.xml"
            report.write_text(
                '<testsuites><testsuite tests="1"><testcase name="one"/></testsuite></testsuites>'
            )

            # Act
            _label_junit_launch_mode(report, "owned")
            properties = (
                ET.parse(report).getroot().findall("testsuite/properties/property")
            )

            # Assert
            self.assertEqual(
                [(item.get("name"), item.get("value")) for item in properties],
                [(JUNIT_LAUNCH_MODE_PROPERTY, "owned")],
            )
            with self.assertRaisesRegex(ValueError, "conflicts"):
                _label_junit_launch_mode(report, "cdp-adapter")

    def test_owned_resume_requires_matching_receipt_and_case_report(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            case_dir = Path(directory) / "result.xml.cases"
            case_dir.mkdir()
            old_case = case_dir / "00001-old.xml"
            old_case.write_text(
                '<testsuites><testsuite><testcase name="one"/></testsuite></testsuites>'
            )

            # Act and Assert
            with self.assertRaisesRegex(SystemExit, "cannot resume unlabeled"):
                _prepare_isolated_launch_receipt(case_dir, "owned", True)
            _prepare_isolated_launch_receipt(case_dir, "cdp-adapter", True)
            with self.assertRaisesRegex(SystemExit, "differs from owned"):
                _prepare_isolated_launch_receipt(case_dir, "owned", True)
            with self.assertRaisesRegex(ValueError, "cannot resume an unlabeled"):
                _require_resumable_launch_mode(old_case, "owned")
            _label_junit_launch_mode(old_case, "cdp-adapter")
            with self.assertRaisesRegex(ValueError, "launch mode differs"):
                _require_resumable_launch_mode(old_case, "owned")

    def test_candidate_shim_patches_only_adapter_mode(self) -> None:
        for api_name in ("async_api", "sync_api"):
            for launch_mode, has_launch in (
                ("cdp-adapter", True),
                ("owned", True),
                ("owned", False),
            ):
                with self.subTest(
                    api=api_name, launch_mode=launch_mode, has_launch=has_launch
                ):
                    # Arrange
                    def original_launch(_self: object) -> None:
                        return None

                    candidate = ModuleType(f"pwright.{api_name}")
                    browser_type = type(
                        "BrowserType",
                        (),
                        {"launch": original_launch} if has_launch else {},
                    )
                    vars(candidate)["BrowserType"] = browser_type
                    vars(candidate)["Browser"] = type("Browser", (), {"contexts": []})

                    def missing_type(name: str) -> object:
                        if name.startswith("__"):
                            raise AttributeError(name)
                        return type(name, (), {})

                    candidate.__getattr__ = missing_type
                    pwright = ModuleType("pwright")
                    pwright.__path__ = []
                    vars(pwright)["Error"] = RuntimeError
                    vars(pwright)["TimeoutError"] = TimeoutError
                    vars(pwright)["UnsupportedError"] = RuntimeError
                    playwright = ModuleType("playwright")
                    playwright.__path__ = [str(SHIM)]
                    modules = {
                        "pwright": pwright,
                        f"pwright.{api_name}": candidate,
                        "playwright": playwright,
                    }
                    with (
                        patch.dict(sys.modules, modules),
                        patch.dict(
                            os.environ, {"PWRIGHT_OFFICIAL_LAUNCH_MODE": launch_mode}
                        ),
                    ):
                        # Act
                        if not has_launch:
                            with self.assertRaisesRegex(
                                RuntimeError, "requires pwright"
                            ):
                                importlib.import_module(f"playwright.{api_name}")
                            self.assertFalse(hasattr(browser_type, "launch"))
                            continue
                        shim_api = importlib.import_module(f"playwright.{api_name}")

                        # Assert
                        if launch_mode == "owned":
                            self.assertIs(vars(browser_type)["launch"], original_launch)
                        else:
                            self.assertIs(
                                vars(browser_type)["launch"], shim_api._candidate_launch
                            )
                            with (
                                patch.dict(
                                    os.environ,
                                    {"PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned"},
                                ),
                                self.assertRaisesRegex(
                                    RuntimeError, "disabled in owned"
                                ),
                            ):
                                if api_name == "async_api":
                                    asyncio.run(
                                        shim_api._candidate_launch(browser_type())
                                    )
                                else:
                                    shim_api._candidate_launch(browser_type())

    def test_oracle_runtime_patches_only_adapter_mode(self) -> None:
        for launch_mode in ("cdp-adapter", "owned"):
            with self.subTest(launch_mode=launch_mode):
                # Arrange
                def async_original(_self: object) -> None:
                    return None

                def sync_original(_self: object) -> None:
                    return None

                async_type = type(
                    "BrowserType",
                    (),
                    {
                        "launch": async_original,
                        "connect_over_cdp": lambda self, endpoint: None,
                    },
                )
                sync_type = type(
                    "BrowserType",
                    (),
                    {
                        "launch": sync_original,
                        "connect_over_cdp": lambda self, endpoint: None,
                    },
                )
                async_api = ModuleType("playwright.async_api")
                sync_api = ModuleType("playwright.sync_api")
                for api, browser_type in (
                    (async_api, async_type),
                    (sync_api, sync_type),
                ):
                    vars(api)["BrowserType"] = browser_type
                    vars(api)["Browser"] = type("Browser", (), {})
                    vars(api)["BrowserContext"] = type("BrowserContext", (), {})
                playwright = ModuleType("playwright")
                playwright.__path__ = []
                modules = {
                    "playwright": playwright,
                    "playwright.async_api": async_api,
                    "playwright.sync_api": sync_api,
                }
                spec = importlib.util.spec_from_file_location(
                    "owned_oracle_test_runtime", ORACLE_RUNTIME
                )
                if spec is None or spec.loader is None:
                    self.fail("oracle runtime module is unavailable")
                    return
                runtime = importlib.util.module_from_spec(spec)
                with (
                    patch.dict(sys.modules, modules),
                    patch.dict(
                        os.environ,
                        {
                            "PWRIGHT_OFFICIAL_LAUNCH_MODE": launch_mode,
                            "PWRIGHT_CDP_ENDPOINT": "http://127.0.0.1:9222",
                        },
                    ),
                ):
                    # Act
                    spec.loader.exec_module(runtime)
                    runtime.install_oracle_launch_adapter()

                    # Assert
                    if launch_mode == "owned":
                        self.assertIs(vars(async_type)["launch"], async_original)
                        self.assertIs(vars(sync_type)["launch"], sync_original)
                    else:
                        self.assertIsNot(vars(async_type)["launch"], async_original)
                        self.assertIsNot(vars(sync_type)["launch"], sync_original)
                        with patch.dict(
                            os.environ, {"PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned"}
                        ):
                            with self.assertRaisesRegex(
                                RuntimeError, "already installed"
                            ):
                                runtime.install_oracle_launch_adapter()
                            with self.assertRaisesRegex(
                                RuntimeError, "disabled in owned"
                            ):
                                asyncio.run(vars(async_type)["launch"](async_type()))
                            with self.assertRaisesRegex(
                                RuntimeError, "disabled in owned"
                            ):
                                vars(sync_type)["launch"](sync_type())

    def test_oracle_adapter_keeps_browser_api_name_and_hides_fixture_contexts(
        self,
    ) -> None:
        # Arrange
        class Context:
            def __init__(self) -> None:
                self._impl_obj = object()

        def browser_class() -> type:
            class Browser:
                def __init__(self) -> None:
                    self._contexts = [Context()]

                @property
                def contexts(self) -> list[Context]:
                    return self._contexts

                def new_context_error(self) -> str:
                    return f"{self.__class__.__name__}.new_context"

            return Browser

        async_browser_class = browser_class()
        sync_browser_class = browser_class()

        async def async_connect(_self: object, _endpoint: str) -> object:
            return async_browser_class()

        def sync_connect(_self: object, _endpoint: str) -> object:
            return sync_browser_class()

        async_type = type(
            "BrowserType",
            (),
            {"name": "chromium", "connect_over_cdp": async_connect},
        )
        sync_type = type(
            "BrowserType",
            (),
            {"name": "chromium", "connect_over_cdp": sync_connect},
        )
        async_api = ModuleType("playwright.async_api")
        sync_api = ModuleType("playwright.sync_api")
        for api, browser_type, browser in (
            (async_api, async_type, async_browser_class),
            (sync_api, sync_type, sync_browser_class),
        ):
            vars(api)["BrowserType"] = browser_type
            vars(api)["Browser"] = browser
            vars(api)["BrowserContext"] = Context
        playwright = ModuleType("playwright")
        playwright.__path__ = []
        modules = {
            "playwright": playwright,
            "playwright.async_api": async_api,
            "playwright.sync_api": sync_api,
        }
        spec = importlib.util.spec_from_file_location(
            "oracle_api_name_test_runtime", ORACLE_RUNTIME
        )
        if spec is None or spec.loader is None:
            self.fail("oracle runtime module is unavailable")
            return
        runtime = importlib.util.module_from_spec(spec)
        with (
            patch.dict(sys.modules, modules),
            patch.dict(
                os.environ,
                {
                    "PWRIGHT_OFFICIAL_LAUNCH_MODE": "cdp-adapter",
                    "PWRIGHT_CDP_ENDPOINT": "http://127.0.0.1:9222",
                },
            ),
        ):
            spec.loader.exec_module(runtime)
            runtime.install_oracle_launch_adapter()

            # Act
            async_launched = asyncio.run(async_type().launch())
            sync_launched = sync_type().launch()

            # Assert
            for launched in (async_launched, sync_launched):
                self.assertEqual(type(launched).__name__, "Browser")
                self.assertEqual(launched.new_context_error(), "Browser.new_context")
                self.assertEqual(launched.contexts, [])
                visible = Context()
                launched._contexts.append(visible)
                self.assertEqual(launched.contexts, [visible])


if __name__ == "__main__":
    unittest.main()
